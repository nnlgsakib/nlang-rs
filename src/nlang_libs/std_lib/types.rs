use crate::ast::{Expr, Type};

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
