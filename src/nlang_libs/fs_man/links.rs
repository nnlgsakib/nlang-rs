use crate::ast::{Expr, Literal};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs as unix_fs;

#[cfg(windows)]
use std::os::windows::fs as windows_fs;

pub fn is_symlink(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("is_symlink() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        let path = Path::new(path_str);
        let metadata = std::fs::symlink_metadata(path)
            .map_err(|e| format!("Failed to get metadata for '{}': {}", path_str, e))?;
        Ok(Expr::Literal(Literal::Boolean(metadata.file_type().is_symlink())))
    } else {
        Err("is_symlink() argument must be a string".to_string())
    }
}

pub fn read_link(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("read_link() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        let target = std::fs::read_link(path_str)
            .map_err(|e| format!("Failed to read link '{}': {}", path_str, e))?;
        Ok(Expr::Literal(Literal::String(target.to_string_lossy().to_string())))
    } else {
        Err("read_link() argument must be a string".to_string())
    }
}

pub fn create_symlink(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 2 {
        return Err("create_symlink() takes exactly 2 arguments".to_string());
    }
    
    let target = if let Expr::Literal(Literal::String(s)) = &args[0] {
        s
    } else {
        return Err("create_symlink() first argument must be a string".to_string());
    };
    
    let link = if let Expr::Literal(Literal::String(s)) = &args[1] {
        s
    } else {
        return Err("create_symlink() second argument must be a string".to_string());
    };
    
    #[cfg(unix)]
    {
        unix_fs::symlink(target, link)
            .map_err(|e| format!("Failed to create symlink from '{}' to '{}': {}", link, target, e))?;
    }
    
    #[cfg(windows)]
    {
        let target_path = Path::new(target);
        if target_path.is_dir() {
            windows_fs::symlink_dir(target, link)
                .map_err(|e| format!("Failed to create symlink from '{}' to '{}': {}", link, target, e))?;
        } else {
            windows_fs::symlink_file(target, link)
                .map_err(|e| format!("Failed to create symlink from '{}' to '{}': {}", link, target, e))?;
        }
    }
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn create_hard_link(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 2 {
        return Err("create_hard_link() takes exactly 2 arguments".to_string());
    }
    
    let target = if let Expr::Literal(Literal::String(s)) = &args[0] {
        s
    } else {
        return Err("create_hard_link() first argument must be a string".to_string());
    };
    
    let link = if let Expr::Literal(Literal::String(s)) = &args[1] {
        s
    } else {
        return Err("create_hard_link() second argument must be a string".to_string());
    };
    
    std::fs::hard_link(target, link)
        .map_err(|e| format!("Failed to create hard link from '{}' to '{}': {}", link, target, e))?;
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn canonicalize(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("canonicalize() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        let canonical = std::fs::canonicalize(path_str)
            .map_err(|e| format!("Failed to canonicalize path '{}': {}", path_str, e))?;
        Ok(Expr::Literal(Literal::String(canonical.to_string_lossy().to_string())))
    } else {
        Err("canonicalize() argument must be a string".to_string())
    }
}

pub fn absolute_path(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("absolute_path() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        let path = Path::new(path_str);
        let absolute = if path.is_absolute() {
            PathBuf::from(path)
        } else {
            std::env::current_dir()
                .map_err(|e| format!("Failed to get current directory: {}", e))?
                .join(path)
        };
        Ok(Expr::Literal(Literal::String(absolute.to_string_lossy().to_string())))
    } else {
        Err("absolute_path() argument must be a string".to_string())
    }
}

pub fn is_absolute(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("is_absolute() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        let path = Path::new(path_str);
        Ok(Expr::Literal(Literal::Boolean(path.is_absolute())))
    } else {
        Err("is_absolute() argument must be a string".to_string())
    }
}

pub fn is_relative(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("is_relative() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        let path = Path::new(path_str);
        Ok(Expr::Literal(Literal::Boolean(path.is_relative())))
    } else {
        Err("is_relative() argument must be a string".to_string())
    }
}
