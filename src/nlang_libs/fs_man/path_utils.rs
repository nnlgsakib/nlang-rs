use crate::ast::{Expr, Literal};
use std::path::{Path, PathBuf};

pub fn exists(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("exists() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        let path = Path::new(path_str);
        Ok(Expr::Literal(Literal::Boolean(path.exists())))
    } else {
        Err("exists() argument must be a string".to_string())
    }
}

pub fn is_file(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("is_file() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        let path = Path::new(path_str);
        Ok(Expr::Literal(Literal::Boolean(path.is_file())))
    } else {
        Err("is_file() argument must be a string".to_string())
    }
}

pub fn is_dir(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("is_dir() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        let path = Path::new(path_str);
        Ok(Expr::Literal(Literal::Boolean(path.is_dir())))
    } else {
        Err("is_dir() argument must be a string".to_string())
    }
}

pub fn join(args: &[Expr]) -> Result<Expr, String> {
    if args.is_empty() {
        return Err("join() requires at least 1 argument".to_string());
    }
    
    let mut path = PathBuf::new();
    
    for arg in args {
        if let Expr::Literal(Literal::String(part)) = arg {
            path.push(part);
        } else {
            return Err("join() arguments must be strings".to_string());
        }
    }
    
    let result = path.to_string_lossy().to_string();
    Ok(Expr::Literal(Literal::String(result)))
}

pub fn basename(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("basename() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        let path = Path::new(path_str);
        if let Some(name) = path.file_name() {
            Ok(Expr::Literal(Literal::String(name.to_string_lossy().to_string())))
        } else {
            Ok(Expr::Literal(Literal::String(String::new())))
        }
    } else {
        Err("basename() argument must be a string".to_string())
    }
}

pub fn dirname(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("dirname() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        let path = Path::new(path_str);
        if let Some(parent) = path.parent() {
            Ok(Expr::Literal(Literal::String(parent.to_string_lossy().to_string())))
        } else {
            Ok(Expr::Literal(Literal::String(String::new())))
        }
    } else {
        Err("dirname() argument must be a string".to_string())
    }
}

pub fn extname(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("extname() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        let path = Path::new(path_str);
        if let Some(ext) = path.extension() {
            Ok(Expr::Literal(Literal::String(format!(".{}", ext.to_string_lossy()))))
        } else {
            Ok(Expr::Literal(Literal::String(String::new())))
        }
    } else {
        Err("extname() argument must be a string".to_string())
    }
}

pub fn create_path(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("create_path() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        let path = Path::new(path_str);
        std::fs::create_dir_all(path)
            .map_err(|e| format!("Failed to create path '{}': {}", path_str, e))?;
        Ok(Expr::Literal(Literal::Null))
    } else {
        Err("create_path() argument must be a string".to_string())
    }
}

pub fn remove_path(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("remove_path() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        let path = Path::new(path_str);
        
        if !path.exists() {
            return Err(format!("Path does not exist: '{}'", path_str));
        }
        
        if path.is_dir() {
            std::fs::remove_dir_all(path)
                .map_err(|e| format!("Failed to remove directory '{}': {}", path_str, e))?;
        } else {
            std::fs::remove_file(path)
                .map_err(|e| format!("Failed to remove file '{}': {}", path_str, e))?;
        }
        
        Ok(Expr::Literal(Literal::Null))
    } else {
        Err("remove_path() argument must be a string".to_string())
    }
}
