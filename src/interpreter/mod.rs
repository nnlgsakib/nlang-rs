use crate::ast::{Program, Statement, Expr, Type, BinaryOperator, UnaryOperator, Literal, Parameter};
use crate::lexer::Lexer;
use crate::parser::Parser;
use std::collections::HashMap;
use std::fs;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InterpreterError {
    #[error("Variable '{name}' not found")]
    VariableNotFound { name: String },
    #[error("Function '{name}' not found")]
    FunctionNotFound { name: String },
    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },
    #[error("Division by zero")]
    DivisionByZero,
    #[error("Invalid operation: {message}")]
    InvalidOperation { message: String },
    #[error("Return statement executed")]
    ReturnValue(Value),
    #[error("Break statement executed")]
    Break,
    #[error("Continue statement executed")]
    Continue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    Float(f64),
    Boolean(bool),
    String(String),
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub body: Vec<Statement>,
    pub return_type: Option<Type>,
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Integer(_) => "int",
            Value::Float(_) => "float",
            Value::Boolean(_) => "bool",
            Value::String(_) => "string",
        }
    }
    
    pub fn to_int(&self) -> Result<i64, InterpreterError> {
        match self {
            Value::Integer(i) => Ok(*i),
            Value::Float(f) => Ok(*f as i64),
            Value::Boolean(b) => Ok(if *b { 1 } else { 0 }),
            _ => Err(InterpreterError::TypeMismatch {
                expected: "int".to_string(),
                actual: self.type_name().to_string(),
            }),
        }
    }
    
    pub fn to_float(&self) -> Result<f64, InterpreterError> {
        match self {
            Value::Integer(i) => Ok(*i as f64),
            Value::Float(f) => Ok(*f),
            _ => Err(InterpreterError::TypeMismatch {
                expected: "float".to_string(),
                actual: self.type_name().to_string(),
            }),
        }
    }
    
    pub fn to_bool(&self) -> Result<bool, InterpreterError> {
        match self {
            Value::Boolean(b) => Ok(*b),
            Value::Integer(i) => Ok(*i != 0),
            Value::Float(f) => Ok(*f != 0.0),
            _ => Err(InterpreterError::TypeMismatch {
                expected: "bool".to_string(),
                actual: self.type_name().to_string(),
            }),
        }
    }
}

#[derive(Clone)]
pub struct Environment {
    variables: HashMap<String, Value>,
    functions: HashMap<String, Function>,
}

impl Environment {
    pub fn new() -> Self {
        let mut env = Environment {
            variables: HashMap::new(),
            functions: HashMap::new(),
        };
        
        // Add built-in variables
        env.variables.insert("PI".to_string(), Value::Float(std::f64::consts::PI));
        
        env
    }
    
    pub fn define_variable(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }
    
    pub fn get_variable(&self, name: &str) -> Result<Value, InterpreterError> {
        self.variables.get(name)
            .cloned()
            .ok_or_else(|| InterpreterError::VariableNotFound { name: name.to_string() })
    }
    
    pub fn set_variable(&mut self, name: String, value: Value) -> Result<(), InterpreterError> {
        if self.variables.contains_key(&name) {
            self.variables.insert(name, value);
            Ok(())
        } else {
            Err(InterpreterError::VariableNotFound { name })
        }
    }
    
    pub fn define_function(&mut self, func: Function) {
        self.functions.insert(func.name.clone(), func);
    }
    
    pub fn get_function(&self, name: &str) -> Result<&Function, InterpreterError> {
        self.functions.get(name)
            .ok_or_else(|| InterpreterError::FunctionNotFound { name: name.to_string() })
    }
}

