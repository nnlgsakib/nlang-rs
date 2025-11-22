use super::error::InterpreterError;
use crate::ast::{Expr, Parameter, Statement, Type};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    Float(f64),
    Boolean(bool),
    String(String),
    Array(Vec<Value>),
    Vault(HashMap<String, Value>),
    Pool(HashSet<SimpleValue>),
    Tree(Box<TreeNode>),
    Lambda {
        parameters: Vec<Parameter>,
        body: Arc<Expr>,
        return_type: Option<Type>,
        closure: Arc<crate::interpreter::environment::Environment>,
    },
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Integer(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "{}", s),
            Value::Array(arr) => {
                write!(f, "[")?;
                for (i, item) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            Value::Vault(map) => {
                write!(f, "vault{{")?;
                let mut first = true;
                for (k, v) in map.iter() {
                    if !first {
                        write!(f, ", ")?;
                    } else {
                        first = false;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Pool(set) => {
                write!(f, "pool{{")?;
                let mut first = true;
                for v in set.iter() {
                    if !first {
                        write!(f, ", ")?;
                    } else {
                        first = false;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "}}")
            }
            Value::Tree(node) => {
                write!(f, "tree({})", node.value)
            }
            Value::Lambda { .. } => write!(f, "<lambda function>"),
        }
    }
}

use crate::nlang_libs::common::NativeFunction;

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub body: Vec<Statement>,
    pub return_type: Option<Type>,
    pub native_impl: Option<NativeFunction>,
}

impl PartialEq for Function {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.parameters == other.parameters
            && self.body == other.body
            && self.return_type == other.return_type
        // Ignore native_impl in comparison as function pointers cannot be reliably compared
    }
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Integer(_) => "int",
            Value::Float(_) => "float",
            Value::Boolean(_) => "bool",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Vault(_) => "vault",
            Value::Pool(_) => "pool",
            Value::Tree(_) => "tree",
            Value::Lambda { .. } => "lambda",
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimpleValue {
    Integer(i64),
    Float(i64),
    Boolean(bool),
    String(String),
}

impl Hash for SimpleValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            SimpleValue::Integer(i) => {
                0u8.hash(state);
                i.hash(state);
            }
            SimpleValue::Float(fscaled) => {
                1u8.hash(state);
                fscaled.hash(state);
            }
            SimpleValue::Boolean(b) => {
                2u8.hash(state);
                b.hash(state);
            }
            SimpleValue::String(s) => {
                3u8.hash(state);
                s.hash(state);
            }
        }
    }
}

impl fmt::Display for SimpleValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SimpleValue::Integer(i) => write!(f, "{}", i),
            SimpleValue::Float(fi) => write!(f, "{}", (*fi as f64) / 1_000_000.0),
            SimpleValue::Boolean(b) => write!(f, "{}", b),
            SimpleValue::String(s) => write!(f, "{}", s),
        }
    }
}

impl SimpleValue {
    pub fn from_value(v: Value) -> Result<SimpleValue, InterpreterError> {
        match v {
            Value::Integer(i) => Ok(SimpleValue::Integer(i)),
            Value::Float(f) => Ok(SimpleValue::Float((f * 1_000_000.0) as i64)),
            Value::Boolean(b) => Ok(SimpleValue::Boolean(b)),
            Value::String(s) => Ok(SimpleValue::String(s)),
            _ => Err(InterpreterError::InvalidOperation {
                message: "Pool supports int, float, bool, string".to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TreeNode {
    pub value: SimpleValue,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    pub fn new(value: SimpleValue) -> Self {
        Self {
            value,
            children: Vec::new(),
        }
    }
    pub fn add_child(&mut self, child: SimpleValue) {
        self.children.push(TreeNode::new(child));
    }
}
