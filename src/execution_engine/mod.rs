use crate::lexer::tokenize;
use crate::parser::parse;
use crate::semantic::{analyze, analyze_with_file_path, SemanticError};
use crate::memmanager::MemManager;
use crate::interpreter::{Interpreter, InterpreterError};
use crate::c_codegen::{CCodeGenerator, CCodeGenError};
use std::path::{Path, PathBuf};
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
    gcc_path: Option<PathBuf>,
}

impl ExecutionEngine {
    pub fn new() -> Self {
        ExecutionEngine {
            interpreter: Interpreter::new(),
            gcc_path: None,
        }
    }
    
    pub fn new_with_gcc_path<P: AsRef<Path>>(path: P) -> Self {
        ExecutionEngine {
            interpreter: Interpreter::new(),
            gcc_path: Some(path.as_ref().to_path_buf()),
        }
    }
    
    pub fn set_gcc_path<P: AsRef<Path>>(&mut self, path: P) {
        self.gcc_path = Some(path.as_ref().to_path_buf());
    }
    
    /// Execute a nlang program from source code
    pub fn execute_source(&mut self, source: &str, _module_name: &str) -> Result<i32, ExecutionError> {
        self.execute_source_with_file_path(source, _module_name, None)
    }
    
    /// Execute a nlang program from source code with file path for proper module resolution
    pub fn execute_source_with_file_path(&mut self, source: &str, _module_name: &str, file_path: Option<&std::path::Path>) -> Result<i32, ExecutionError> {
        // Create interpreter with file path for module registry support
        if file_path.is_some() {
            self.interpreter = Interpreter::new_with_file_path(file_path);
        }
        
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
        {
            let mut mm = MemManager::new();
            mm.analyze(&analyzed_program)?;
        }
        
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
        let gcc_cmd = self.resolve_gcc_path();
        let mut cmd = Command::new(&gcc_cmd);
        cmd.arg("-o")
            .arg(output_path)
            .arg(&c_file);
        if !cfg!(windows) {
            cmd.arg("-lm");
        }
        let gcc_output = cmd
            .output()
            .map_err(|e| ExecutionError::NotImplemented {
                message: format!("gcc not found or cannot execute: {} ({})", gcc_cmd.display(), e),
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
    
    fn resolve_gcc_path(&self) -> PathBuf {
        if let Some(p) = &self.gcc_path {
            return p.clone();
        }
        if let Some(p) = std::env::var_os("NLANG_GCC") {
            return PathBuf::from(p);
        }
        if let Some(p) = std::env::var_os("CC") {
            return PathBuf::from(p);
        }
        PathBuf::from("gcc")
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
    
    #[test]
    fn test_custom_gcc_path_selection() {
        let engine = ExecutionEngine::new_with_gcc_path("nonexistent_gcc_binary");
        let source = r#"
            def main() {
            }
        "#;
        let output = std::env::temp_dir().join("nlang_test_bin");
        let res = engine.compile_to_executable(source, "test_module", &output);
        assert!(res.is_err());
        match res {
            Err(ExecutionError::NotImplemented { message }) => {
                assert!(message.contains("nonexistent_gcc_binary"));
            }
            _ => panic!("unexpected result"),
        }
        let _ = std::fs::remove_file(output);
    }

    #[test]
    fn codegen_emits_scope_frees_for_owned_values() {
        let engine = ExecutionEngine::new();
        let source = r#"
            import std;
            def main() {
                store msg = "Hello".upper();
                store parts = "a,b,c".split(",");
                println(parts.join("-"));
            }
        "#;
        let c = engine.compile_to_c(source, "test_module").expect("compile to C");
        assert!(c.contains("free(msg)"), "expected free(msg) in generated C, got:\n{}", c);
        assert!(c.contains("parts_len"), "expected parts_len tracking for split result, got:\n{}", c);
        assert!(c.contains("free(parts[i])"), "expected per-element free for parts, got:\n{}", c);
        assert!(c.contains("free(parts)"), "expected free(parts) after element frees, got:\n{}", c);
    }
    

}