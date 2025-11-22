use super::value::Value;
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
    #[error("Index out of bounds: index {index}, length {length}")]
    IndexOutOfBounds { index: i64, length: usize },
    #[error("Runtime error: {message}")]
    RuntimeError { message: String },
}
