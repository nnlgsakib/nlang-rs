use crate::lexer::tokenize;
use crate::parser::parse;
use crate::semantic::{analyze, analyze_with_file_path, SemanticError};
use crate::interpreter::{Interpreter, InterpreterError};
use crate::llvm_codegen::{LLVMCodeGenerator, LLVMCodeGenError};
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
    #[error("LLVM CodeGen error: {0}")]
    LLVMCodeGenError(#[from] LLVMCodeGenError),
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
        // Generate LLVM IR
        let ir_code = self.compile_to_ir(source, module_name)?;
        
        // Create temporary IR file
        let temp_dir = std::env::temp_dir();
        let ir_file = temp_dir.join(format!("{}.ll", module_name));
        std::fs::write(&ir_file, ir_code)?;
        
        let mut errors = Vec::new();
        
        // Try LLVM tools first (llc + lld-link)
        match self.compile_with_llvm_tools(&ir_file, output_path, module_name) {
            Ok(()) => {
                let _ = std::fs::remove_file(&ir_file);
                return Ok(());
            }
            Err(e) => {
                errors.push(format!("LLVM tools: {}", e));
            }
        }
        
        // Try clang as fallback
        match self.compile_with_clang_from_ir(&ir_file, output_path) {
            Ok(()) => {
                let _ = std::fs::remove_file(&ir_file);
                return Ok(());
            }
            Err(e) => {
                errors.push(format!("Clang (from IR): {}", e));
            }
        }
        
        // Try GCC as final fallback (compile to C first)
        match self.compile_with_gcc(source, module_name, output_path) {
            Ok(()) => {
                let _ = std::fs::remove_file(&ir_file);
                return Ok(());
            }
            Err(e) => {
                errors.push(format!("GCC (from C): {}", e));
            }
        }
        
        // Clean up and return detailed error
        let _ = std::fs::remove_file(&ir_file);
        Err(ExecutionError::NotImplemented {
            message: format!("No suitable compiler found. Attempted compilation methods failed:\n{}", errors.join("\n")),
        })
    }
    
    /// Try compilation with LLVM tools (llc + lld-link)
    fn compile_with_llvm_tools(
        &self,
        ir_file: &Path,
        output_path: &Path,
        module_name: &str,
    ) -> Result<(), ExecutionError> {
        let temp_dir = std::env::temp_dir();
        let obj_file = temp_dir.join(format!("{}.obj", module_name));
        
        // Compile IR to object file using llc
        let llc_output = Command::new("llc")
            .arg("-filetype=obj")
            .arg("-o")
            .arg(&obj_file)
            .arg(ir_file)
            .output()
            .map_err(|_| ExecutionError::NotImplemented {
                message: "llc not found".to_string(),
            })?;
            
        if !llc_output.status.success() {
            let stderr = String::from_utf8_lossy(&llc_output.stderr);
            return Err(ExecutionError::NotImplemented {
                message: format!("llc compilation failed: {}", stderr),
            });
        }
        
        // Link object file to executable
        let link_output = if cfg!(windows) {
            Command::new("lld-link")
                .arg("/entry:main")
                .arg("/subsystem:console")
                .arg(format!("/out:{}", output_path.display()))
                .arg(&obj_file)
                .arg("msvcrt.lib")
                .arg("legacy_stdio_definitions.lib")
                .output()
        } else {
            Command::new("ld")
                .arg("-o")
                .arg(output_path)
                .arg(&obj_file)
                .arg("-lc")
                .output()
        };
        
        match link_output {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(ExecutionError::NotImplemented {
                        message: format!("Linking failed: {}", stderr),
                    });
                }
            }
            Err(_) => {
                return Err(ExecutionError::NotImplemented {
                    message: "Linker not found".to_string(),
                });
            }
        }
        
        // Clean up object file
        let _ = std::fs::remove_file(&obj_file);
        Ok(())
    }
    
    /// Compile using clang from IR file
    fn compile_with_clang_from_ir(
        &self,
        ir_file: &Path,
        output_path: &Path,
    ) -> Result<(), ExecutionError> {
        let clang_output = Command::new("clang")
            .arg("-o")
            .arg(output_path)
            .arg(ir_file)
            .output()
            .map_err(|_| ExecutionError::NotImplemented {
                message: "clang not found".to_string(),
            })?;
            
        if !clang_output.status.success() {
            let stderr = String::from_utf8_lossy(&clang_output.stderr);
            return Err(ExecutionError::NotImplemented {
                message: format!("clang compilation failed: {}", stderr),
            });
        }
        
        Ok(())
    }
    
    /// Compile using GCC (generate C code first)
    fn compile_with_gcc(
        &self,
        source: &str,
        module_name: &str,
        output_path: &Path,
    ) -> Result<(), ExecutionError> {
        // Generate C code instead of LLVM IR
        let c_code = self.compile_to_c(source, module_name)?;
        
        // Create temporary C file
        let temp_dir = std::env::temp_dir();
        let c_file = temp_dir.join(format!("{}.c", module_name));
        std::fs::write(&c_file, c_code)?;
        
        // Compile C to executable using GCC
        let gcc_output = Command::new("gcc")
            .arg("-o")
            .arg(output_path)
            .arg(&c_file)
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
    pub fn compile_to_c(&self, source: &str, module_name: &str) -> Result<String, ExecutionError> {
        // Tokenize
        let tokens = tokenize(source)?;
        
        // Parse
        let program = parse(&tokens)?;
        
        // Semantic analysis
        let analyzed_program = analyze(program)?;
        
        // Generate C code
        let mut c_generator = CCodeGenerator::new(module_name.to_string());
        Ok(c_generator.generate_program(&analyzed_program)?)
    }

    /// Generate LLVM IR representation
    pub fn compile_to_ir(&self, source: &str, module_name: &str) -> Result<String, ExecutionError> {
        // Tokenize
        let tokens = tokenize(source)?;
        
        // Parse
        let program = parse(&tokens)?;
        
        // Semantic analysis
        let analyzed_program = analyze(program)?;
        
        // Generate actual LLVM IR
        let mut llvm_gen = LLVMCodeGenerator::new(module_name);
        let ir = llvm_gen.generate_program(&analyzed_program)?;
        
        Ok(ir)
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
    fn test_ir_generation() {
        let engine = ExecutionEngine::new();
        let source = r#"
            def add(x: int, y: int): int {
                return x + y;
            }
            
            def main() {
                store result = add(5, 3);
            }
        "#;
        
        let ir = engine.compile_to_ir(source, "test_module");
        assert!(ir.is_ok());
        let ir_code = ir.unwrap();
        // Check for LLVM IR content instead of "Pseudo-IR"
        assert!(ir_code.contains("ModuleID"));
        assert!(ir_code.contains("define"));
    }
}