use crate::ast::*;
use crate::semantic::SemanticError;
use super::ownership_tracker::{OwnershipTracker, OwnershipSnapshot, OwnershipState};

fn is_copy_type(ty: &Type) -> bool {
    matches!(ty,
        Type::Integer | Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::ISize |
        Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::USize |
        Type::F32 | Type::F64 | Type::Boolean
    )
}

pub struct MoveAnalyzer {}

impl MoveAnalyzer {
    pub fn new() -> Self { Self {} }

    pub fn ensure_not_moved(&self, name: &str, ownership: &OwnershipTracker) -> Result<(), SemanticError> {
        ownership.ensure_alive(name)
    }

    pub fn handle_initial_assignment(&self, _target: &str, expr: &Expr, ty: &Type, ownership: &mut OwnershipTracker) -> Result<(), SemanticError> {
        if let Expr::Variable(src_name) = expr { if !is_copy_type(ty) { ownership.set_state(src_name, OwnershipState::Moved); } }
        Ok(())
    }

    pub fn ensure_assign_allowed(&self, name: &str, ownership: &mut OwnershipTracker) -> Result<(), SemanticError> {
        if let Some(info) = ownership.get(name) {
            if !info.is_mutable {
                return Err(SemanticError { message: format!("Cannot assign to immutable variable '{}'", name) });
            }
        }
        Ok(())
    }

    pub fn handle_assignment(&self, target: &str, value: &Expr, ownership: &mut OwnershipTracker) -> Result<(), SemanticError> {
        if let Some(_tinfo) = ownership.get(target) {
            match value {
                Expr::Variable(src_name) => {
                    if let Some(sinfo) = ownership.get(src_name) {
                        if !is_copy_type(&sinfo.ty) {
                            ownership.set_state(src_name, OwnershipState::Moved);
                        }
                    }
                }
                _ => {}
            }
            // After assignment, target remains alive
            ownership.set_state(target, OwnershipState::Alive);
        }
        Ok(())
    }

    pub fn join_branch_states(&self, then_s: &OwnershipSnapshot, else_s: &OwnershipSnapshot, ownership: &mut OwnershipTracker) -> Result<(), SemanticError> {
        // If a variable is moved in any branch, treat it as moved afterwards
        let mut moved_vars: Vec<String> = Vec::new();
        for (k, v) in &then_s.entries { if v.state == OwnershipState::Moved { moved_vars.push(k.clone()); } }
        for (k, v) in &else_s.entries { if v.state == OwnershipState::Moved { if !moved_vars.contains(k) { moved_vars.push(k.clone()); } } }
        for name in moved_vars { ownership.set_state(&name, OwnershipState::Moved); }
        Ok(())
    }

    pub fn validate_loop_consistency(&self, before: &OwnershipSnapshot, after: &OwnershipSnapshot) -> Result<(), SemanticError> {
        // If a variable is moved inside loop body, it must be moved on every iteration path; conservative error if changed
        for (k, v_before) in &before.entries {
            if let Some(v_after) = after.entries.get(k) {
                if v_before.state != v_after.state {
                    return Err(SemanticError { message: format!("Conditional move of '{}' inside loop is not allowed", k) });
                }
            }
        }
        Ok(())
    }
}