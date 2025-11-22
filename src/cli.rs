use crate::diagnostics;
use crate::execution_engine::ExecutionEngine;
use crate::lexer::tokenize;
use crate::parser::parse;
use anyhow::bail;
use serde_json;
use std::io::Write;
use std::path::PathBuf;

/// Validates that the input file has a .nlang extension.
fn validate_nlang_file(input: &PathBuf) -> anyhow::Result<()> {
    if input.extension().map_or(false, |ext| ext == "nlang") {
        Ok(())
    } else {
        bail!(
            "Input file must have a .nlang extension, but got: {}",
            input.display()
        );
    }
}

pub fn compile(
    input: PathBuf,
    output: Option<PathBuf>,
    generate_lex: bool,
    generate_ast: bool,
) -> anyhow::Result<()> {
    validate_nlang_file(&input)?;
    println!("Compiling {}...", input.display());

    // Read the source code
    let source = std::fs::read_to_string(&input)?;

    // Handle lexer output if requested
    if generate_lex {
        let lex_output_path = output.as_ref().map(|p| {
            let mut path = p.clone();
            path.set_extension("lex.json");
            path
        });
        lex(input.clone(), lex_output_path)?;
    }

    // Handle AST output if requested
    if generate_ast {
        let ast_output_path = output.as_ref().map(|p| {
            let mut path = p.clone();
            path.set_extension("ast.json");
            path
        });
        gen_ast(input.clone(), ast_output_path)?;
    }

    // Create execution engine
    let engine = ExecutionEngine::new();

    // Get module name from file name
    let module_name = input.file_stem().and_then(|s| s.to_str()).unwrap_or("main");

    let output_path = output.unwrap_or_else(|| {
        let mut path = input.clone();
        if cfg!(windows) {
            path.set_extension("exe");
        } else {
            path.set_extension("");
        }
        path
    });

    // Compile to executable with file path for proper module resolution
    if let Err(e) = engine.compile_to_executable_with_file_path(
        &source,
        module_name,
        &output_path,
        Some(&input),
    ) {
        let diag = diagnostics::from_execution_error(&input, &source, &e);
        return Err(anyhow::anyhow!(diag));
    }

    println!("Compiled successfully to: {}", output_path.display());
    Ok(())
}

pub fn version() -> anyhow::Result<()> {
    println!("nlang compiler version {}", env!("CARGO_PKG_VERSION"));
    println!("A new programming language with Python-like syntax compiled to machine code using C");
    Ok(())
}

