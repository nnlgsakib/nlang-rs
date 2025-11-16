use crate::ast::*;
use crate::semantic::error::SemanticError;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TypeVariable {
    pub id: usize,
    pub name: Option<String>,
    pub constraints: Vec<Type>,
    pub inferred_type: Option<Type>,
}

impl TypeVariable {
    pub fn new(id: usize, name: Option<String>) -> Self {
        Self {
            id,
            name,
            constraints: Vec::new(),
            inferred_type: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeConstraint {
    pub type_var: Type,
    pub constraint_type: Type,
    pub source: String,
}

#[derive(Debug)]
pub struct TypeInferenceEngine {
    type_variables: Vec<TypeVariable>,
    constraints: Vec<TypeConstraint>,
    type_map: HashMap<String, Type>,
    current_type_var_id: usize,
    function_context: Option<FunctionContext>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct FunctionContext {
    parameters: Vec<Parameter>,
    return_type: Option<Type>,
    generic_params: Vec<String>,
}

impl TypeInferenceEngine {
    pub fn new() -> Self {
        Self {
            type_variables: Vec::new(),
            constraints: Vec::new(),
            type_map: HashMap::new(),
            current_type_var_id: 0,
            function_context: None,
        }
    }

    /// Create a new type variable for inference
    pub fn new_type_variable(&mut self, name: Option<String>) -> Type {
        let id = self.current_type_var_id;
        self.current_type_var_id += 1;

        let type_var = TypeVariable::new(id, name.clone());
        self.type_variables.push(type_var.clone());

        // Return a generic type that represents the type variable
        Type::Generic(name.unwrap_or(format!("T{}", id)))
    }

    /// Add a type constraint to the inference system
    pub fn add_constraint(&mut self, type_var: Type, constraint_type: Type, source: String) {
        self.constraints.push(TypeConstraint {
            type_var,
            constraint_type,
            source,
        });
    }

    /// Infer the type of a parameter based on its usage in the function body
    pub fn infer_parameter_type(&mut self, param: &str, usage_expressions: &[Expr]) -> Option<Type> {
        let mut inferred_types = Vec::new();

        for expr in usage_expressions {
            if let Some(type_hint) = self.extract_type_hint_from_usage(param, expr) {
                inferred_types.push(type_hint);
            }
        }

        // Find the most specific type that satisfies all constraints
        self.unify_types(&inferred_types)
    }

    /// Extract type hints from how a parameter is used
    fn extract_type_hint_from_usage(&self, param: &str, expr: &Expr) -> Option<Type> {
        match expr {
            Expr::Binary { left, operator, right } => {
                if let Expr::Variable(name) = left.as_ref() {
                    if name == param {
                        return self.get_operator_result_type(operator, right);
                    }
                }
                if let Expr::Variable(name) = right.as_ref() {
                    if name == param {
                        return self.get_operator_result_type(operator, left);
                    }
                }
                None
            },
            Expr::Call { callee, arguments } => {
                // Check if parameter is used as argument to a function
                for (i, arg) in arguments.iter().enumerate() {
                    if let Expr::Variable(name) = arg {
                        if name == param {
                            // Try to infer from function signature
                            return self.infer_from_function_call(callee, i);
                        }
                    }
                }
                None
            },
            Expr::Unary { operator, operand } => {
                if let Expr::Variable(name) = operand.as_ref() {
                    if name == param {
                        return self.get_unary_operator_result_type(operator);
                    }
                }
                None
            },
            _ => None,
        }
    }

    /// Get the result type of a binary operation
    fn get_operator_result_type(&self, operator: &BinaryOperator, other: &Expr) -> Option<Type> {
        let other_type = self.infer_expression_type(other);

        match operator {
            BinaryOperator::Plus | BinaryOperator::Minus | BinaryOperator::Star |
            BinaryOperator::Slash | BinaryOperator::Percent => {
                // Arithmetic operations - promote to most general type
                Some(self.promote_numeric_type(&other_type))
            },
            BinaryOperator::EqualEqual | BinaryOperator::NotEqual |
            BinaryOperator::Less | BinaryOperator::LessEqual |
            BinaryOperator::Greater | BinaryOperator::GreaterEqual => {
                // Comparison operations always return boolean
                Some(Type::Boolean)
            },
            BinaryOperator::And | BinaryOperator::Or => {
                // Logical operations require boolean operands and return boolean
                Some(Type::Boolean)
            },
            BinaryOperator::BitAnd | BinaryOperator::BitOr | BinaryOperator::BitXor | BinaryOperator::ShiftLeft | BinaryOperator::ShiftRight => {
                Some(Type::Integer)
            },
        }
    }

    /// Get the result type of a unary operation
    fn get_unary_operator_result_type(&self, operator: &UnaryOperator) -> Option<Type> {
        match operator {
            // Negation can apply to numeric types (integers and floats)
            UnaryOperator::Negate => {
                // Return a generic numeric type that will be resolved during constraint solving
                Some(Type::Unknown) // Will be unified with the operand type
            },
            UnaryOperator::Not => Some(Type::Boolean),
            UnaryOperator::BitNot => Some(Type::Integer),
        }
    }

    /// Infer type from function call context
    fn infer_from_function_call(&self, callee: &Expr, _arg_index: usize) -> Option<Type> {
        match callee {
            Expr::Variable(name) => {
                // Check built-in functions
                match name.as_str() {
                    "abs" | "max" | "min" | "pow" => Some(Type::Integer),
                    "abs_float" => Some(Type::F64),
                    "len" => Some(Type::Integer),
                    "sha256" => Some(Type::String),
                    "sha256_random" => Some(Type::String),
                    "upper" | "lower" | "trim" | "contains" => Some(Type::String),
                    "int" => Some(Type::Integer),
                    "float" => Some(Type::F64),
                    "str" => Some(Type::String),
                    _ => None,
                }
            },
            _ => None,
        }
    }

    /// Promote numeric type to most general form
    fn promote_numeric_type(&self, type_hint: &Type) -> Type {
        match type_hint {
            Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::ISize | Type::Integer => Type::Integer,
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::USize => Type::USize,
            Type::F32 => Type::F32,
            Type::F64 | Type::Float => Type::F64,
            _ => Type::Integer, // Default fallback
        }
    }

    /// Infer the type of an expression
    pub fn infer_expression_type(&self, expr: &Expr) -> Type {
        match expr {
            Expr::Literal(literal) => self.infer_literal_type(literal),
            Expr::Variable(name) => self.type_map.get(name).cloned().unwrap_or(Type::Unknown),
            Expr::Binary { left, operator, right } => {
                let left_type = self.infer_expression_type(left);
                let right_type = self.infer_expression_type(right);
                self.infer_binary_operation_type(operator, &left_type, &right_type)
            },
            Expr::Unary { operator, operand } => {
                let operand_type = self.infer_expression_type(operand);
                self.infer_unary_operation_type(operator, &operand_type)
            },
            Expr::Call { callee, arguments } => {
                self.infer_call_type(callee, arguments)
            },
            Expr::ArrayLiteral { elements } => {
                self.infer_array_type(elements)
            },
            Expr::Tuple { elements } => {
                let element_types: Vec<Type> = elements.iter()
                    .map(|e| self.infer_expression_type(e))
                    .collect();
                Type::Tuple(element_types)
            },
            Expr::IfExpression { condition, then_branch, else_branch } => {
                // Condition must be boolean
                let _condition_type = self.infer_expression_type(condition);
                let then_type = self.infer_expression_type(then_branch);
                let else_type = self.infer_expression_type(else_branch);
                self.unify_types(&[then_type, else_type]).unwrap_or(Type::Unknown)
            },
            Expr::Match { expression, cases } => {
                let _expr_type = self.infer_expression_type(expression);
                let case_types: Vec<Type> = cases.iter()
                    .map(|case| self.infer_expression_type(&case.body))
                    .collect();
                self.unify_types(&case_types).unwrap_or(Type::Unknown)
            },
            _ => Type::Unknown,
        }
    }

    /// Infer literal type
    fn infer_literal_type(&self, literal: &Literal) -> Type {
        match literal {
            Literal::Integer(_) => Type::Integer,
            Literal::I8(_) => Type::I8,
            Literal::I16(_) => Type::I16,
            Literal::I32(_) => Type::I32,
            Literal::I64(_) => Type::I64,
            Literal::ISize(_) => Type::ISize,
            Literal::U8(_) => Type::U8,
            Literal::U16(_) => Type::U16,
            Literal::U32(_) => Type::U32,
            Literal::U64(_) => Type::U64,
            Literal::USize(_) => Type::USize,
            Literal::Float(_) => Type::F64,
            Literal::Boolean(_) => Type::Boolean,
            Literal::String(_) => Type::String,
            Literal::Null => Type::Void,
        }
    }

    /// Infer binary operation type
    fn infer_binary_operation_type(&self, operator: &BinaryOperator, left: &Type, right: &Type) -> Type {
        match operator {
            BinaryOperator::Plus | BinaryOperator::Minus | BinaryOperator::Star |
            BinaryOperator::Slash | BinaryOperator::Percent => {
                self.infer_arithmetic_operation_type(left, right)
            },
            BinaryOperator::EqualEqual | BinaryOperator::NotEqual |
            BinaryOperator::Less | BinaryOperator::LessEqual |
            BinaryOperator::Greater | BinaryOperator::GreaterEqual => {
                Type::Boolean
            },
            BinaryOperator::And | BinaryOperator::Or => {
                Type::Boolean
            },
            BinaryOperator::BitAnd | BinaryOperator::BitOr | BinaryOperator::BitXor | BinaryOperator::ShiftLeft | BinaryOperator::ShiftRight => {
                Type::Integer
            },
        }
    }

    /// Infer arithmetic operation type with promotion
    fn infer_arithmetic_operation_type(&self, left: &Type, right: &Type) -> Type {
        // If both types are the same, use that type
        if left == right {
            return left.clone();
        }

        // Handle integer promotion
        let left_numeric = self.is_numeric_type(left);
        let right_numeric = self.is_numeric_type(right);

        if left_numeric && right_numeric {
            // Promote to float if either operand is float
            if self.is_float_type(left) || self.is_float_type(right) {
                return Type::F64;
            }

            // Promote to the most general integer type
            return Type::Integer;
        }

        // Handle string concatenation
        if left == &Type::String && right == &Type::String {
            return Type::String;
        }

        Type::Unknown
    }

    /// Infer unary operation type
    fn infer_unary_operation_type(&self, operator: &UnaryOperator, operand: &Type) -> Type {
        match operator {
            UnaryOperator::Negate => operand.clone(),
            UnaryOperator::Not => Type::Boolean,
            UnaryOperator::BitNot => Type::Integer,
        }
    }

    /// Infer function call type
    fn infer_call_type(&self, callee: &Expr, _arguments: &[Expr]) -> Type {
        match callee {
            Expr::Variable(name) => {
                // Check built-in functions
                match name.as_str() {
                    "print" | "println" => Type::Void,
                    "input" => Type::String,
                    "int" => Type::Integer,
                    "float" => Type::F64,
                    "str" => Type::String,
                    "abs" | "max" | "min" | "len" => Type::Integer,
                    "abs_float" | "pow" => Type::F64,
                    "upper" | "lower" | "trim" | "contains" => Type::String,
                    _ => Type::Unknown,
                }
            },
            _ => Type::Unknown,
        }
    }

    /// Infer array type from elements
    fn infer_array_type(&self, elements: &[Expr]) -> Type {
        if elements.is_empty() {
            return Type::Array(Box::new(Type::Unknown), 0);
        }

        let element_types: Vec<Type> = elements.iter()
            .map(|e| self.infer_expression_type(e))
            .collect();

        // Try to find a common type for all elements
        if let Some(common_type) = self.unify_types(&element_types) {
            Type::Array(Box::new(common_type), elements.len())
        } else {
            // If no common type, create a union type
            Type::Union(element_types)
        }
    }

    /// Unify multiple types to find common type
    pub fn unify_types(&self, types: &[Type]) -> Option<Type> {
        if types.is_empty() {
            return None;
        }

        let mut result_type = types[0].clone();

        for type_hint in types.iter().skip(1) {
            result_type = match self.unify_two_types(&result_type, type_hint) {
                Some(unified) => unified,
                None => return None,
            };
        }

        Some(result_type)
    }

    /// Unify two types to find common type
    fn unify_two_types(&self, type1: &Type, type2: &Type) -> Option<Type> {
        if type1 == type2 {
            return Some(type1.clone());
        }

        // Handle numeric type promotion
        if self.is_numeric_type(type1) && self.is_numeric_type(type2) {
            return Some(self.promote_numeric_type(type2));
        }

        // Handle integer <-> float conversion
        if (self.is_integer_type(type1) && self.is_float_type(type2)) ||
           (self.is_float_type(type1) && self.is_integer_type(type2)) {
            return Some(Type::F64);
        }

        // Handle unknown types
        if *type1 == Type::Unknown {
            return Some(type2.clone());
        }
        if *type2 == Type::Unknown {
            return Some(type1.clone());
        }

        // Handle generic/inference types
        if matches!(type1, Type::Generic(_)) || matches!(type1, Type::Infer) {
            return Some(type2.clone());
        }
        if matches!(type2, Type::Generic(_)) || matches!(type2, Type::Infer) {
            return Some(type1.clone());
        }

        None
    }

    /// Check if a type is numeric
    fn is_numeric_type(&self, type_hint: &Type) -> bool {
        self.is_integer_type(type_hint) || self.is_float_type(type_hint)
    }

    /// Check if a type is integer
    fn is_integer_type(&self, type_hint: &Type) -> bool {
        matches!(type_hint,
            Type::Integer | Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::ISize |
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::USize
        )
    }

    /// Check if a type is float
    fn is_float_type(&self, type_hint: &Type) -> bool {
        matches!(type_hint, Type::F32 | Type::F64 | Type::Float)
    }

    /// Set a type for a variable in the current scope
    pub fn set_variable_type(&mut self, name: String, type_hint: Type) {
        self.type_map.insert(name, type_hint);
    }

    /// Get the type of a variable
    pub fn get_variable_type(&self, name: &str) -> Option<Type> {
        self.type_map.get(name).cloned()
    }

    /// Solve all type constraints and resolve type variables
    pub fn solve_constraints(&mut self) -> Result<(), SemanticError> {
        let mut changed = true;

        // Iterate until no more inferences can be made
        while changed {
            changed = false;

            for constraint in self.constraints.clone() {
                if let Some(resolved_type) = self.resolve_constraint(&constraint) {
                    if self.apply_type_solution(&constraint.type_var, &resolved_type) {
                        changed = true;
                    }
                }
            }
        }

        // Validate that all type variables are resolved
        for type_var in &self.type_variables {
            if type_var.inferred_type.is_none() && !type_var.constraints.is_empty() {
                // Try to resolve from constraints
                if let Some(_unified) = self.unify_types(&type_var.constraints) {
                    // This would be resolved in a full implementation
                    continue;
                }
            }
        }

        Ok(())
    }

    /// Resolve a single type constraint
    fn resolve_constraint(&self, constraint: &TypeConstraint) -> Option<Type> {
        // In a full implementation, this would use constraint solving algorithms
        // For now, return the constraint type if it's concrete
        if !matches!(constraint.constraint_type, Type::Unknown | Type::Infer) {
            Some(constraint.constraint_type.clone())
        } else {
            None
        }
    }

    /// Apply a type solution to a type variable
    fn apply_type_solution(&mut self, type_var: &Type, solution: &Type) -> bool {
        // Update type variables with resolved types
        for tv in &mut self.type_variables {
            if let Type::Generic(name) = &type_var {
                if let Some(tv_name) = &tv.name {
                    if name == tv_name && tv.inferred_type.is_none() {
                        tv.inferred_type = Some(solution.clone());
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Enter function context for parameter type inference
    pub fn enter_function_context(&mut self, parameters: Vec<Parameter>, return_type: Option<Type>) {
        self.function_context = Some(FunctionContext {
            parameters,
            return_type,
            generic_params: Vec::new(),
        });
    }

    /// Exit function context
    pub fn exit_function_context(&mut self) {
        self.function_context = None;
    }

    /// Infer function return type from return statements in body
    pub fn infer_function_return_type(&self, body: &[Statement]) -> Option<Type> {
        let mut return_types = Vec::new();

        for stmt in body {
            if let Statement::Return { value } = stmt {
                if let Some(expr) = value {
                    return_types.push(self.infer_expression_type(expr));
                } else {
                    return_types.push(Type::Void);
                }
            }
        }

        // If no explicit return, check if function returns implicitly at end
        if return_types.is_empty() {
            // Check if the last statement is an expression (implicit return)
            if let Some(last_stmt) = body.last() {
                if let Statement::Expression(expression) = last_stmt {
                    let inferred_type = self.infer_expression_type(expression);
                    return if inferred_type == Type::Unknown {
                        Some(Type::Void)  // Default to Void for unknown implicit returns
                    } else {
                        Some(inferred_type)
                    };
                }
            }
            // No explicit or implicit return - default to Void
            return Some(Type::Void);
        }

        self.unify_types(&return_types)
    }

    /// Infer function parameter types from usage in function body
    pub fn infer_function_parameter_types(&mut self, parameters: &mut [Parameter], body: &[Statement]) -> Result<(), SemanticError> {
        for param in parameters.iter_mut() {
            if param.param_type.is_none() {
                // Collect all expressions where this parameter is used
                let usage_expressions = self.collect_parameter_usages(&param.name, body);

                // Infer type from usage
                if let Some(inferred_type) = self.infer_parameter_type(&param.name, &usage_expressions) {
                    param.inferred_type = Some(inferred_type.clone());
                    self.set_variable_type(param.name.clone(), inferred_type);
                } else {
                    // If no type can be inferred, default to Unknown and let it be resolved later
                    param.inferred_type = Some(Type::Unknown);
                    self.set_variable_type(param.name.clone(), Type::Unknown);
                }
            } else {
                // Parameter has explicit type, set it in the type map
                self.set_variable_type(param.name.clone(), param.param_type.clone().unwrap());
            }
        }

        Ok(())
    }

    /// Collect all expressions where a parameter is used
    fn collect_parameter_usages(&self, param_name: &str, body: &[Statement]) -> Vec<Expr> {
        let mut usages = Vec::new();

        for stmt in body {
            self.collect_usages_in_statement(param_name, stmt, &mut usages);
        }

        usages
    }

    /// Recursively collect parameter usages in a statement
    fn collect_usages_in_statement(&self, param_name: &str, stmt: &Statement, usages: &mut Vec<Expr>) {
        match stmt {
            Statement::LetDeclaration { initializer, .. } => {
                if let Some(init) = initializer {
                    self.collect_usages_in_expression(param_name, init, usages);
                }
            },
            Statement::Expression(expression) => {
                self.collect_usages_in_expression(param_name, expression, usages);
            },
            Statement::Return { value } => {
                if let Some(expr) = value {
                    self.collect_usages_in_expression(param_name, expr, usages);
                }
            },
            Statement::If { condition, then_branch, else_branch } => {
                self.collect_usages_in_expression(param_name, condition, usages);
                self.collect_usages_in_statement(param_name, then_branch, usages);
                if let Some(else_stmt) = else_branch {
                    self.collect_usages_in_statement(param_name, else_stmt, usages);
                }
            },
            Statement::While { condition, body } => {
                self.collect_usages_in_expression(param_name, condition, usages);
                self.collect_usages_in_statement(param_name, body, usages);
            },
            Statement::For { initializer, condition, increment, body } => {
                if let Some(init) = initializer {
                    self.collect_usages_in_statement(param_name, init, usages);
                }
                if let Some(cond) = condition {
                    self.collect_usages_in_expression(param_name, cond, usages);
                }
                if let Some(inc) = increment {
                    self.collect_usages_in_expression(param_name, inc, usages);
                }
                self.collect_usages_in_statement(param_name, body, usages);
            },
            Statement::FunctionDeclaration { .. } => {
                // Don't collect usages from nested function declarations
                // as they have separate scopes
            },
            _ => {}
        }
    }

    /// Recursively collect parameter usages in an expression
    fn collect_usages_in_expression(&self, param_name: &str, expr: &Expr, usages: &mut Vec<Expr>) {
        match expr {
            Expr::Variable(name) if name == param_name => {
                usages.push(expr.clone());
            },
            Expr::Binary { left, right, .. } => {
                self.collect_usages_in_expression(param_name, left, usages);
                self.collect_usages_in_expression(param_name, right, usages);
            },
            Expr::Unary { operand, .. } => {
                self.collect_usages_in_expression(param_name, operand, usages);
            },
            Expr::Call { callee, arguments } => {
                self.collect_usages_in_expression(param_name, callee, usages);
                for arg in arguments {
                    self.collect_usages_in_expression(param_name, arg, usages);
                }
            },
            Expr::ArrayLiteral { elements } => {
                for element in elements {
                    self.collect_usages_in_expression(param_name, element, usages);
                }
            },
            Expr::Tuple { elements } => {
                for element in elements {
                    self.collect_usages_in_expression(param_name, element, usages);
                }
            },
            Expr::IfExpression { condition, then_branch, else_branch } => {
                self.collect_usages_in_expression(param_name, condition, usages);
                self.collect_usages_in_expression(param_name, then_branch, usages);
                self.collect_usages_in_expression(param_name, else_branch, usages);
            },
            _ => {}
        }
    }
}

impl Default for TypeInferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_engine() -> TypeInferenceEngine {
        TypeInferenceEngine::new()
    }

    #[test]
    fn test_literal_type_inference() {
        let engine = create_test_engine();

        // Test integer literals
        assert_eq!(
            engine.infer_expression_type(&Expr::Literal(Literal::Integer(42))),
            Type::Integer
        );
        assert_eq!(
            engine.infer_expression_type(&Expr::Literal(Literal::I8(8))),
            Type::I8
        );
        assert_eq!(
            engine.infer_expression_type(&Expr::Literal(Literal::Float(3.14))),
            Type::F64
        );

        // Test other literals
        assert_eq!(
            engine.infer_expression_type(&Expr::Literal(Literal::Boolean(true))),
            Type::Boolean
        );
        assert_eq!(
            engine.infer_expression_type(&Expr::Literal(Literal::String("hello".to_string()))),
            Type::String
        );
        assert_eq!(
            engine.infer_expression_type(&Expr::Literal(Literal::Null)),
            Type::Void
        );
    }

    #[test]
    fn test_arithmetic_type_inference() {
        let engine = create_test_engine();

        // Integer arithmetic
        let int_plus = Expr::Binary {
            left: Box::new(Expr::Literal(Literal::Integer(5))),
            operator: BinaryOperator::Plus,
            right: Box::new(Expr::Literal(Literal::Integer(3))),
        };
        assert_eq!(engine.infer_expression_type(&int_plus), Type::Integer);

        // Float arithmetic
        let float_plus = Expr::Binary {
            left: Box::new(Expr::Literal(Literal::Float(2.5))),
            operator: BinaryOperator::Plus,
            right: Box::new(Expr::Literal(Literal::Float(1.5))),
        };
        assert_eq!(engine.infer_expression_type(&float_plus), Type::F64);

        // Mixed arithmetic (should promote to float)
        let mixed = Expr::Binary {
            left: Box::new(Expr::Literal(Literal::Integer(5))),
            operator: BinaryOperator::Plus,
            right: Box::new(Expr::Literal(Literal::Float(3.2))),
        };
        assert_eq!(engine.infer_expression_type(&mixed), Type::F64);
    }

    #[test]
    fn test_type_unification() {
        let engine = create_test_engine();

        // Same types should unify
        assert_eq!(
            engine.unify_types(&[Type::Integer, Type::Integer]),
            Some(Type::Integer)
        );

        // Different numeric types should promote
        assert_eq!(
            engine.unify_types(&[Type::Integer, Type::F64]),
            Some(Type::F64)
        );

        // Incompatible types should not unify
        assert_eq!(
            engine.unify_types(&[Type::Integer, Type::String]),
            None
        );
    }

    #[test]
    fn test_function_call_type_inference() {
        let engine = create_test_engine();

        // Built-in function calls
        let len_call = Expr::Call {
            callee: Box::new(Expr::Variable("len".to_string())),
            arguments: vec![Expr::Literal(Literal::String("hello".to_string()))],
        };
        assert_eq!(engine.infer_expression_type(&len_call), Type::Integer);

        let print_call = Expr::Call {
            callee: Box::new(Expr::Variable("println".to_string())),
            arguments: vec![Expr::Literal(Literal::String("hello".to_string()))],
        };
        assert_eq!(engine.infer_expression_type(&print_call), Type::Void);
    }
}
