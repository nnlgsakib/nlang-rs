use crate::ast::*;
use crate::semantic::SemanticError;
use std::collections::HashSet;

pub struct LifetimeAnalyzer {
    in_function: bool,
    function_return: Option<Type>,
    scopes: Vec<HashSet<String>>,         // locals per lexical scope
    params: HashSet<String>,              // parameters in current function
    borrow_assignments: Vec<(String, String, usize, usize)>, // (target, source, target_depth, source_depth)
}

impl LifetimeAnalyzer {
    pub fn new() -> Self {
        Self { in_function: false, function_return: None, scopes: Vec::new(), params: HashSet::new(), borrow_assignments: Vec::new() }
    }
    pub fn begin_program(&mut self) { self.scopes.clear(); self.params.clear(); self.borrow_assignments.clear(); }
    pub fn end_program(&mut self) {}
    pub fn begin_scope(&mut self) { self.scopes.push(HashSet::new()); }
    pub fn end_scope(&mut self) { self.scopes.pop(); }
    pub fn declare(&mut self, name: String) { if let Some(s) = self.scopes.last_mut() { s.insert(name); } }
    pub fn declare_param(&mut self, name: String) { self.params.insert(name); }

    pub fn begin_function(&mut self, ret: Option<Type>) { self.in_function = true; self.function_return = ret; self.scopes.clear(); self.params.clear(); self.borrow_assignments.clear(); self.begin_scope(); }
    pub fn end_function(&mut self) -> Result<(), SemanticError> { self.in_function = false; self.function_return = None; self.scopes.clear(); self.params.clear(); self.borrow_assignments.clear(); Ok(()) }

    fn name_scope_depth(&self, name: &str) -> Option<usize> {
        for (idx, scope) in self.scopes.iter().enumerate() {
            if scope.contains(name) { return Some(idx); }
        }
        None
    }

    pub fn record_borrow_assignment(&mut self, target: &str, source: &str) -> Result<(), SemanticError> {
        let t_depth = self.name_scope_depth(target).unwrap_or(0);
        let s_depth = self.name_scope_depth(source).unwrap_or(0);
        // If assigning a reference to a source declared in a deeper (inner) scope into a target declared in a shallower (outer) scope, reject.
        if s_depth > t_depth {
            return Err(SemanticError { message: format!("Cannot store reference to local '{}' in outer variable '{}'", source, target) });
        }
        self.borrow_assignments.push((target.to_string(), source.to_string(), t_depth, s_depth));
        Ok(())
    }

    pub fn validate_return(&self, value: &Option<Box<Expr>>) -> Result<(), SemanticError> {
        if !self.in_function { return Ok(()); }
        if let Some(ret_t) = &self.function_return {
            match ret_t {
                Type::Ref(_inner) | Type::RefMut(_inner) => {
                    if let Some(expr) = value {
                        match expr.as_ref() {
                            Expr::Borrow { target, .. } => {
                                if let Expr::Variable(name) = target.as_ref() {
                                    if !self.params.contains(name) {
                                        return Err(SemanticError { message: format!("Cannot return reference to local variable '{}'", name) });
                                    }
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