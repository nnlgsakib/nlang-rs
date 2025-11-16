pub mod analyzer;
pub mod error;
pub mod symbol;
pub mod type_inference;

#[cfg(test)]
mod tests;

use crate::ast::Program;
use analyzer::SemanticAnalyzer;
pub use error::SemanticError;
use crate::memmanager::MemManager;

pub fn analyze(program: Program) -> Result<Program, SemanticError> {
    analyze_with_file_path(program, None)
}

pub fn analyze_with_file_path(
    program: Program,
    file_path: Option<&std::path::Path>,
) -> Result<Program, SemanticError> {
    let mut analyzer = SemanticAnalyzer::new_with_file_path(file_path);
    let analyzed = analyzer.analyze_program(program, true)?; // true indicates this is the main program
    // Run memory safety checks
    let mut mm = MemManager::new();
    mm.analyze(&analyzed)?;
    Ok(analyzed)
}
