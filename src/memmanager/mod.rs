pub mod ownership_tracker;
pub mod borrow_checker;
pub mod lifetime_analyzer;
pub mod drop_analyzer;
pub mod move_analyzer;

use crate::ast::*;
use crate::semantic::SemanticError;
use ownership_tracker::OwnershipTracker;
use borrow_checker::BorrowChecker;
use lifetime_analyzer::LifetimeAnalyzer;
use move_analyzer::MoveAnalyzer;
use drop_analyzer::DropAnalyzer;

pub struct MemManager {
    ownership: OwnershipTracker,
    borrows: BorrowChecker,
    lifetimes: LifetimeAnalyzer,
    moves: MoveAnalyzer,
    drops: DropAnalyzer,
}

impl MemManager {
    pub fn new() -> Self {
        Self {
            ownership: OwnershipTracker::new(),
            borrows: BorrowChecker::new(),
            lifetimes: LifetimeAnalyzer::new(),
            moves: MoveAnalyzer::new(),
            drops: DropAnalyzer::new(),
        }
    }

    pub fn analyze(&mut self, program: &Program) -> Result<(), SemanticError> {
        self.lifetimes.begin_program();

        for stmt in &program.statements {
            self.analyze_statement(stmt)?;
        }

        self.lifetimes.end_program();
        Ok(())
    }

    fn analyze_statement(&mut self, stmt: &Statement) -> Result<(), SemanticError> {
        match stmt {
            Statement::LetDeclaration { name, initializer, var_type, is_mutable, .. } => {
                let ty = var_type.clone().unwrap_or(Type::Unknown);
                self.ownership.declare(name.clone(), *is_mutable, ty.clone())?;
                self.lifetimes.declare(name.clone());
                if let Some(init) = initializer {
                    self.analyze_expr(init)?;
                    // Moves on initialization
                    self.moves.handle_initial_assignment(name, init, &ty, &mut self.ownership)?;
                    if let Expr::Borrow { target, mutable } = init {
                        if let Expr::Variable(src) = target.as_ref() {
                            if *mutable { let _ = self.borrows.borrow_mut(src); } else { let _ = self.borrows.borrow_immut(src); }
                            self.lifetimes.record_borrow_assignment(name, src)?;
                        }
                    }
                }
            }
            Statement::Block { statements } => {
                self.ownership.begin_scope();
                self.borrows.begin_scope();
                self.lifetimes.begin_scope();
                for s in statements { self.analyze_statement(s)?; }
                self.lifetimes.end_scope();
                self.borrows.end_scope()?;
                self.ownership.end_scope();
                self.drops.on_scope_end()?;
            }
            Statement::FunctionDeclaration { name: _, parameters, body, return_type, .. } => {
                // Function-level scope
                self.ownership.begin_scope();
                self.borrows.begin_scope();
                self.lifetimes.begin_function(return_type.clone());
                for p in parameters {
                    let ty = p.param_type.clone().unwrap_or(Type::Unknown);
                    self.ownership.declare(p.name.clone(), false, ty)?;
                    self.lifetimes.declare_param(p.name.clone());
                }
                for s in body { self.analyze_statement(s)?; }
                self.lifetimes.end_function()?;
                self.borrows.end_scope()?;
                self.ownership.end_scope();
                self.drops.on_function_end()?;
            }
            Statement::If { condition, then_branch, else_branch } => {
                self.analyze_expr(condition)?;
                // Branch analysis: analyze both and join
                let mut left = self.ownership.snapshot();
                let mut bor_left = self.borrows.snapshot();

                self.analyze_statement(then_branch)?;
                let then_own = self.ownership.snapshot();
                let then_bor = self.borrows.snapshot();

                // Restore and analyze else
                self.ownership.restore(&mut left);
                self.borrows.restore(&mut bor_left);
                if let Some(else_b) = else_branch {
                    self.analyze_statement(else_b)?;
                }
                let else_own = self.ownership.snapshot();
                let else_bor = self.borrows.snapshot();

                self.moves.join_branch_states(&then_own, &else_own, &mut self.ownership)?;
                self.borrows.join_branch_states(&then_bor, &else_bor)?;
            }
            Statement::While { condition, body } => {
                self.analyze_expr(condition)?;
                // Conservative: analyze body once and require consistent moves
                let mut before = self.ownership.snapshot();
                let mut bor_before = self.borrows.snapshot();
                self.analyze_statement(body)?;
                let after = self.ownership.snapshot();
                self.moves.validate_loop_consistency(&before, &after)?;
                self.borrows.restore(&mut bor_before);
                self.ownership.restore(&mut before);
            }
            Statement::Loop { body } => {
                // Analyze loop body once
                let mut before = self.ownership.snapshot();
                let mut bor_before = self.borrows.snapshot();
                self.analyze_statement(body)?;
                let after = self.ownership.snapshot();
                self.moves.validate_loop_consistency(&before, &after)?;
                self.borrows.restore(&mut bor_before);
                self.ownership.restore(&mut before);
            }
            Statement::For { initializer, condition, increment, body } => {
                if let Some(init) = initializer { self.analyze_statement(init)?; }
                if let Some(cond) = condition { self.analyze_expr(cond)?; }
                let mut before = self.ownership.snapshot();
                let mut bor_before = self.borrows.snapshot();
                self.analyze_statement(body)?;
                if let Some(inc) = increment { self.analyze_expr(inc)?; }
                let after = self.ownership.snapshot();
                self.moves.validate_loop_consistency(&before, &after)?;
                self.borrows.restore(&mut bor_before);
                self.ownership.restore(&mut before);
            }
            Statement::Return { value } => {
                if let Some(val) = value { self.analyze_expr(val)?; }
                self.lifetimes.validate_return(value)?;
            }
            Statement::Expression(expr) => {
                self.analyze_expr(expr)?;
            }
            Statement::Break | Statement::Continue | Statement::AssignMain { .. } | Statement::Import { .. } | Statement::ImportFrom { .. } | Statement::Pick { .. } | Statement::RepeatUntil { .. } => {
                // For now, treat these as non-memory-affecting; extend as needed
            }
        }
        Ok(())
    }

