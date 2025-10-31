use crate::ast::*;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CCodeGenError {
    #[error("Unsupported feature: {message}")]
    UnsupportedFeature { message: String },
    #[error("Variable not found: {name}")]
    VariableNotFound { name: String },
}

pub struct CCodeGenerator {
    #[allow(dead_code)]
    module_name: String,
    variables: HashMap<String, String>, // variable name -> C type
    #[allow(dead_code)]
    temp_counter: usize,
    string_constants: HashMap<String, String>, // string literal -> constant name
    string_counter: usize,
}

impl CCodeGenerator {
    pub fn new(module_name: String) -> Self {
        Self {
            module_name,
            variables: HashMap::new(),
            temp_counter: 0,
            string_constants: HashMap::new(),
            string_counter: 0,
        }
    }

    pub fn generate_program(&mut self, program: &Program) -> Result<String, CCodeGenError> {
        let mut code = String::new();
        
        // Add includes
        code.push_str("#include <stdio.h>\n");
        code.push_str("#include <string.h>\n");
        code.push_str("#include <stdlib.h>\n");
        code.push_str("#include <math.h>\n\n");
        
        // Collect string literals first
        self.collect_string_literals(program);
        
        // Generate string constants
        for (literal, const_name) in &self.string_constants {
            let escaped = self.escape_c_string(literal);
            code.push_str(&format!("static const char {}[] = \"{}\";\n", const_name, escaped));
        }
        
        if !self.string_constants.is_empty() {
            code.push_str("\n");
        }
        
        // First pass: Generate function declarations
        for statement in &program.statements {
            if let Statement::FunctionDeclaration { name, parameters, return_type, .. } = statement {
                let decl = self.generate_function_declaration(name, parameters, return_type.as_ref())?;
                code.push_str(&decl);
                code.push_str("\n");
            }
        }
        
        if program.statements.iter().any(|s| matches!(s, Statement::FunctionDeclaration { .. })) {
            code.push_str("\n");
        }
        
        // Second pass: Generate function implementations
        for statement in &program.statements {
            if let Statement::FunctionDeclaration { .. } = statement {
                code.push_str(&self.generate_function_from_statement(statement)?);
                code.push_str("\n");
            }
        }
        
        Ok(code)
    }
    
    fn collect_string_literals(&mut self, program: &Program) {
        for statement in &program.statements {
            self.collect_strings_from_statement(statement);
        }
    }
    
