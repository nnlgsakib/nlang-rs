use crate::lexer::tokenize;
use crate::parser::parse;
use crate::semantic::{analyze, analyze_with_file_path, SemanticError};
use crate::interpreter::{Interpreter, InterpreterError};
use crate::c_codegen::{CCodeGenerator, CCodeGenError};
use std::path::Path;
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Lexer error: {0}")]
    LexerError(#[from] crate::lexer::LexerError),
    #[error("Parser error: {0}")]
    ParserError(#[from] crate::parser::ParseError),
    #[error("Semantic error: {0}")]
    SemanticError(#[from] SemanticError),
    #[error("Interpreter error: {0}")]
    InterpreterError(#[from] InterpreterError),

    #[error("C CodeGen error: {0}")]
    CCodeGenError(#[from] CCodeGenError),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Feature not implemented: {message}")]
    NotImplemented { message: String },
}

pub struct ExecutionEngine {
    interpreter: Interpreter,
}

impl ExecutionEngine {
    pub fn new() -> Self {
        ExecutionEngine {
            interpreter: Interpreter::new(),
        }
    }
    
    /// Execute a nlang program from source code
    pub fn execute_source(&mut self, source: &str, _module_name: &str) -> Result<i32, ExecutionError> {
        self.execute_source_with_file_path(source, _module_name, None)
    }
    
    /// Execute a nlang program from source code with file path for proper module resolution
    pub fn execute_source_with_file_path(&mut self, source: &str, _module_name: &str, file_path: Option<&std::path::Path>) -> Result<i32, ExecutionError> {
        // Tokenize
        let tokens = tokenize(source)?;
        
        // Parse
        let program = parse(&tokens)?;
        
        // Semantic analysis with file path for proper module resolution
        let analyzed_program = if let Some(path) = file_path {
            analyze_with_file_path(program, Some(path))?
        } else {
            analyze(program)?
        };
        
        // Execute with interpreter
        let result = if let Some(path) = file_path {
            self.interpreter.execute_program_with_path(&analyzed_program, Some(path.to_str().unwrap()))?
        } else {
            self.interpreter.execute_program(&analyzed_program)?
        };
        Ok(result)
    }
    
    /// Compile a nlang program to an executable binary
    pub fn compile_to_executable(
        &self,
        source: &str,
        module_name: &str,
        output_path: &Path,
    ) -> Result<(), ExecutionError> {
        // Use GCC compilation (compile to C first)
        self.compile_with_gcc(source, module_name, output_path)
    }
    
    /// Compile a nlang program to an executable binary with file path for proper module resolution
    pub fn compile_to_executable_with_file_path(
        &self,
        source: &str,
        module_name: &str,
        output_path: &Path,
        file_path: Option<&std::path::Path>,
    ) -> Result<(), ExecutionError> {
        // Use GCC compilation (compile to C first)
        self.compile_with_gcc_with_file_path(source, module_name, output_path, file_path)
    }
    

    
    /// Compile using GCC (generate C code first)
    fn compile_with_gcc(
        &self,
        source: &str,
        module_name: &str,
        output_path: &Path,
    ) -> Result<(), ExecutionError> {
        self.compile_with_gcc_with_file_path(source, module_name, output_path, None)
    }
    
    /// Compile using GCC with file path for proper module resolution
    fn compile_with_gcc_with_file_path(
        &self,
        source: &str,
        module_name: &str,
        output_path: &Path,
        file_path: Option<&std::path::Path>,
    ) -> Result<(), ExecutionError> {
        // Generate C code instead of LLVM IR
        let c_code = self.compile_to_c_with_file_path(source, module_name, file_path)?;
        
        // Create temporary C file
        let temp_dir = std::env::temp_dir();
        let c_file = temp_dir.join(format!("{}.c", module_name));
        std::fs::write(&c_file, c_code)?;
        
        // Compile C to executable using GCC
        let mut cmd = Command::new("gcc");
        cmd.arg("-o")
            .arg(output_path)
            .arg(&c_file);
        if !cfg!(windows) {
            cmd.arg("-lm");
        }
        let gcc_output = cmd
            .output()
            .map_err(|_| ExecutionError::NotImplemented {
                message: "gcc not found".to_string(),
            })?;
            
        if !gcc_output.status.success() {
            let stderr = String::from_utf8_lossy(&gcc_output.stderr);
            let _ = std::fs::remove_file(&c_file);
            return Err(ExecutionError::NotImplemented {
                message: format!("gcc compilation failed: {}", stderr),
            });
        }
        
        // Clean up temporary file
        let _ = std::fs::remove_file(&c_file);
        Ok(())
    }
    
    /// Generate C code representation (fallback for GCC compilation)
    pub fn compile_to_c(&self, source: &str, _module_name: &str) -> Result<String, ExecutionError> {
        self.compile_to_c_with_file_path(source, _module_name, None)
    }
    
    /// Generate C code representation with file path for proper module resolution
    pub fn compile_to_c_with_file_path(&self, source: &str, _module_name: &str, file_path: Option<&std::path::Path>) -> Result<String, ExecutionError> {
        // Tokenize
        let tokens = tokenize(source)?;
        
        // Parse
        let program = parse(&tokens)?;
        
        // Semantic analysis with file path for proper module resolution
        let analyzed_program = if let Some(path) = file_path {
            analyze_with_file_path(program, Some(path))?
        } else {
            analyze(program)?
        };
        
        let c_generator = CCodeGenerator::new();
        Ok(c_generator.generate_program(&analyzed_program)?)
    }


}

impl Default for ExecutionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_execution() {
        let mut engine = ExecutionEngine::new();
        let source = r#"
            def main() {
                store x = 5;
                store y = 10;
                store result = x + y;
            }
        "#;
        
        let result = engine.execute_source(source, "test_module");
        assert!(result.is_ok());
        // Main function returns 0 by default when no explicit return
        assert_eq!(result.unwrap(), 0);
    }
    

}