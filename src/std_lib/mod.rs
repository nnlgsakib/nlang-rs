//! Standard library for Nlang
//! This module contains the built-in functions and types available in Nlang

use crate::ast::{Expr, Literal, Type};
use std::io::{self, Write};

pub struct StdLib {
    pub functions: Vec<BuiltInFunction>,
    pub types: Vec<BuiltInType>,
}

pub struct BuiltInFunction {
    pub name: String,
    pub parameters: Vec<Type>,
    pub return_type: Type,
    pub implementation: fn(&[Expr]) -> Result<Expr, String>, // Better error handling
}

pub struct BuiltInType {
    pub name: String,
    pub methods: Vec<BuiltInMethod>,
}

pub struct BuiltInMethod {
    pub name: String,
    pub parameters: Vec<Type>,
    pub return_type: Type,
}

impl StdLib {
    pub fn new() -> Self {
        Self {
            functions: vec![
                // I/O Functions
                BuiltInFunction {
                    name: "print".to_string(),
                    parameters: vec![Type::String],
                    return_type: Type::Void,
                    implementation: builtin_print,
                },
                BuiltInFunction {
                    name: "println".to_string(),
                    parameters: vec![Type::String],
                    return_type: Type::Void,
                    implementation: builtin_println,
                },
                BuiltInFunction {
                    name: "input".to_string(),
                    parameters: vec![],
                    return_type: Type::String,
                    implementation: builtin_input,
                },
                
                // String Functions
                BuiltInFunction {
                    name: "len".to_string(),
                    parameters: vec![Type::String],
                    return_type: Type::Integer,
                    implementation: builtin_len,
                },
                
                // Type Conversion Functions
                BuiltInFunction {
                    name: "int".to_string(),
                    parameters: vec![Type::String],
                    return_type: Type::Integer,
                    implementation: builtin_int,
                },
                BuiltInFunction {
                    name: "str".to_string(),
                    parameters: vec![Type::Integer],
                    return_type: Type::String,
                    implementation: builtin_str,
                },
                BuiltInFunction {
                    name: "float".to_string(),
                    parameters: vec![Type::String],
                    return_type: Type::Float,
                    implementation: builtin_float,
                },
                
                // Mathematical Functions
                BuiltInFunction {
                    name: "abs".to_string(),
                    parameters: vec![Type::Integer],
                    return_type: Type::Integer,
                    implementation: builtin_abs,
                },
                BuiltInFunction {
                    name: "max".to_string(),
                    parameters: vec![Type::Integer, Type::Integer],
                    return_type: Type::Integer,
                    implementation: builtin_max,
                },
                BuiltInFunction {
                    name: "min".to_string(),
                    parameters: vec![Type::Integer, Type::Integer],
                    return_type: Type::Integer,
                    implementation: builtin_min,
                },
                BuiltInFunction {
                    name: "pow".to_string(),
                    parameters: vec![Type::Integer, Type::Integer],
                    return_type: Type::Integer,
                    implementation: builtin_pow,
                },
                
                // Boolean Functions
                BuiltInFunction {
                    name: "bool".to_string(),
                    parameters: vec![Type::Integer],
                    return_type: Type::Boolean,
                    implementation: builtin_bool,
                },
            ],
            types: vec![
                BuiltInType {
                    name: "list".to_string(),
                    methods: vec![
                        BuiltInMethod {
                            name: "append".to_string(),
                            parameters: vec![Type::Integer], // Placeholder type
                            return_type: Type::Void,
                        },
                        BuiltInMethod {
                            name: "len".to_string(),
                            parameters: vec![],
                            return_type: Type::Integer,
                        },
                        BuiltInMethod {
                            name: "pop".to_string(),
                            parameters: vec![],
                            return_type: Type::Integer, // Placeholder
                        },
                        BuiltInMethod {
                            name: "clear".to_string(),
                            parameters: vec![],
                            return_type: Type::Void,
                        },
                    ],
                },
                BuiltInType {
                    name: "string".to_string(),
                    methods: vec![
                        BuiltInMethod {
                            name: "len".to_string(),
                            parameters: vec![],
                            return_type: Type::Integer,
                        },
                        BuiltInMethod {
                            name: "upper".to_string(),
                            parameters: vec![],
                            return_type: Type::String,
                        },
                        BuiltInMethod {
                            name: "lower".to_string(),
                            parameters: vec![],
                            return_type: Type::String,
                        },
                        BuiltInMethod {
                            name: "trim".to_string(),
                            parameters: vec![],
                            return_type: Type::String,
                        },
                        BuiltInMethod {
                            name: "contains".to_string(),
                            parameters: vec![Type::String],
                            return_type: Type::Boolean,
                        },
                    ],
                },
            ],
        }
    }
    
