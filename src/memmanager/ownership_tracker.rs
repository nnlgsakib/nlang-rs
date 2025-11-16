use crate::ast::Type;
use crate::semantic::SemanticError;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct VarInfo {
    pub ty: Type,
    pub is_mutable: bool,
    pub state: OwnershipState,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OwnershipState {
    Alive,
    Moved,
}

pub struct OwnershipSnapshot {
    pub entries: HashMap<String, VarInfo>,
}

pub struct OwnershipTracker {
    scopes: Vec<HashMap<String, VarInfo>>, 
}

impl OwnershipTracker {
    pub fn new() -> Self { Self { scopes: vec![HashMap::new()] } }

    pub fn begin_scope(&mut self) { self.scopes.push(HashMap::new()); }
    pub fn end_scope(&mut self) { self.scopes.pop(); }

    pub fn declare(&mut self, name: String, is_mutable: bool, ty: Type) -> Result<(), SemanticError> {
        let scope = self.scopes.last_mut().unwrap();
        if scope.contains_key(&name) { return Err(SemanticError { message: format!("Variable '{}' already declared", name) }); }
        scope.insert(name, VarInfo { ty, is_mutable, state: OwnershipState::Alive });
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<VarInfo> {
        for s in self.scopes.iter().rev() {
            if let Some(v) = s.get(name) { return Some(v.clone()); }
        }
        None
    }

    pub fn set_state(&mut self, name: &str, state: OwnershipState) {
        for s in self.scopes.iter_mut().rev() {
            if let Some(v) = s.get_mut(name) { v.state = state; return; }
        }
    }

    pub fn ensure_alive(&self, name: &str) -> Result<(), SemanticError> {
        if let Some(v) = self.get(name) {
            if v.state == OwnershipState::Moved {
                return Err(SemanticError { message: format!("Use of moved value '{}'", name) });
            }
        }
        Ok(())
    }

    pub fn snapshot(&self) -> OwnershipSnapshot {
        let mut map = HashMap::new();
        for s in &self.scopes {
            for (k, v) in s { map.insert(k.clone(), v.clone()); }
        }
        OwnershipSnapshot { entries: map }
    }

    pub fn restore(&mut self, snap: &mut OwnershipSnapshot) {
        // Restore to snapshot conservatively: overwrite current entries
        let current = self.scopes.last_mut().unwrap();
        current.clear();
        for (k, v) in snap.entries.clone() { current.insert(k, v); }
    }
}