pub struct Interpreter {
    global_env: Environment,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            global_env: Environment::new(),
        }
    }
    
    pub fn execute_program(&mut self, program: &Program) -> Result<i32, InterpreterError> {
        self.execute_program_with_path(program, None)
    }
    
    pub fn execute_program_with_path(&mut self, program: &Program, file_path: Option<&str>) -> Result<i32, InterpreterError> {
        // First pass: handle imports
        for statement in &program.statements {
            match statement {
                Statement::Import { module, alias } => {
                    self.load_module(module, alias.as_deref(), file_path)?;
                }
                Statement::ImportFrom { module, items } => {
                    self.load_module_items(module, items, file_path)?;
                }
                _ => {}
            }
        }
        
        // Second pass: collect all function declarations
        for statement in &program.statements {
            if let Statement::FunctionDeclaration { name, parameters, body, return_type, .. } = statement {
                let func = Function {
                    name: name.clone(),
                    parameters: parameters.clone(),
                    body: body.clone(),
                    return_type: return_type.clone(),
                };
                self.global_env.define_function(func);
            }
        }
        
        // Execute main function if it exists
        if let Ok(main_func) = self.global_env.get_function("main") {
            let main_func = main_func.clone();
            match self.execute_function(&main_func, &[]) {
                Ok(value) => Ok(value.to_int().unwrap_or(0) as i32),
                Err(InterpreterError::ReturnValue(value)) => Ok(value.to_int().unwrap_or(0) as i32),
                Err(e) => Err(e),
            }
        } else {
            // Execute statements in order
            let mut env = self.global_env.clone();
            for statement in &program.statements {
                match self.execute_statement(statement, &mut env) {
                    Ok(_) => {}
                    Err(InterpreterError::ReturnValue(value)) => {
                        return Ok(value.to_int().unwrap_or(0) as i32);
                    }
                    Err(e) => return Err(e),
                }
            }
            Ok(0)
        }
    }
    
    fn load_module(&mut self, module_path: &str, alias: Option<&str>, importing_file: Option<&str>) -> Result<(), InterpreterError> {
        let module_program = self.parse_module(module_path, importing_file)?;
        let namespace = alias.unwrap_or(module_path);
        
        // Load exported functions from the module with qualified names
        for statement in &module_program.statements {
            if let Statement::FunctionDeclaration { name, parameters, body, return_type, is_exported } = statement {
                if *is_exported {
                    let qualified_name = format!("{}.{}", namespace, name);
                    let func = Function {
                        name: qualified_name.clone(),
                        parameters: parameters.clone(),
                        body: body.clone(),
                        return_type: return_type.clone(),
                    };
                    self.global_env.define_function(func);
                }
            }
        }
        
        // Load exported constants from the module with qualified names
        for statement in &module_program.statements {
            if let Statement::LetDeclaration { name, initializer, is_exported } = statement {
                if *is_exported {
                    if let Some(init_expr) = initializer {
                        let mut temp_env = self.global_env.clone();
                        let value = self.evaluate_expression(init_expr, &mut temp_env)?;
                        let qualified_name = format!("{}.{}", namespace, name);
                        self.global_env.define_variable(qualified_name, value);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn load_module_items(&mut self, module_path: &str, imports: &[(String, Option<String>)], importing_file: Option<&str>) -> Result<(), InterpreterError> {
        let module_program = self.parse_module(module_path, importing_file)?;
        
        for (item_name, alias) in imports {
            let local_name = alias.as_ref().unwrap_or(item_name);
            
            // Look for exported functions
            for statement in &module_program.statements {
                if let Statement::FunctionDeclaration { name, parameters, body, return_type, is_exported } = statement {
                    if name == item_name && *is_exported {
                        let func = Function {
                            name: local_name.clone(),
                            parameters: parameters.clone(),
                            body: body.clone(),
                            return_type: return_type.clone(),
                        };
                        self.global_env.define_function(func);
                        break;
                    }
                }
            }
            
            // Look for exported constants
            for statement in &module_program.statements {
                if let Statement::LetDeclaration { name, initializer, is_exported } = statement {
                    if name == item_name && *is_exported {
                        if let Some(init_expr) = initializer {
                            let mut temp_env = self.global_env.clone();
                            let value = self.evaluate_expression(init_expr, &mut temp_env)?;
                            self.global_env.define_variable(local_name.clone(), value);
                        }
                        break;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn parse_module(&self, module_path: &str, importing_file: Option<&str>) -> Result<Program, InterpreterError> {
        let file_path = if let Some(importing_file) = importing_file {
            // Resolve relative to the importing file's directory
            let importing_dir = std::path::Path::new(importing_file)
                .parent()
                .unwrap_or(std::path::Path::new("."));
            importing_dir.join(format!("{}.nlang", module_path))
                .to_string_lossy()
                .to_string()
        } else {
            format!("{}.nlang", module_path)
        };
        
        let content = fs::read_to_string(&file_path)
            .map_err(|_| InterpreterError::InvalidOperation { 
                message: format!("Could not read module file: {}", file_path) 
            })?;
        
        let mut lexer = Lexer::new(&content);
        let tokens = lexer.tokenize()
            .map_err(|e| InterpreterError::InvalidOperation { 
                message: format!("Lexer error in module {}: {:?}", module_path, e) 
            })?;
        
        let mut parser = Parser::new(&tokens);
        let statements = parser.parse_program()
            .map_err(|e| InterpreterError::InvalidOperation { 
                message: format!("Parser error in module {}: {:?}", module_path, e) 
            })?;
        
        Ok(Program { statements })
    }
    
    fn execute_function(&mut self, func: &Function, args: &[Value]) -> Result<Value, InterpreterError> {
        let mut local_env = self.global_env.clone();
        
        // Bind parameters
        for (param, arg) in func.parameters.iter().zip(args.iter()) {
            local_env.define_variable(param.name.clone(), arg.clone());
        }
        
        // Execute function body
        for statement in &func.body {
            match self.execute_statement(statement, &mut local_env) {
                Ok(_) => {}
                Err(InterpreterError::ReturnValue(value)) => return Ok(value),
                Err(e) => return Err(e),
            }
        }
        
        // Default return value
        Ok(Value::Integer(0))
    }
    
    fn execute_statement(&mut self, stmt: &Statement, env: &mut Environment) -> Result<(), InterpreterError> {
        match stmt {
            Statement::LetDeclaration { name, initializer, .. } => {
                if let Some(init_expr) = initializer {
                    let val = self.evaluate_expression(init_expr, env)?;
                    env.define_variable(name.clone(), val);
                } else {
                    // Default initialization based on type (if we had type info)
                    env.define_variable(name.clone(), Value::Integer(0));
                }
                Ok(())
            }
            Statement::Return { value } => {
                if let Some(ret_expr) = value {
                    let val = self.evaluate_expression(ret_expr, env)?;
                    Err(InterpreterError::ReturnValue(val))
                } else {
                    Err(InterpreterError::ReturnValue(Value::Integer(0)))
                }
            }
            Statement::If { condition, then_branch, else_branch } => {
                let cond_val = self.evaluate_expression(condition, env)?;
                if cond_val.to_bool()? {
                    self.execute_statement(then_branch, env)?;
                } else if let Some(else_stmt) = else_branch {
                    self.execute_statement(else_stmt, env)?;
                }
                Ok(())
            }
            Statement::While { condition, body } => {
                loop {
                    let cond_val = self.evaluate_expression(condition, env)?;
                    if !cond_val.to_bool()? {
                        break;
                    }
                    match self.execute_statement(body, env) {
                        Ok(()) => {},
                        Err(InterpreterError::Break) => break,
                        Err(InterpreterError::Continue) => continue,
                        Err(other) => return Err(other),
                    }
                }
                Ok(())
            }
            Statement::FunctionDeclaration { .. } => {
                // Already handled in first pass
                Ok(())
            }
            Statement::Expression(expr) => {
                self.evaluate_expression(expr, env)?;
                Ok(())
            }
            Statement::Block { statements } => {
                for stmt in statements {
                    match self.execute_statement(stmt, env) {
                        Ok(()) => {},
                        Err(InterpreterError::Break) => return Err(InterpreterError::Break),
                        Err(InterpreterError::Continue) => return Err(InterpreterError::Continue),
                        Err(other) => return Err(other),
                    }
                }
                Ok(())
            }
            Statement::Break => {
                Err(InterpreterError::Break)
            }
            Statement::Continue => {
                Err(InterpreterError::Continue)
            }
            _ => {
                // Handle other statement types as needed
                Ok(())
            }
        }
    }
    
    fn evaluate_expression(&mut self, expr: &Expr, env: &mut Environment) -> Result<Value, InterpreterError> {
        match expr {
            Expr::Literal(literal) => {
                match literal {
                    Literal::Integer(i) => Ok(Value::Integer(*i)),
                    Literal::Float(f) => Ok(Value::Float(*f)),
                    Literal::Boolean(b) => Ok(Value::Boolean(*b)),
                    Literal::String(s) => Ok(Value::String(s.clone())),
                    Literal::Null => Ok(Value::Integer(0)), // Default null to 0
                }
            }
            Expr::Variable(name) => {
                env.get_variable(name)
            }
            Expr::Binary { left, operator, right } => {
                let left_val = self.evaluate_expression(left, env)?;
                let right_val = self.evaluate_expression(right, env)?;
                self.evaluate_binary_op(&left_val, operator, &right_val)
            }
            Expr::Unary { operator, operand } => {
                let val = self.evaluate_expression(operand, env)?;
                self.evaluate_unary_op(operator, &val)
            }
            Expr::Call { callee, arguments } => {
                // Handle different types of function calls
                let func_name = match callee.as_ref() {
                    Expr::Variable(name) => name.clone(),
                    Expr::Get { object, name } => {
                        // Handle module-qualified function calls (e.g., math.add())
                        if let Expr::Variable(namespace_name) = object.as_ref() {
                            format!("{}.{}", namespace_name, name)
                        } else {
                            return Err(InterpreterError::InvalidOperation {
                                message: "Complex function calls not yet supported".to_string(),
                            });
                        }
                    },
                    _ => {
                        return Err(InterpreterError::InvalidOperation {
                            message: "Complex function calls not yet supported".to_string(),
                        });
                    }
                };

                // Handle built-in functions
                match func_name.as_str() {
                        "print" => {
                            if arguments.len() != 1 {
                                return Err(InterpreterError::InvalidOperation {
                                    message: "print function requires 1 argument".to_string(),
                                });
                            }
                            let arg = self.evaluate_expression(&arguments[0], env)?;
                            match arg {
                                Value::String(s) => print!("{}", s),
                                Value::Integer(i) => print!("{}", i),
                                Value::Float(f) => print!("{}", f),
                                Value::Boolean(b) => print!("{}", b),
                            }
                            use std::io::{self, Write};
                            io::stdout().flush().unwrap();
                            Ok(Value::Integer(0)) // Return null/void equivalent
                        }
                        "println" => {
                            if arguments.len() != 1 {
                                return Err(InterpreterError::InvalidOperation {
                                    message: "println function requires 1 argument".to_string(),
                                });
                            }
                            let arg = self.evaluate_expression(&arguments[0], env)?;
                            match arg {
                                Value::String(s) => println!("{}", s),
                                Value::Integer(i) => println!("{}", i),
                                Value::Float(f) => println!("{}", f),
                                Value::Boolean(b) => println!("{}", b),
                            }
                            Ok(Value::Integer(0)) // Return null/void equivalent
                        }
                        "add" => {
                            if arguments.len() != 2 {
                                return Err(InterpreterError::InvalidOperation {
                                    message: "add function requires 2 arguments".to_string(),
                                });
                            }
                            let arg1 = self.evaluate_expression(&arguments[0], env)?;
                            let arg2 = self.evaluate_expression(&arguments[1], env)?;
                            self.evaluate_binary_op(&arg1, &BinaryOperator::Plus, &arg2)
                        }
                        "multiply" => {
                            if arguments.len() != 2 {
                                return Err(InterpreterError::InvalidOperation {
                                    message: "multiply function requires 2 arguments".to_string(),
                                });
                            }
                            let arg1 = self.evaluate_expression(&arguments[0], env)?;
                            let arg2 = self.evaluate_expression(&arguments[1], env)?;
                            self.evaluate_binary_op(&arg1, &BinaryOperator::Star, &arg2)
                        }
                        _ => {
                            // User-defined function
                            
                            // First try to find the function as-is
                            if let Ok(func) = env.get_function(&func_name) {
                                let func = func.clone();
                                let mut args = Vec::new();
                                for arg_expr in arguments {
                                    args.push(self.evaluate_expression(arg_expr, env)?);
                                }
                                return self.execute_function(&func, &args);
                            }
                            
                            // If not found, try to find it in the math namespace (for recursive calls)
                            let qualified_name = format!("math.{}", func_name);
                            if let Ok(func) = env.get_function(&qualified_name) {
                                let func = func.clone();
                                let mut args = Vec::new();
                                for arg_expr in arguments {
                                    args.push(self.evaluate_expression(arg_expr, env)?);
                                }
                                return self.execute_function(&func, &args);
                            }
                            
                            // If still not found, return error
                            Err(InterpreterError::FunctionNotFound { name: func_name })
                        }
                    }
            }
            Expr::Get { object, name } => {
                // Handle namespace access like math.PI
                if let Expr::Variable(obj_name) = object.as_ref() {
                    if obj_name == "math" {
                        match name.as_str() {
                            "PI" => Ok(Value::Float(std::f64::consts::PI)),
                            _ => Err(InterpreterError::InvalidOperation {
                                message: format!("Unknown math property: {}", name),
                            }),
                        }
                    } else {
                        Err(InterpreterError::InvalidOperation {
                            message: format!("Unknown namespace: {}", obj_name),
                        })
                    }
                } else {
                    Err(InterpreterError::InvalidOperation {
                        message: "Complex object access not yet supported".to_string(),
                    })
                }
            }
            Expr::Assign { name, value } => {
                let val = self.evaluate_expression(value, env)?;
                env.set_variable(name.clone(), val.clone())?;
                Ok(val)
            }
            _ => {
                Err(InterpreterError::InvalidOperation {
                    message: "Expression type not yet supported".to_string(),
                })
            }
        }
    }
    
    fn evaluate_binary_op(&self, left: &Value, op: &BinaryOperator, right: &Value) -> Result<Value, InterpreterError> {
        match op {
            BinaryOperator::Plus => {
                match (left, right) {
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a + b)),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                    (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
                    (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a + *b as f64)),
                    (Value::String(a), Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
                    _ => Err(InterpreterError::TypeMismatch {
                        expected: "numeric or string".to_string(),
                        actual: format!("{} and {}", left.type_name(), right.type_name()),
                    }),
                }
            }
            BinaryOperator::Minus => {
                match (left, right) {
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a - b)),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
                    (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
                    (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a - *b as f64)),
                    _ => Err(InterpreterError::TypeMismatch {
                        expected: "numeric".to_string(),
                        actual: format!("{} and {}", left.type_name(), right.type_name()),
                    }),
                }
            }
            BinaryOperator::Star => {
                match (left, right) {
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a * b)),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
                    (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
                    (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a * *b as f64)),
                    _ => Err(InterpreterError::TypeMismatch {
                        expected: "numeric".to_string(),
                        actual: format!("{} and {}", left.type_name(), right.type_name()),
                    }),
                }
            }
            BinaryOperator::Slash => {
                match (left, right) {
                    (Value::Integer(a), Value::Integer(b)) => {
                        if *b == 0 {
                            Err(InterpreterError::DivisionByZero)
                        } else {
                            Ok(Value::Float(*a as f64 / *b as f64))
                        }
                    }
                    (Value::Float(a), Value::Float(b)) => {
                        if *b == 0.0 {
                            Err(InterpreterError::DivisionByZero)
                        } else {
                            Ok(Value::Float(a / b))
                        }
                    }
                    (Value::Integer(a), Value::Float(b)) => {
                        if *b == 0.0 {
                            Err(InterpreterError::DivisionByZero)
                        } else {
                            Ok(Value::Float(*a as f64 / b))
                        }
                    }
                    (Value::Float(a), Value::Integer(b)) => {
                        if *b == 0 {
                            Err(InterpreterError::DivisionByZero)
                        } else {
                            Ok(Value::Float(a / *b as f64))
                        }
                    }
                    _ => Err(InterpreterError::TypeMismatch {
                        expected: "numeric".to_string(),
                        actual: format!("{} and {}", left.type_name(), right.type_name()),
                    }),
                }
            }
            BinaryOperator::Percent => {
                match (left, right) {
                    (Value::Integer(a), Value::Integer(b)) => {
                        if *b == 0 {
                            Err(InterpreterError::DivisionByZero)
                        } else {
                            Ok(Value::Integer(a % b))
                        }
                    }
                    _ => Err(InterpreterError::TypeMismatch {
                        expected: "integer".to_string(),
                        actual: format!("{} and {}", left.type_name(), right.type_name()),
                    }),
                }
            }
            BinaryOperator::EqualEqual => {
                Ok(Value::Boolean(left == right))
            }
            BinaryOperator::NotEqual => {
                Ok(Value::Boolean(left != right))
            }
            BinaryOperator::Less => {
                match (left, right) {
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a < b)),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Boolean(a < b)),
                    (Value::Integer(a), Value::Float(b)) => Ok(Value::Boolean((*a as f64) < *b)),
                    (Value::Float(a), Value::Integer(b)) => Ok(Value::Boolean(*a < (*b as f64))),
                    _ => Err(InterpreterError::TypeMismatch {
                        expected: "numeric".to_string(),
                        actual: format!("{} and {}", left.type_name(), right.type_name()),
                    }),
                }
            }
            BinaryOperator::LessEqual => {
                match (left, right) {
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a <= b)),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Boolean(a <= b)),
                    (Value::Integer(a), Value::Float(b)) => Ok(Value::Boolean((*a as f64) <= *b)),
                    (Value::Float(a), Value::Integer(b)) => Ok(Value::Boolean(*a <= (*b as f64))),
                    _ => Err(InterpreterError::TypeMismatch {
                        expected: "numeric".to_string(),
                        actual: format!("{} and {}", left.type_name(), right.type_name()),
                    }),
                }
            }
            BinaryOperator::Greater => {
                match (left, right) {
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a > b)),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Boolean(a > b)),
                    (Value::Integer(a), Value::Float(b)) => Ok(Value::Boolean((*a as f64) > *b)),
                    (Value::Float(a), Value::Integer(b)) => Ok(Value::Boolean(*a > (*b as f64))),
                    _ => Err(InterpreterError::TypeMismatch {
                        expected: "numeric".to_string(),
                        actual: format!("{} and {}", left.type_name(), right.type_name()),
                    }),
                }
            }
            BinaryOperator::GreaterEqual => {
                match (left, right) {
                    (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a >= b)),
                    (Value::Float(a), Value::Float(b)) => Ok(Value::Boolean(a >= b)),
                    (Value::Integer(a), Value::Float(b)) => Ok(Value::Boolean((*a as f64) >= *b)),
                    (Value::Float(a), Value::Integer(b)) => Ok(Value::Boolean(*a >= (*b as f64))),
                    _ => Err(InterpreterError::TypeMismatch {
                        expected: "numeric".to_string(),
                        actual: format!("{} and {}", left.type_name(), right.type_name()),
                    }),
                }
            }
            BinaryOperator::And => {
                Ok(Value::Boolean(left.to_bool()? && right.to_bool()?))
            }
            BinaryOperator::Or => {
                Ok(Value::Boolean(left.to_bool()? || right.to_bool()?))
            }
        }
    }
    
    fn evaluate_unary_op(&self, op: &UnaryOperator, operand: &Value) -> Result<Value, InterpreterError> {
        match op {
            UnaryOperator::Negate => {
                match operand {
                    Value::Integer(i) => Ok(Value::Integer(-i)),
                    Value::Float(f) => Ok(Value::Float(-f)),
                    _ => Err(InterpreterError::TypeMismatch {
                        expected: "numeric".to_string(),
                        actual: operand.type_name().to_string(),
                    }),
                }
            }
            UnaryOperator::Not => {
                Ok(Value::Boolean(!operand.to_bool()?))
            }
        }
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;
    
    #[test]
    fn test_simple_arithmetic() {
        let mut interpreter = Interpreter::new();
        let program = Program {
            statements: vec![
                Statement::LetDeclaration {
                    name: "x".to_string(),
                    initializer: Some(Expr::Literal(Literal::Integer(5))),
                    is_exported: false,
                },
                Statement::Return {
                    value: Some(Box::new(Expr::Variable("x".to_string()))),
                },
            ],
        };
        
        let result = interpreter.execute_program(&program);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 5);
    }
}