    /// Check if a function name is a built-in function
    pub fn is_builtin_function(&self, name: &str) -> bool {
        self.functions.iter().any(|f| f.name == name)
    }
    
    /// Get a built-in function by name
    pub fn get_builtin_function(&self, name: &str) -> Option<&BuiltInFunction> {
        self.functions.iter().find(|f| f.name == name)
    }
    
    /// Check if a type name is a built-in type
    pub fn is_builtin_type(&self, name: &str) -> bool {
        self.types.iter().any(|t| t.name == name)
    }
    
    /// Execute a built-in function
    pub fn execute_builtin(&self, name: &str, args: &[Expr]) -> Result<Expr, String> {
        if let Some(func) = self.get_builtin_function(name) {
            (func.implementation)(args)
        } else {
            Err(format!("Unknown built-in function: {}", name))
        }
    }
}

impl Default for StdLib {
    fn default() -> Self {
        Self::new()
    }
}

// Helper function to extract string value from expression
fn extract_string_value(expr: &Expr) -> Result<String, String> {
    match expr {
        Expr::Literal(Literal::String(s)) => Ok(s.clone()),
        _ => Err("Expected string literal".to_string()),
    }
}

// Helper function to extract integer value from expression
fn extract_integer_value(expr: &Expr) -> Result<i64, String> {
    match expr {
        Expr::Literal(Literal::Integer(i)) => Ok(*i),
        _ => Err("Expected integer literal".to_string()),
    }
}

// Helper function to extract float value from expression
fn extract_float_value(expr: &Expr) -> Result<f64, String> {
    match expr {
        Expr::Literal(Literal::Float(f)) => Ok(*f),
        _ => Err("Expected float literal".to_string()),
    }
}

// Helper function to extract boolean value from expression
fn extract_boolean_value(expr: &Expr) -> Result<bool, String> {
    match expr {
        Expr::Literal(Literal::Boolean(b)) => Ok(*b),
        _ => Err("Expected boolean literal".to_string()),
    }
}

// Helper function to convert any expression to string for printing
fn expr_to_string(expr: &Expr) -> Result<String, String> {
    match expr {
        Expr::Literal(Literal::String(s)) => Ok(s.clone()),
        Expr::Literal(Literal::Integer(i)) => Ok(i.to_string()),
        Expr::Literal(Literal::Float(f)) => Ok(f.to_string()),
        Expr::Literal(Literal::Boolean(b)) => Ok(b.to_string()),
        Expr::Literal(Literal::Null) => Ok("null".to_string()),
        _ => Err("Cannot convert expression to string".to_string()),
    }
}

// Built-in function implementations
fn builtin_print(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("print() takes exactly 1 argument".to_string());
    }
    
    let text = expr_to_string(&args[0])?;
    print!("{}", text);
    io::stdout().flush().map_err(|e| format!("IO error: {}", e))?;
    
    Ok(Expr::Literal(Literal::Null))
}

fn builtin_println(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("println() takes exactly 1 argument".to_string());
    }
    
    let text = expr_to_string(&args[0])?;
    println!("{}", text);
    
    Ok(Expr::Literal(Literal::Null))
}

fn builtin_input(_args: &[Expr]) -> Result<Expr, String> {
    let mut input = String::new();
    io::stdin().read_line(&mut input)
        .map_err(|e| format!("IO error: {}", e))?;
    
    // Remove trailing newline
    if input.ends_with('\n') {
        input.pop();
        if input.ends_with('\r') {
            input.pop();
        }
    }
    
    Ok(Expr::Literal(Literal::String(input)))
}

fn builtin_len(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("len() takes exactly 1 argument".to_string());
    }
    
    let text = extract_string_value(&args[0])?;
    Ok(Expr::Literal(Literal::Integer(text.len() as i64)))
}