    fn analyze_expr(&mut self, expr: &Expr) -> Result<(), SemanticError> {
        match expr {
            Expr::Variable(name) => {
                self.moves.ensure_not_moved(name, &self.ownership)?;
                self.borrows.ensure_not_mut_borrow_blocking(name)?;
                self.lifetimes.track_use(name);
            }
            Expr::Borrow { target, mutable } => {
                // Resolve variable name
                if let Expr::Variable(name) = target.as_ref() {
                    if *mutable { self.borrows.borrow_mut_ephemeral(name)?; } else { self.borrows.borrow_immut_ephemeral(name)?; }
                    self.lifetimes.add_borrow(name.clone(), *mutable);
                }
            }
            Expr::Assign { name, value } => {
                self.moves.ensure_assign_allowed(name, &mut self.ownership)?;
                // Analyze RHS first
                self.analyze_expr(value)?;
                self.moves.handle_assignment(name, value, &mut self.ownership)?;
                self.borrows.ensure_not_borrowed_for_mutation(name)?;
                // Lifetime: if assigning a borrow into a variable, validate scopes
                if let Expr::Borrow { target, .. } = value.as_ref() {
                    if let Expr::Variable(src) = target.as_ref() {
                        // Persist borrow for assignment
                        if let Expr::Borrow { mutable, .. } = value.as_ref() { if *mutable { let _ = self.borrows.borrow_mut(src); } else { let _ = self.borrows.borrow_immut(src); } }
                        self.lifetimes.record_borrow_assignment(name, src)?;
                    }
                }
            }
            Expr::AssignIndex { sequence, value, .. } => {
                if let Expr::Variable(name) = sequence.as_ref() {
                    self.borrows.ensure_not_borrowed_for_mutation(name)?;
                    self.moves.ensure_not_moved(name, &self.ownership)?;
                }
                self.analyze_expr(value)?;
            }
            Expr::Binary { left, right, .. } => { self.borrows.begin_expr(); self.analyze_expr(left)?; self.borrows.end_expr(); self.borrows.begin_expr(); self.analyze_expr(right)?; self.borrows.end_expr(); }
            Expr::Unary { operand, .. } => { self.borrows.begin_expr(); self.analyze_expr(operand)?; self.borrows.end_expr(); }
            Expr::Call { callee, arguments } => {
                self.borrows.begin_expr(); self.analyze_expr(callee)?; self.borrows.end_expr();
                for a in arguments { self.borrows.begin_expr(); self.analyze_expr(a)?; self.borrows.end_expr(); }
            }
            Expr::Get { object, .. } => { self.borrows.begin_expr(); self.analyze_expr(object)?; self.borrows.end_expr(); }
            Expr::Index { sequence, index } => { self.borrows.begin_expr(); self.analyze_expr(sequence)?; self.borrows.end_expr(); self.borrows.begin_expr(); self.analyze_expr(index)?; self.borrows.end_expr(); }
            Expr::ArrayLiteral { elements } => { for e in elements { self.borrows.begin_expr(); self.analyze_expr(e)?; self.borrows.end_expr(); } }
            Expr::Tuple { elements } => { for e in elements { self.borrows.begin_expr(); self.analyze_expr(e)?; self.borrows.end_expr(); } }
            Expr::IfExpression { condition, then_branch, else_branch } => {
                self.borrows.begin_expr(); self.analyze_expr(condition)?; self.borrows.end_expr();
                let mut before = self.ownership.snapshot();
                let mut bor_before = self.borrows.snapshot();
                self.analyze_expr(then_branch)?;
                let then_own = self.ownership.snapshot();
                let then_bor = self.borrows.snapshot();
                self.ownership.restore(&mut before);
                self.borrows.restore(&mut bor_before);
                self.analyze_expr(else_branch)?;
                let else_own = self.ownership.snapshot();
                let else_bor = self.borrows.snapshot();
                self.moves.join_branch_states(&then_own, &else_own, &mut self.ownership)?;
                self.borrows.join_branch_states(&then_bor, &else_bor)?;
            }
            Expr::VaultLiteral { entries } => { for (_, v) in entries { self.analyze_expr(v)?; } }
            Expr::PoolLiteral { elements } => { for e in elements { self.analyze_expr(e)?; } }
            Expr::TreeLiteral { root, children } => { self.analyze_expr(root)?; for c in children { self.analyze_expr(c)?; } }
            Expr::Match { expression, cases } => { self.analyze_expr(expression)?; for c in cases { self.analyze_expr(&c.body)?; } }
            Expr::Literal(_) => {}
            Expr::Function { .. } => {}
            Expr::Set { object, value, .. } => { self.analyze_expr(object)?; self.analyze_expr(value)?; }
        }
        Ok(())
    }
}