    fn collect_strings_from_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Expression(expr) => {
                self.collect_strings_from_expression(expr);
            }
            Statement::LetDeclaration { initializer, .. } => {
                if let Some(init) = initializer {
                    self.collect_strings_from_expression(init);
                }
            }
            Statement::FunctionDeclaration { body, .. } => {
                for stmt in body {
                    self.collect_strings_from_statement(stmt);
                }
            }
            Statement::If { condition, then_branch, else_branch } => {
                self.collect_strings_from_expression(condition);
                self.collect_strings_from_statement(then_branch);
                if let Some(else_stmt) = else_branch {
                    self.collect_strings_from_statement(else_stmt);
                }
            }
            Statement::While { condition, body } => {
                self.collect_strings_from_expression(condition);
                self.collect_strings_from_statement(body);
            }
            Statement::Return { value } => {
                if let Some(e) = value {
                    self.collect_strings_from_expression(e);
                }
            }
            Statement::Block { statements } => {
                for stmt in statements {
                    self.collect_strings_from_statement(stmt);
                }
            }
            _ => {}
        }
    }
    
    fn collect_strings_from_expression(&mut self, expression: &Expr) {
        match expression {
            Expr::Literal(Literal::String(s)) => {
                if !self.string_constants.contains_key(s) {
                    let const_name = format!("str_const_{}", self.string_counter);
                    self.string_counter += 1;
                    self.string_constants.insert(s.clone(), const_name);
                }
            }
            Expr::Binary { left, right, .. } => {
                self.collect_strings_from_expression(left);
                self.collect_strings_from_expression(right);
            }
            Expr::Unary { operand, .. } => {
                self.collect_strings_from_expression(operand);
            }
            Expr::Call { arguments, .. } => {
                for arg in arguments {
                    self.collect_strings_from_expression(arg);
                }
            }
            _ => {}
        }
    }
    
    fn generate_function_from_statement(&mut self, statement: &Statement) -> Result<String, CCodeGenError> {
        if let Statement::FunctionDeclaration { name, parameters, body, return_type, .. } = statement {
            let mut code = String::new();
            
            // Function signature
            let ret_type = return_type.as_ref().unwrap_or(&Type::Void);
            // Special case: main function should always return int in C
            let return_type_str = if name == "main" {
                "int".to_string()
            } else {
                self.type_to_c(ret_type)
            };
            code.push_str(&format!("{} {}(", return_type_str, name));
            
            // Parameters
            for (i, param) in parameters.iter().enumerate() {
                if i > 0 {
                    code.push_str(", ");
                }
                let param_type = self.type_to_c(&param.param_type);
                code.push_str(&format!("{} {}", param_type, param.name));
                self.variables.insert(param.name.clone(), param_type);
            }
            
            if parameters.is_empty() {
                code.push_str("void");
            }
            
            code.push_str(") {\n");
            
            // Function body
            for stmt in body {
                code.push_str(&self.generate_statement(stmt)?);
            }
            
            // Ensure main function returns 0 if no explicit return
            if name == "main" && !body.iter().any(|s| matches!(s, Statement::Return { .. })) {
                code.push_str("    return 0;\n");
            }
            
            code.push_str("}\n");
            
            Ok(code)
        } else {
            Err(CCodeGenError::UnsupportedFeature {
                message: "Expected function declaration".to_string(),
            })
        }
    }
    
    fn generate_statement(&mut self, statement: &Statement) -> Result<String, CCodeGenError> {
        match statement {
            Statement::Expression(expr) => {
                let expr_code = self.generate_expression(expr)?;
                Ok(format!("    {};\n", expr_code))
            }
            Statement::LetDeclaration { name, initializer, .. } => {
                // For simplicity, assume int type for now
                let c_type = "int".to_string();
                self.variables.insert(name.clone(), c_type.clone());
                
                if let Some(init) = initializer {
                    let init_code = self.generate_expression(init)?;
                    Ok(format!("    {} {} = {};\n", c_type, name, init_code))
                } else {
                    Ok(format!("    {} {};\n", c_type, name))
                }
            }
            Statement::If { condition, then_branch, else_branch } => {
                let mut code = String::new();
                let cond_code = self.generate_expression(condition)?;
                code.push_str(&format!("    if ({}) {{\n", cond_code));
                
                let then_code = self.generate_statement(then_branch)?;
                code.push_str(&format!("    {}", then_code));
                
                if let Some(else_stmt) = else_branch {
                    code.push_str("    } else {\n");
                    let else_code = self.generate_statement(else_stmt)?;
                    code.push_str(&format!("    {}", else_code));
                }
                
                code.push_str("    }\n");
                Ok(code)
            }
            Statement::While { condition, body } => {
                let mut code = String::new();
                let cond_code = self.generate_expression(condition)?;
                code.push_str(&format!("    while ({}) {{\n", cond_code));
                
                let body_code = self.generate_statement(body)?;
                code.push_str(&format!("    {}", body_code));
                
                code.push_str("    }\n");
                Ok(code)
            }
            Statement::Return { value } => {
                if let Some(e) = value {
                    let expr_code = self.generate_expression(e)?;
                    Ok(format!("    return {};\n", expr_code))
                } else {
                    Ok("    return;\n".to_string())
                }
            }
            Statement::Block { statements } => {
                let mut code = String::new();
                for stmt in statements {
                    code.push_str(&self.generate_statement(stmt)?);
                }
                Ok(code)
            }
            Statement::Break => {
                Ok("    break;\n".to_string())
            }
            Statement::Continue => {
                Ok("    continue;\n".to_string())
            }
            _ => {
                Err(CCodeGenError::UnsupportedFeature {
                    message: format!("Statement type not supported: {:?}", statement),
                })
            }
        }
    }
    
    fn generate_expression(&mut self, expression: &Expr) -> Result<String, CCodeGenError> {
        match expression {
            Expr::Literal(literal) => self.generate_literal(literal),
            Expr::Variable(name) => {
                if self.variables.contains_key(name) {
                    Ok(name.clone())
                } else {
                    Err(CCodeGenError::VariableNotFound { name: name.clone() })
                }
            }
            Expr::Binary { left, right, operator, .. } => {
                let left_code = self.generate_expression(left)?;
                let right_code = self.generate_expression(right)?;
                let op_str = self.binary_op_to_c(operator);
                Ok(format!("({} {} {})", left_code, op_str, right_code))
            }
            Expr::Unary { operand, operator, .. } => {
                let operand_code = self.generate_expression(operand)?;
                let op_str = self.unary_op_to_c(operator);
                Ok(format!("({}{})", op_str, operand_code))
            }
            Expr::Call { callee, arguments, .. } => {
                let func_name = if let Expr::Variable(name) = callee.as_ref() {
                    name.clone()
                } else {
                    return Err(CCodeGenError::UnsupportedFeature {
                        message: "Complex function calls not supported".to_string(),
                    });
                };
                
                // Handle print functions specially
                if func_name == "print" || func_name == "println" {
                    if arguments.len() == 1 {
                        let arg_code = self.generate_expression(&arguments[0])?;
                        let format_and_code = self.generate_print_format_and_arg(&arguments[0], &arg_code)?;
                        if func_name == "println" {
                            return Ok(format!("printf(\"{}\\n\", {})", format_and_code.0, format_and_code.1));
                        } else {
                            return Ok(format!("printf(\"{}\", {})", format_and_code.0, format_and_code.1));
                        }
                    }
                }
                
                let mut args_code = Vec::new();
                for arg in arguments {
                    args_code.push(self.generate_expression(arg)?);
                }
                Ok(format!("{}({})", func_name, args_code.join(", ")))
            }
            Expr::Function { .. } => {
                Err(CCodeGenError::UnsupportedFeature {
                    message: "Function expressions not supported".to_string(),
                })
            }
            Expr::Get { object, name } => {
                // Handle module-qualified access like math.PI
                if let Expr::Variable(module_name) = object.as_ref() {
                    let qualified_name = format!("{}.{}", module_name, name);
                    if self.variables.contains_key(&qualified_name) {
                        Ok(qualified_name)
                    } else {
                        Err(CCodeGenError::UnsupportedFeature {
                            message: format!("Undefined variable: {}", qualified_name),
                        })
                    }
                } else {
                    Err(CCodeGenError::UnsupportedFeature {
                        message: "Complex object access not supported".to_string(),
                    })
                }
            }
            Expr::Set { .. } => {
                Err(CCodeGenError::UnsupportedFeature {
                    message: "Property assignment not supported".to_string(),
                })
            }
            Expr::Index { .. } => {
                Err(CCodeGenError::UnsupportedFeature {
                    message: "Array indexing not supported".to_string(),
                })
            }
            Expr::Assign { name, value } => {
                let value_code = self.generate_expression(value)?;
                // Register the variable as int type (simplified)
                self.variables.insert(name.clone(), "int".to_string());
                // Return assignment expression
                Ok(format!("({} = {})", name, value_code))
            }
        }
    }
    
    fn generate_literal(&self, literal: &Literal) -> Result<String, CCodeGenError> {
        match literal {
            Literal::Integer(i) => Ok(i.to_string()),
            Literal::Float(f) => Ok(f.to_string()),
            Literal::String(s) => {
                if let Some(const_name) = self.string_constants.get(s) {
                    Ok(const_name.clone())
                } else {
                    Err(CCodeGenError::UnsupportedFeature {
                        message: format!("String constant not found: {}", s),
                    })
                }
            }
            Literal::Boolean(b) => Ok(if *b { "1" } else { "0" }.to_string()),
            Literal::Null => Ok("NULL".to_string()),
        }
    }
    
    fn type_to_c(&self, nlang_type: &Type) -> String {
        match nlang_type {
            Type::Integer => "int".to_string(),
            Type::Float => "double".to_string(),
            Type::String => "const char*".to_string(),
            Type::Boolean => "int".to_string(),
            Type::Void => "void".to_string(),
            Type::Array(_) => "void*".to_string(), // Simplified array handling
            Type::Function { .. } => "void*".to_string(), // Simplified function pointer handling
        }
    }
    
    fn binary_op_to_c(&self, op: &BinaryOperator) -> &'static str {
        match op {
            BinaryOperator::Plus => "+",
            BinaryOperator::Minus => "-",
            BinaryOperator::Star => "*",
            BinaryOperator::Slash => "/",
            BinaryOperator::Percent => "%",
            BinaryOperator::EqualEqual => "==",
            BinaryOperator::NotEqual => "!=",
            BinaryOperator::Less => "<",
            BinaryOperator::LessEqual => "<=",
            BinaryOperator::Greater => ">",
            BinaryOperator::GreaterEqual => ">=",
            BinaryOperator::And => "&&",
            BinaryOperator::Or => "||",
        }
    }
    
    fn unary_op_to_c(&self, op: &UnaryOperator) -> &'static str {
        match op {
            UnaryOperator::Negate => "-",
            UnaryOperator::Not => "!",
        }
    }
    
    fn generate_function_declaration(&self, name: &str, parameters: &[Parameter], return_type: Option<&Type>) -> Result<String, CCodeGenError> {
        let ret_type_str = match return_type {
            Some(ret_type) => {
                // Special case: main function should always return int in C
                if name == "main" {
                    "int".to_string()
                } else {
                    self.type_to_c(ret_type)
                }
            },
            None => {
                // Special case: main function should always return int in C
                if name == "main" {
                    "int".to_string()
                } else {
                    "void".to_string()
                }
            },
        };
        
        let mut params_str = String::new();
        for (i, param) in parameters.iter().enumerate() {
            if i > 0 {
                params_str.push_str(", ");
            }
            let param_type_str = self.type_to_c(&param.param_type);
            params_str.push_str(&format!("{} {}", param_type_str, param.name));
        }
        
        if params_str.is_empty() {
            params_str = "void".to_string();
        }
        
        Ok(format!("{} {}({});", ret_type_str, name, params_str))
    }
    
    fn generate_print_format_and_arg(&self, expr: &Expr, arg_code: &str) -> Result<(String, String), CCodeGenError> {
        match expr {
            Expr::Literal(Literal::Integer(_)) => Ok(("%d".to_string(), arg_code.to_string())),
            Expr::Literal(Literal::Float(_)) => Ok(("%f".to_string(), arg_code.to_string())),
            Expr::Literal(Literal::Boolean(b)) => {
                let bool_str = if *b { "true" } else { "false" };
                Ok(("%s".to_string(), format!("\"{}\"", bool_str)))
            }
            Expr::Literal(Literal::String(_)) => Ok(("%s".to_string(), arg_code.to_string())),
            Expr::Literal(Literal::Null) => Ok(("%s".to_string(), "\"null\"".to_string())),
            Expr::Variable(name) => {
                // Check the variable type from our tracking
                if let Some(var_type) = self.variables.get(name) {
                    match var_type.as_str() {
                        "int" => Ok(("%d".to_string(), arg_code.to_string())),
                        "float" | "double" => Ok(("%f".to_string(), arg_code.to_string())),
                        "char*" => Ok(("%s".to_string(), arg_code.to_string())),
                        _ => Ok(("%s".to_string(), arg_code.to_string())), // Default to string
                    }
                } else {
                    // Variable not found in tracking, default to string
                    Ok(("%s".to_string(), arg_code.to_string()))
                }
            }
            Expr::Binary { .. } => {
                // For expressions, assume they evaluate to integers for now
                // This is a simplification - in a full implementation, we'd need type inference
                Ok(("%d".to_string(), arg_code.to_string()))
            }
            _ => {
                // Default to string representation
                Ok(("%s".to_string(), arg_code.to_string()))
            }
        }
    }
    
    fn escape_c_string(&self, s: &str) -> String {
        s.chars()
            .map(|c| match c {
                '"' => "\\\"".to_string(),
                '\\' => "\\\\".to_string(),
                '\n' => "\\n".to_string(),
                '\r' => "\\r".to_string(),
                '\t' => "\\t".to_string(),
                c => c.to_string(),
            })
            .collect()
    }
}