fn builtin_int(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("int() takes exactly 1 argument".to_string());
    }
    
    let text = extract_string_value(&args[0])?;
    let parsed = text.trim().parse::<i64>()
        .map_err(|_| format!("Cannot convert '{}' to integer", text))?;
    
    Ok(Expr::Literal(Literal::Integer(parsed)))
}

fn builtin_str(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("str() takes exactly 1 argument".to_string());
    }
    
    let num = extract_integer_value(&args[0])?;
    Ok(Expr::Literal(Literal::String(num.to_string())))
}

fn builtin_float(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("float() takes exactly 1 argument".to_string());
    }
    
    let text = extract_string_value(&args[0])?;
    let parsed = text.trim().parse::<f64>()
        .map_err(|_| format!("Cannot convert '{}' to float", text))?;
    
    Ok(Expr::Literal(Literal::Float(parsed)))
}

fn builtin_abs(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("abs() takes exactly 1 argument".to_string());
    }
    
    let num = extract_integer_value(&args[0])?;
    Ok(Expr::Literal(Literal::Integer(num.abs())))
}

fn builtin_max(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 2 {
        return Err("max() takes exactly 2 arguments".to_string());
    }
    
    let a = extract_integer_value(&args[0])?;
    let b = extract_integer_value(&args[1])?;
    Ok(Expr::Literal(Literal::Integer(a.max(b))))
}

fn builtin_min(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 2 {
        return Err("min() takes exactly 2 arguments".to_string());
    }
    
    let a = extract_integer_value(&args[0])?;
    let b = extract_integer_value(&args[1])?;
    Ok(Expr::Literal(Literal::Integer(a.min(b))))
}

fn builtin_pow(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 2 {
        return Err("pow() takes exactly 2 arguments".to_string());
    }
    
    let base = extract_integer_value(&args[0])?;
    let exp = extract_integer_value(&args[1])?;
    
    if exp < 0 {
        return Err("Negative exponents not supported for integer power".to_string());
    }
    
    let result = base.pow(exp as u32);
    Ok(Expr::Literal(Literal::Integer(result)))
}

fn builtin_bool(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("bool() takes exactly 1 argument".to_string());
    }
    
    let num = extract_integer_value(&args[0])?;
    Ok(Expr::Literal(Literal::Boolean(num != 0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_std_lib_creation() {
        let std_lib = StdLib::new();
        assert!(!std_lib.functions.is_empty());
        assert!(std_lib.is_builtin_function("print"));
        assert!(std_lib.is_builtin_function("println"));
        assert!(std_lib.is_builtin_function("input"));
        assert!(std_lib.is_builtin_function("len"));
        assert!(std_lib.is_builtin_function("abs"));
        assert!(std_lib.is_builtin_function("max"));
        assert!(std_lib.is_builtin_function("min"));
    }
    
    #[test]
    fn test_builtin_functions() {
        let std_lib = StdLib::new();
        assert!(std_lib.get_builtin_function("print").is_some());
        assert!(std_lib.get_builtin_function("nonexistent").is_none());
    }
    
    #[test]
    fn test_string_length() {
        let args = vec![Expr::Literal(Literal::String("hello".to_string()))];
        let result = builtin_len(&args).unwrap();
        
        match result {
            Expr::Literal(Literal::Integer(5)) => {},
            _ => panic!("Expected integer 5"),
        }
    }
    
    #[test]
    fn test_int_conversion() {
        let args = vec![Expr::Literal(Literal::String("42".to_string()))];
        let result = builtin_int(&args).unwrap();
        
        match result {
            Expr::Literal(Literal::Integer(42)) => {},
            _ => panic!("Expected integer 42"),
        }
    }
    
    #[test]
    fn test_mathematical_functions() {
        // Test abs
        let args = vec![Expr::Literal(Literal::Integer(-5))];
        let result = builtin_abs(&args).unwrap();
        assert!(matches!(result, Expr::Literal(Literal::Integer(5))));
        
        // Test max
        let args = vec![
            Expr::Literal(Literal::Integer(3)),
            Expr::Literal(Literal::Integer(7))
        ];
        let result = builtin_max(&args).unwrap();
        assert!(matches!(result, Expr::Literal(Literal::Integer(7))));
        
        // Test min
        let args = vec![
            Expr::Literal(Literal::Integer(3)),
            Expr::Literal(Literal::Integer(7))
        ];
        let result = builtin_min(&args).unwrap();
        assert!(matches!(result, Expr::Literal(Literal::Integer(3))));
    }
}