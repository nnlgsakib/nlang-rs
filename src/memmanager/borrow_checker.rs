use crate::semantic::SemanticError;
use std::collections::HashMap;

#[derive(Clone, Debug)]
struct BorrowInfo {
    immut_count: usize,
    mut_active: bool,
}

pub struct BorrowSnapshot {
    entries: HashMap<String, BorrowInfo>,
}

pub struct BorrowChecker {
    scopes: Vec<HashMap<String, BorrowInfo>>, 
    expr_stack: Vec<Vec<(String, bool)>>,
}

impl BorrowChecker {
    pub fn new() -> Self { Self { scopes: vec![HashMap::new()], expr_stack: Vec::new() } }
    pub fn begin_scope(&mut self) { self.scopes.push(HashMap::new()); }
    pub fn end_scope(&mut self) -> Result<(), SemanticError> { self.scopes.pop(); Ok(()) }

    fn get_mut_info(&mut self, name: &str) -> &mut BorrowInfo {
        let current = self.scopes.last_mut().unwrap();
        current.entry(name.to_string()).or_insert(BorrowInfo { immut_count: 0, mut_active: false })
    }

    pub fn borrow_immut(&mut self, name: &str) -> Result<(), SemanticError> {
        let info = self.get_mut_info(name);
        if info.mut_active {
            return Err(SemanticError { message: format!("Cannot borrow '{}' immutably while it is mutably borrowed", name) });
        }
        info.immut_count += 1;
        Ok(())
    }

    pub fn borrow_mut(&mut self, name: &str) -> Result<(), SemanticError> {
        let info = self.get_mut_info(name);
        if info.mut_active || info.immut_count > 0 {
            return Err(SemanticError { message: format!("Cannot borrow '{}' mutably: existing borrows present", name) });
        }
        info.mut_active = true;
        Ok(())
    }

    pub fn begin_expr(&mut self) { self.expr_stack.push(Vec::new()); }
    pub fn end_expr(&mut self) {
        if let Some(records) = self.expr_stack.pop() {
            for (name, mutable) in records.iter() {
                if *mutable {
                    self.release_mut(name);
                } else {
                    self.release_immut(name);
                }
            }
        }
    }

    pub fn borrow_immut_ephemeral(&mut self, name: &str) -> Result<(), SemanticError> {
        self.borrow_immut(name)?;
        if let Some(top) = self.expr_stack.last_mut() { top.push((name.to_string(), false)); }
        Ok(())
    }
    pub fn borrow_mut_ephemeral(&mut self, name: &str) -> Result<(), SemanticError> {
        self.borrow_mut(name)?;
        if let Some(top) = self.expr_stack.last_mut() { top.push((name.to_string(), true)); }
        Ok(())
    }

    fn release_immut(&mut self, name: &str) {
        for s in self.scopes.iter_mut().rev() {
            if let Some(info) = s.get_mut(name) { if info.immut_count > 0 { info.immut_count -= 1; } return; }
        }
    }
    fn release_mut(&mut self, name: &str) {
        for s in self.scopes.iter_mut().rev() {
            if let Some(info) = s.get_mut(name) { info.mut_active = false; return; }
        }
    }

    pub fn ensure_not_mut_borrow_blocking(&self, name: &str) -> Result<(), SemanticError> {
        for s in self.scopes.iter().rev() {
            if let Some(info) = s.get(name) { if info.mut_active { return Err(SemanticError { message: format!("Cannot use '{}' while it is mutably borrowed", name) }); } }
        }
        Ok(())
    }

    pub fn ensure_not_borrowed_for_mutation(&self, name: &str) -> Result<(), SemanticError> {
        for s in self.scopes.iter().rev() {
            if let Some(info) = s.get(name) { if info.mut_active || info.immut_count > 0 { return Err(SemanticError { message: format!("Cannot mutate '{}' while it is borrowed", name) }); } }
        }
        Ok(())
    }

    pub fn snapshot(&self) -> BorrowSnapshot {
        let mut map = HashMap::new();
        for s in &self.scopes { for (k, v) in s { map.insert(k.clone(), v.clone()); } }
        BorrowSnapshot { entries: map }
    }

    pub fn restore(&mut self, snap: &mut BorrowSnapshot) {
        let current = self.scopes.last_mut().unwrap();
        current.clear();
        for (k, v) in snap.entries.clone() { current.insert(k, v); }
    }

    pub fn join_branch_states(&mut self, then_s: &BorrowSnapshot, else_s: &BorrowSnapshot) -> Result<(), SemanticError> {
        // Conservative: if either branch has active borrows, keep them active
        let mut merged: HashMap<String, BorrowInfo> = HashMap::new();
        for (k, v) in &then_s.entries { merged.insert(k.clone(), v.clone()); }
        for (k, v) in &else_s.entries {
            let entry = merged.entry(k.clone()).or_insert(BorrowInfo { immut_count: 0, mut_active: false });
            entry.immut_count = entry.immut_count.max(v.immut_count);
            entry.mut_active = entry.mut_active || v.mut_active;
        }
        let current = self.scopes.last_mut().unwrap();
        current.clear();
        for (k, v) in merged { current.insert(k, v); }
        Ok(())
    }

    pub fn add_borrow(&mut self, name: String, mutable: bool) {
        if mutable { let _ = self.borrow_mut(&name); } else { let _ = self.borrow_immut(&name); }
    }
}