use std::path::PathBuf;
use crate::execution_engine::ExecutionEngine;

pub fn compile(input: PathBuf, output: Option<PathBuf>) -> anyhow::Result<()> {
    println!("Compiling {}...", input.display());
    
    // Read the source code
    let source = std::fs::read_to_string(&input)?;
    
    // Create execution engine
    let engine = ExecutionEngine::new();
    
    // Get module name from file name
    let module_name = input.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");
    
    // Determine output path
    let output_path = output.unwrap_or_else(|| {
        let mut path = input.clone();
        path.set_extension("exe");
        path
    });
    
    // Compile to executable
    engine.compile_to_executable(&source, module_name, &output_path)?;
    
    println!("Compiled successfully to: {}", output_path.display());
    Ok(())
}

pub fn generate_ir(input: PathBuf, output: Option<PathBuf>) -> anyhow::Result<()> {
    println!("Generating LLVM IR for {}...", input.display());
    
    // Read the source code
    let source = std::fs::read_to_string(&input)?;
    
    // Create execution engine
    let engine = ExecutionEngine::new();
    
    // Get module name from file name
    let module_name = input.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");
    
    // Generate IR
    let ir_code = engine.compile_to_ir(&source, module_name)?;
    
    // Determine output path
    let output_path = output.unwrap_or_else(|| {
        let mut path = input.clone();
        path.set_extension("ll");
        path
    });
    
    // Write IR to file
    std::fs::write(&output_path, ir_code)?;
    
    println!("LLVM IR generated successfully: {}", output_path.display());
    Ok(())
}

pub fn generate_c(input: PathBuf, output: Option<PathBuf>) -> anyhow::Result<()> {
    println!("Generating C code for {}...", input.display());
    
    // Read the source code
    let source = std::fs::read_to_string(&input)?;
    
    // Create execution engine
    let engine = ExecutionEngine::new();
    
    // Get module name from file name
    let module_name = input.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");
    
    // Generate C code
    let c_code = engine.compile_to_c(&source, module_name)?;
    
    // Determine output path
    let output_path = output.unwrap_or_else(|| {
        let mut path = input.clone();
        path.set_extension("c");
        path
    });
    
    // Write C code to file
    std::fs::write(&output_path, c_code)?;
    
    println!("C code generated successfully: {}", output_path.display());
    Ok(())
}

pub fn run(input: PathBuf) -> anyhow::Result<()> {
    println!("Running {}...", input.display());
    
    // Read the source code
    let source = std::fs::read_to_string(&input)?;
    
    // Create execution engine
    let mut engine = ExecutionEngine::new();
    
    // Get module name from file name
    let module_name = input.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");
    
    // Execute the program with file path for proper module resolution
    match engine.execute_source_with_file_path(&source, module_name, Some(&input)) {
        Ok(exit_code) => {
            println!("Program executed successfully with exit code: {}", exit_code);
        }
        Err(e) => {
            eprintln!("Execution error: {}", e);
            return Err(e.into());
        }
    }
    
    Ok(())
}