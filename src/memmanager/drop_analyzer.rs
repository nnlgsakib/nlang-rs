use crate::semantic::SemanticError;

pub struct DropAnalyzer {}

impl DropAnalyzer {
    pub fn new() -> Self { Self {} }
    pub fn on_scope_end(&mut self) -> Result<(), SemanticError> { Ok(()) }
    pub fn on_function_end(&mut self) -> Result<(), SemanticError> { Ok(()) }
}