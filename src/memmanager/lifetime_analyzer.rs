use crate::ast::*;
use crate::semantic::SemanticError;

pub struct LifetimeAnalyzer {
    in_function: bool,
    function_return: Option<Type>,
}

impl LifetimeAnalyzer {
    pub fn new() -> Self { Self { in_function: false, function_return: None } }
    pub fn begin_program(&mut self) {}
    pub fn end_program(&mut self) {}
    pub fn begin_scope(&mut self) {}
    pub fn end_scope(&mut self) {}
    pub fn declare(&mut self, _name: String) {}
    pub fn declare_param(&mut self, _name: String) {}

    pub fn begin_function(&mut self, ret: Option<Type>) { self.in_function = true; self.function_return = ret; }
    pub fn end_function(&mut self) -> Result<(), SemanticError> { self.in_function = false; self.function_return = None; Ok(()) }

    pub fn validate_return(&self, value: &Option<Box<Expr>>) -> Result<(), SemanticError> {
        if !self.in_function { return Ok(()); }
        if let Some(ret_t) = &self.function_return {
            match ret_t {
                Type::Ref(_inner) | Type::RefMut(_inner) => {
                    if let Some(expr) = value {
                        // Simple rule: disallow returning reference to a local variable created within function body.
                        match expr.as_ref() {
                            Expr::Borrow { target, .. } => {
                                if let Expr::Variable(name) = target.as_ref() {
                                    // For now, assume variables not in parameters are locals; enforcing is handled by Semantic layer.
                                    // Here we signal an error to prevent returning reference to locals.
                                    return Err(SemanticError { message: format!("Cannot return reference to local variable '{}'", name) });
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn track_use(&mut self, _name: &str) {}
    pub fn add_borrow(&mut self, _name: String, _mutable: bool) {}
}