pub fn generate_c(
    input: PathBuf,
    output: Option<PathBuf>,
    generate_lex: bool,
    generate_ast: bool,
) -> anyhow::Result<()> {
    validate_nlang_file(&input)?;
    println!("Generating C code for {}...", input.display());

    // Read the source code
    let source = std::fs::read_to_string(&input)?;

    // Handle lexer output if requested
    if generate_lex {
        let lex_output_path = output.as_ref().map(|p| {
            let mut path = p.clone();
            path.set_extension("lex.json");
            path
        });
        lex(input.clone(), lex_output_path)?;
    }

    // Handle AST output if requested
    if generate_ast {
        let ast_output_path = output.as_ref().map(|p| {
            let mut path = p.clone();
            path.set_extension("ast.json");
            path
        });
        gen_ast(input.clone(), ast_output_path)?;
    }

    // Create execution engine
    let engine = ExecutionEngine::new();

    // Get module name from file name
    let module_name = input.file_stem().and_then(|s| s.to_str()).unwrap_or("main");

    // Generate C code with file path for proper module resolution
    let c_code = engine.compile_to_c_with_file_path(&source, module_name, Some(&input))?;

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

pub fn lex(input: PathBuf, output: Option<PathBuf>) -> anyhow::Result<()> {
    validate_nlang_file(&input)?;
    println!("Generating lexer tokens for {}...", input.display());

    // Read the source code
    let source = std::fs::read_to_string(&input)?;

    // Tokenize
    let tokens = tokenize(&source)?;

    // Convert tokens to JSON-serializable format
    let token_data: Vec<serde_json::Value> = tokens
        .iter()
        .map(|token| {
            serde_json::json!({
                "type": format!("{:?}", token.token_type),
                "lexeme": token.lexeme,
                "line": token.line
            })
        })
        .collect();

    // Determine output path
    let output_path = output.unwrap_or_else(|| {
        let mut path = input.clone();
        path.set_extension("nlang.lex.json");
        path
    });

    // Write JSON to file
    let json = serde_json::to_string_pretty(&token_data)?;
    std::fs::write(&output_path, json)?;

    println!(
        "Lexer tokens generated successfully: {}",
        output_path.display()
    );
    Ok(())
}

pub fn gen_ast(input: PathBuf, output: Option<PathBuf>) -> anyhow::Result<()> {
    validate_nlang_file(&input)?;
    println!("Generating AST for {}...", input.display());

    // Read the source code
    let source = std::fs::read_to_string(&input)?;

    // Tokenize and parse
    let tokens = tokenize(&source)?;
    let program = parse(&tokens)?;

    // Convert AST to JSON
    let json = serde_json::to_string_pretty(&program)?;

    // Determine output path
    let output_path = output.unwrap_or_else(|| {
        let mut path = input.clone();
        path.set_extension("nlang.ast.json");
        path
    });

    // Write JSON to file
    std::fs::write(&output_path, json)?;

    println!("AST generated successfully: {}", output_path.display());
    Ok(())
}

pub fn run(input: PathBuf) -> anyhow::Result<()> {
    validate_nlang_file(&input)?;
    println!("Running {}...", input.display());

    // Read the source code
    let source = std::fs::read_to_string(&input)?;

    // Create execution engine
    let mut engine = ExecutionEngine::new();

    // Get module name from file name
    let module_name = input.file_stem().and_then(|s| s.to_str()).unwrap_or("main");

    // Execute the program with file path for proper module resolution
    match engine.execute_source_with_file_path(&source, module_name, Some(&input)) {
        Ok(exit_code) => {
            println!(
                "Program executed successfully with exit code: {}",
                exit_code
            );
        }
        Err(e) => {
            let diag = diagnostics::from_execution_error(&input, &source, &e);
            return Err(anyhow::anyhow!(diag));
        }
    }

    Ok(())
}

pub fn add_lib(name: String) -> anyhow::Result<()> {
    let lib_dir = PathBuf::from("src").join("nlang_libs").join(&name);
    if lib_dir.exists() {
        bail!("Library '{}' already exists at {}", name, lib_dir.display());
    }

    std::fs::create_dir_all(&lib_dir)?;

    let mod_file = lib_dir.join("mod.rs");

    print!("Do you want to use external dependencies (requires manual C implementation)? [y/N]: ");
    std::io::stdout().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let use_native = input.trim().eq_ignore_ascii_case("y");

    let template = if use_native {
        format!(
            r#"use crate::ast::{{Expr, Type}};
use crate::nlang_libs::common::{{LibraryDefinition, LibraryFunction}};

pub fn create_{}_lib() -> LibraryDefinition {{
    let mut lib = LibraryDefinition::new("{}");
    
    // Example function
    // lib.add_function("example", vec![], Type::Void, example_impl);
    
    lib
}}

// fn example_impl(_args: &[Expr]) -> Result<Expr, String> {{
//     Ok(Expr::Literal(crate::ast::Literal::Null))
// }}
"#,
            name, name
        )
    } else {
        format!(
            r#"use crate::ast::{{Expr, Literal, Statement, Type}};
use crate::nlang_libs::common::LibraryDefinition;

pub fn create_{}_lib() -> LibraryDefinition {{
    let mut lib = LibraryDefinition::new("{}");
    
    // Example AST function
    lib.add_ast_function(
        "hello",
        vec![],
        Type::String,
        vec![Statement::Return {{
            value: Some(Box::new(Expr::Literal(Literal::String(
                "Hello from {}!".to_string(),
            )))),
        }}],
    );
    
    lib
}}
"#,
            name, name, name
        )
    };

    std::fs::write(&mod_file, template)?;

    // Update nlang_libs/mod.rs
    let libs_mod_path = PathBuf::from("src").join("nlang_libs").join("mod.rs");
    let mut libs_mod_content = std::fs::read_to_string(&libs_mod_path)?;
    libs_mod_content.push_str(&format!("pub mod {};\n", name));
    std::fs::write(&libs_mod_path, libs_mod_content)?;

    // Update nlang_libs/registry.rs
    let registry_path = PathBuf::from("src").join("nlang_libs").join("registry.rs");
    let mut registry_content = std::fs::read_to_string(&registry_path)?;

    // Insert use statement
    let use_stmt = format!("use crate::nlang_libs::{}::create_{}_lib;\n", name, name);
    if let Some(pos) = registry_content.find("use ") {
        registry_content.insert_str(pos, &use_stmt);
    } else {
        registry_content.insert_str(0, &use_stmt);
    }

    // Insert registration
    let reg_stmt = format!("    registry.register_library(create_{}_lib());\n", name);
    // Look for the comment block in get_default_registry
    if let Some(comment_pos) =
        registry_content.find("// registry.register_library(create_fs_lib());")
    {
        if let Some(newline_pos) = registry_content[comment_pos..].find('\n') {
            registry_content.insert_str(comment_pos + newline_pos + 1, &reg_stmt);
        }
    } else if let Some(fn_pos) = registry_content.find("fn get_default_registry") {
        // Fallback: insert before the return statement
        if let Some(return_pos) = registry_content[fn_pos..].find("registry\n}") {
            registry_content.insert_str(fn_pos + return_pos, &reg_stmt);
        }
    }

    std::fs::write(&registry_path, registry_content)?;

    println!("Library '{}' created successfully.", name);
    Ok(())
}
