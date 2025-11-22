//! Standard library for Nlang
//! This module contains the built-in functions and types available in Nlang

pub mod functions;
pub mod nlang;
pub mod string;
pub mod time_lib;
pub mod types;

use self::functions::*;
use self::types::{BuiltInFunction, BuiltInMethod, BuiltInType};
use crate::ast::{Expr, Type};

pub struct StdLib {
    pub functions: Vec<BuiltInFunction>,
    pub types: Vec<BuiltInType>,
}

impl StdLib {
    pub fn new() -> Self {
        Self {
            functions: vec![
                // I/O Functions
                BuiltInFunction {
                    name: "print".to_string(),
                    parameters: vec![],
                    return_type: Type::Void,
                    implementation: Some(builtin_print),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "println".to_string(),
                    parameters: vec![],
                    return_type: Type::Void,
                    implementation: Some(builtin_println),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "input".to_string(),
                    parameters: vec![],
                    return_type: Type::String,
                    implementation: Some(builtin_input),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "input".to_string(),
                    parameters: vec![Type::String],
                    return_type: Type::String,
                    implementation: Some(builtin_input),
                    c_implementation: None,
                    ast_body: None,
                },
                // String Functions
                BuiltInFunction {
                    name: "len".to_string(),
                    parameters: vec![Type::String],
                    return_type: Type::Integer,
                    implementation: Some(builtin_len_string),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "len".to_string(),
                    parameters: vec![Type::Array(Box::new(Type::Void), 0)], // Placeholder for any array
                    return_type: Type::Integer,
                    implementation: Some(builtin_len_array),
                    c_implementation: None,
                    ast_body: None,
                },
                // Cryptographic Functions
                BuiltInFunction {
                    name: "sha256".to_string(),
                    parameters: vec![Type::String],
                    return_type: Type::String,
                    implementation: Some(builtin_sha256_string),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "sha256".to_string(),
                    parameters: vec![Type::Array(Box::new(Type::Void), 0)],
                    return_type: Type::String,
                    implementation: Some(builtin_sha256_array),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "sha256_random".to_string(),
                    parameters: vec![Type::Integer],
                    return_type: Type::String,
                    implementation: Some(builtin_sha256_random),
                    c_implementation: None,
                    ast_body: None,
                },
                // Type Conversion Functions
                BuiltInFunction {
                    name: "int".to_string(),
                    parameters: vec![Type::String],
                    return_type: Type::Integer,
                    implementation: Some(builtin_int),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "int".to_string(),
                    parameters: vec![Type::Float],
                    return_type: Type::Integer,
                    implementation: Some(builtin_int_from_float),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "int".to_string(),
                    parameters: vec![Type::F32],
                    return_type: Type::Integer,
                    implementation: Some(builtin_int_from_float),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "int".to_string(),
                    parameters: vec![Type::F64],
                    return_type: Type::Integer,
                    implementation: Some(builtin_int_from_float),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "str".to_string(),
                    parameters: vec![Type::Integer],
                    return_type: Type::String,
                    implementation: Some(builtin_str),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "str".to_string(),
                    parameters: vec![Type::I8],
                    return_type: Type::String,
                    implementation: Some(builtin_str_i8),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "str".to_string(),
                    parameters: vec![Type::I16],
                    return_type: Type::String,
                    implementation: Some(builtin_str_i16),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "str".to_string(),
                    parameters: vec![Type::I32],
                    return_type: Type::String,
                    implementation: Some(builtin_str_i32),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "str".to_string(),
                    parameters: vec![Type::I64],
                    return_type: Type::String,
                    implementation: Some(builtin_str_i64),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "str".to_string(),
                    parameters: vec![Type::ISize],
                    return_type: Type::String,
                    implementation: Some(builtin_str_isize),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "str".to_string(),
                    parameters: vec![Type::U8],
                    return_type: Type::String,
                    implementation: Some(builtin_str_u8),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "str".to_string(),
                    parameters: vec![Type::U16],
                    return_type: Type::String,
                    implementation: Some(builtin_str_u16),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "str".to_string(),
                    parameters: vec![Type::U32],
                    return_type: Type::String,
                    implementation: Some(builtin_str_u32),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "str".to_string(),
                    parameters: vec![Type::U64],
                    return_type: Type::String,
                    implementation: Some(builtin_str_u64),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "str".to_string(),
                    parameters: vec![Type::USize],
                    return_type: Type::String,
                    implementation: Some(builtin_str_usize),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "str".to_string(),
                    parameters: vec![Type::Float],
                    return_type: Type::String,
                    implementation: Some(builtin_str_from_float),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "str".to_string(),
                    parameters: vec![Type::F32],
                    return_type: Type::String,
                    implementation: Some(builtin_str_from_float),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "str".to_string(),
                    parameters: vec![Type::F64],
                    return_type: Type::String,
                    implementation: Some(builtin_str_from_float),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "str".to_string(),
                    parameters: vec![Type::Boolean],
                    return_type: Type::String,
                    implementation: Some(builtin_str_from_bool),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "float".to_string(),
                    parameters: vec![Type::String],
                    return_type: Type::F64,
                    implementation: Some(builtin_float),
                    c_implementation: None,
                    ast_body: None,
                },
                // Mathematical Functions
                BuiltInFunction {
                    name: "abs".to_string(),
                    parameters: vec![Type::Integer],
                    return_type: Type::Integer,
                    implementation: Some(builtin_abs),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "abs_float".to_string(),
                    parameters: vec![Type::Float],
                    return_type: Type::F64,
                    implementation: Some(builtin_abs_float),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "abs_float".to_string(),
                    parameters: vec![Type::F32],
                    return_type: Type::F64,
                    implementation: Some(builtin_abs_float),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "abs_float".to_string(),
                    parameters: vec![Type::F64],
                    return_type: Type::F64,
                    implementation: Some(builtin_abs_float),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "max".to_string(),
                    parameters: vec![Type::Integer, Type::Integer],
                    return_type: Type::Integer,
                    implementation: Some(builtin_max),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "min".to_string(),
                    parameters: vec![Type::Integer, Type::Integer],
                    return_type: Type::Integer,
                    implementation: Some(builtin_min),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "pow".to_string(),
                    parameters: vec![Type::Integer, Type::Integer],
                    return_type: Type::Integer,
                    implementation: Some(builtin_pow),
                    c_implementation: None,
                    ast_body: None,
                },
                // Boolean Functions
                BuiltInFunction {
                    name: "bool".to_string(),
                    parameters: vec![Type::Integer],
                    return_type: Type::Boolean,
                    implementation: Some(builtin_bool),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "now".to_string(),
                    parameters: vec![],
                    return_type: Type::String,
                    implementation: Some(time_lib::builtin_now),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "now_utc".to_string(),
                    parameters: vec![],
                    return_type: Type::String,
                    implementation: Some(time_lib::builtin_now_utc),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "now_local".to_string(),
                    parameters: vec![],
                    return_type: Type::String,
                    implementation: Some(time_lib::builtin_now_local),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "timestamp".to_string(),
                    parameters: vec![],
                    return_type: Type::Integer,
                    implementation: Some(time_lib::builtin_timestamp),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "timestamp_ms".to_string(),
                    parameters: vec![],
                    return_type: Type::Integer,
                    implementation: Some(time_lib::builtin_timestamp_ms),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "timestamp_us".to_string(),
                    parameters: vec![],
                    return_type: Type::Integer,
                    implementation: Some(time_lib::builtin_timestamp_us),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "timestamp_ns".to_string(),
                    parameters: vec![],
                    return_type: Type::Integer,
                    implementation: Some(time_lib::builtin_timestamp_ns),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "sleep".to_string(),
                    parameters: vec![Type::Integer],
                    return_type: Type::Void,
                    implementation: Some(time_lib::builtin_sleep),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "sleep_ms".to_string(),
                    parameters: vec![Type::Integer],
                    return_type: Type::Void,
                    implementation: Some(time_lib::builtin_sleep_ms),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "sleep_ns".to_string(),
                    parameters: vec![Type::Integer],
                    return_type: Type::Void,
                    implementation: Some(time_lib::builtin_sleep_ns),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "year".to_string(),
                    parameters: vec![],
                    return_type: Type::Integer,
                    implementation: Some(time_lib::builtin_year),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "month".to_string(),
                    parameters: vec![],
                    return_type: Type::Integer,
                    implementation: Some(time_lib::builtin_month),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "day".to_string(),
                    parameters: vec![],
                    return_type: Type::Integer,
                    implementation: Some(time_lib::builtin_day),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "weekday".to_string(),
                    parameters: vec![],
                    return_type: Type::Integer,
                    implementation: Some(time_lib::builtin_weekday),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "hour".to_string(),
                    parameters: vec![],
                    return_type: Type::Integer,
                    implementation: Some(time_lib::builtin_hour),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "minute".to_string(),
                    parameters: vec![],
                    return_type: Type::Integer,
                    implementation: Some(time_lib::builtin_minute),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "second".to_string(),
                    parameters: vec![],
                    return_type: Type::Integer,
                    implementation: Some(time_lib::builtin_second),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "nanosecond".to_string(),
                    parameters: vec![],
                    return_type: Type::Integer,
                    implementation: Some(time_lib::builtin_nanosecond),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "to_local".to_string(),
                    parameters: vec![Type::Integer],
                    return_type: Type::String,
                    implementation: Some(time_lib::builtin_to_local),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "to_utc".to_string(),
                    parameters: vec![Type::Integer],
                    return_type: Type::String,
                    implementation: Some(time_lib::builtin_to_utc),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "from_timestamp".to_string(),
                    parameters: vec![Type::Integer],
                    return_type: Type::String,
                    implementation: Some(time_lib::builtin_from_timestamp),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "from_timestamp_ms".to_string(),
                    parameters: vec![Type::Integer],
                    return_type: Type::String,
                    implementation: Some(time_lib::builtin_from_timestamp_ms),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "format".to_string(),
                    parameters: vec![Type::String],
                    return_type: Type::String,
                    implementation: Some(time_lib::builtin_format_now_local),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "time_to_string".to_string(),
                    parameters: vec![],
                    return_type: Type::String,
                    implementation: Some(time_lib::builtin_to_string_now_local),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "parse".to_string(),
                    parameters: vec![Type::String],
                    return_type: Type::Integer,
                    implementation: Some(time_lib::builtin_parse_date),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "parse_rfc3339".to_string(),
                    parameters: vec![Type::String],
                    return_type: Type::Integer,
                    implementation: Some(time_lib::builtin_parse_rfc3339),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "parse_rfc2822".to_string(),
                    parameters: vec![Type::String],
                    return_type: Type::Integer,
                    implementation: Some(time_lib::builtin_parse_rfc2822),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "duration_from_seconds".to_string(),
                    parameters: vec![Type::Integer],
                    return_type: Type::Vault(Box::new(Type::String), Box::new(Type::Integer)),
                    implementation: Some(time_lib::builtin_duration_from_seconds),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "duration_from_millis".to_string(),
                    parameters: vec![Type::Integer],
                    return_type: Type::Vault(Box::new(Type::String), Box::new(Type::Integer)),
                    implementation: Some(time_lib::builtin_duration_from_millis),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "duration_from_nanos".to_string(),
                    parameters: vec![Type::Integer],
                    return_type: Type::Vault(Box::new(Type::String), Box::new(Type::Integer)),
                    implementation: Some(time_lib::builtin_duration_from_nanos),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "duration_as_secs".to_string(),
                    parameters: vec![Type::Vault(Box::new(Type::String), Box::new(Type::Integer))],
                    return_type: Type::Integer,
                    implementation: Some(time_lib::builtin_duration_as_secs),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "duration_as_millis".to_string(),
                    parameters: vec![Type::Vault(Box::new(Type::String), Box::new(Type::Integer))],
                    return_type: Type::Integer,
                    implementation: Some(time_lib::builtin_duration_as_millis),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "duration_add".to_string(),
                    parameters: vec![
                        Type::Vault(Box::new(Type::String), Box::new(Type::Integer)),
                        Type::Vault(Box::new(Type::String), Box::new(Type::Integer)),
                    ],
                    return_type: Type::Vault(Box::new(Type::String), Box::new(Type::Integer)),
                    implementation: Some(time_lib::builtin_duration_add),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "duration_sub".to_string(),
                    parameters: vec![
                        Type::Vault(Box::new(Type::String), Box::new(Type::Integer)),
                        Type::Vault(Box::new(Type::String), Box::new(Type::Integer)),
                    ],
                    return_type: Type::Vault(Box::new(Type::String), Box::new(Type::Integer)),
                    implementation: Some(time_lib::builtin_duration_sub),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "timer_start".to_string(),
                    parameters: vec![],
                    return_type: Type::Vault(Box::new(Type::String), Box::new(Type::Integer)),
                    implementation: Some(time_lib::builtin_timer_start),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "timer_elapsed".to_string(),
                    parameters: vec![Type::Vault(Box::new(Type::String), Box::new(Type::Integer))],
                    return_type: Type::Integer,
                    implementation: Some(time_lib::builtin_timer_elapsed),
                    c_implementation: None,
                    ast_body: None,
                },
                BuiltInFunction {
                    name: "timer_reset".to_string(),
                    parameters: vec![Type::Vault(Box::new(Type::String), Box::new(Type::Integer))],
                    return_type: Type::Vault(Box::new(Type::String), Box::new(Type::Integer)),
                    implementation: Some(time_lib::builtin_timer_reset),
                    c_implementation: None,
                    ast_body: None,
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

    /// Get a built-in function by name and parameter types (for overloading)
    pub fn get_builtin_function_by_signature(
        &self,
        name: &str,
        param_types: &[Type],
    ) -> Option<&BuiltInFunction> {
        self.functions
            .iter()
            .find(|f| f.name == name && f.parameters == param_types)
    }

    /// Check if a type name is a built-in type
    pub fn is_builtin_type(&self, name: &str) -> bool {
        self.types.iter().any(|t| t.name == name)
    }

    /// Execute a built-in function
    pub fn execute_builtin(&self, name: &str, args: &[Expr]) -> Result<Expr, String> {
        if let Some(func) = self.get_builtin_function(name) {
            if let Some(impl_fn) = func.implementation {
                (impl_fn)(args)
            } else {
                Err(format!(
                    "Built-in function '{}' has no native implementation",
                    name
                ))
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Expr, Literal};

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
        let result = builtin_len_string(&args).unwrap();

        match result {
            Expr::Literal(Literal::Integer(5)) => {}
            _ => panic!("Expected integer 5"),
        }
    }

    #[test]
    fn test_int_conversion() {
        let args = vec![Expr::Literal(Literal::String("42".to_string()))];
        let result = builtin_int(&args).unwrap();

        match result {
            Expr::Literal(Literal::Integer(42)) => {}
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
            Expr::Literal(Literal::Integer(7)),
        ];
        let result = builtin_max(&args).unwrap();
        assert!(matches!(result, Expr::Literal(Literal::Integer(7))));

        // Test min
        let args = vec![
            Expr::Literal(Literal::Integer(3)),
            Expr::Literal(Literal::Integer(7)),
        ];
        let result = builtin_min(&args).unwrap();
        assert!(matches!(result, Expr::Literal(Literal::Integer(3))));
    }
}
