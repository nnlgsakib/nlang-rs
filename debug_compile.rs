use std::path::PathBuf;
use nlang::execution_engine::ExecutionEngine;

fn main() -> anyhow::Result<()> {
    let input = PathBuf::from("nlang_test_writes/print_types.nlang");
    
    println!("Reading source file: {}", input.display());
    let source = std::fs::read_to_string(&input)?;
    
    let engine = ExecutionEngine::new();
    let module_name = "print_types";
    
    println!("Attempting to generate C code...");
    match engine.compile_to_c(&source, module_name) {
        Ok(c_code) => {
            println!("C code generation successful!");
            println!("Generated C code:");
            println!("{}", c_code);
            
            // Try to compile with GCC manually
            let temp_dir = std::env::temp_dir();
            let c_file = temp_dir.join("print_types.c");
            let exe_file = temp_dir.join("print_types.exe");
            
            std::fs::write(&c_file, c_code)?;
            println!("Wrote C code to: {}", c_file.display());
            
            let output = std::process::Command::new("gcc")
                .arg("-o")
                .arg(&exe_file)
                .arg(&c_file)
                .output()?;
                
            if output.status.success() {
                println!("GCC compilation successful!");
                println!("Executable created at: {}", exe_file.display());
            } else {
                println!("GCC compilation failed:");
                println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
                println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
            }
        }
        Err(e) => {
            println!("C code generation failed: {}", e);
        }
    }
    
    Ok(())
}