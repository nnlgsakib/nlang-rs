use crate::ast::{Expr, Literal};
use std::fs;
use std::path::Path;

pub fn read_dir(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("read_dir() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        let entries = fs::read_dir(path_str)
            .map_err(|e| format!("Failed to read directory '{}': {}", path_str, e))?;
        
        let mut result = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let name = entry.file_name().to_string_lossy().to_string();
            result.push(Expr::Literal(Literal::String(name)));
        }
        
        Ok(Expr::ArrayLiteral { elements: result })
    } else {
        Err("read_dir() argument must be a string".to_string())
    }
}

pub fn list_files(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("list_files() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        let entries = fs::read_dir(path_str)
            .map_err(|e| format!("Failed to read directory '{}': {}", path_str, e))?;
        
        let mut result = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            if path.is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                result.push(Expr::Literal(Literal::String(name)));
            }
        }
        
        Ok(Expr::ArrayLiteral { elements: result })
    } else {
        Err("list_files() argument must be a string".to_string())
    }
}

pub fn list_dirs(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("list_dirs() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        let entries = fs::read_dir(path_str)
            .map_err(|e| format!("Failed to read directory '{}': {}", path_str, e))?;
        
        let mut result = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                result.push(Expr::Literal(Literal::String(name)));
            }
        }
        
        Ok(Expr::ArrayLiteral { elements: result })
    } else {
        Err("list_dirs() argument must be a string".to_string())
    }
}

pub fn walk_dir(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("walk_dir() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        let mut result = Vec::new();
        walk_dir_recursive(Path::new(path_str), &mut result)?;
        Ok(Expr::ArrayLiteral { elements: result })
    } else {
        Err("walk_dir() argument must be a string".to_string())
    }
}

fn walk_dir_recursive(path: &Path, result: &mut Vec<Expr>) -> Result<(), String> {
    if path.is_dir() {
        let entries = fs::read_dir(path)
            .map_err(|e| format!("Failed to read directory '{}': {}", path.display(), e))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let entry_path = entry.path();
            let path_str = entry_path.to_string_lossy().to_string();
            result.push(Expr::Literal(Literal::String(path_str)));
            
            if entry_path.is_dir() {
                walk_dir_recursive(&entry_path, result)?;
            }
        }
    }
    
    Ok(())
}

pub fn glob_pattern(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 2 {
        return Err("glob_pattern() takes exactly 2 arguments".to_string());
    }
    
    let dir = if let Expr::Literal(Literal::String(s)) = &args[0] {
        s
    } else {
        return Err("glob_pattern() first argument must be a string".to_string());
    };
    
    let pattern = if let Expr::Literal(Literal::String(s)) = &args[1] {
        s
    } else {
        return Err("glob_pattern() second argument must be a string".to_string());
    };
    
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory '{}': {}", dir, e))?;
    
    let mut result = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let name = entry.file_name().to_string_lossy().to_string();
        
        if simple_pattern_match(&name, pattern) {
            result.push(Expr::Literal(Literal::String(name)));
        }
    }
    
    Ok(Expr::ArrayLiteral { elements: result })
}

fn simple_pattern_match(text: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    
    if pattern.starts_with('*') && pattern.ends_with('*') {
        let middle = &pattern[1..pattern.len()-1];
        return text.contains(middle);
    }
    
    if pattern.starts_with('*') {
        let suffix = &pattern[1..];
        return text.ends_with(suffix);
    }
    
    if pattern.ends_with('*') {
        let prefix = &pattern[..pattern.len()-1];
        return text.starts_with(prefix);
    }
    
    text == pattern
}

pub fn create_dir(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("create_dir() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        fs::create_dir(path_str)
            .map_err(|e| format!("Failed to create directory '{}': {}", path_str, e))?;
        Ok(Expr::Literal(Literal::Null))
    } else {
        Err("create_dir() argument must be a string".to_string())
    }
}

pub fn create_dir_all(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("create_dir_all() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        fs::create_dir_all(path_str)
            .map_err(|e| format!("Failed to create directory '{}': {}", path_str, e))?;
        Ok(Expr::Literal(Literal::Null))
    } else {
        Err("create_dir_all() argument must be a string".to_string())
    }
}
