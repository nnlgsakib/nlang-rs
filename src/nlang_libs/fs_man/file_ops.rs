use crate::ast::{Expr, Literal};
use std::fs;
use std::path::Path;

pub fn copy_file(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 2 {
        return Err("copy_file() takes exactly 2 arguments".to_string());
    }
    
    let src = if let Expr::Literal(Literal::String(s)) = &args[0] {
        s
    } else {
        return Err("copy_file() first argument must be a string".to_string());
    };
    
    let dst = if let Expr::Literal(Literal::String(s)) = &args[1] {
        s
    } else {
        return Err("copy_file() second argument must be a string".to_string());
    };
    
    fs::copy(src, dst)
        .map_err(|e| format!("Failed to copy file from '{}' to '{}': {}", src, dst, e))?;
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn move_file(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 2 {
        return Err("move_file() takes exactly 2 arguments".to_string());
    }
    
    let src = if let Expr::Literal(Literal::String(s)) = &args[0] {
        s
    } else {
        return Err("move_file() first argument must be a string".to_string());
    };
    
    let dst = if let Expr::Literal(Literal::String(s)) = &args[1] {
        s
    } else {
        return Err("move_file() second argument must be a string".to_string());
    };
    
    fs::rename(src, dst)
        .map_err(|e| format!("Failed to move file from '{}' to '{}': {}", src, dst, e))?;
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn rename_file(args: &[Expr]) -> Result<Expr, String> {
    move_file(args)
}

pub fn copy_dir(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 2 {
        return Err("copy_dir() takes exactly 2 arguments".to_string());
    }
    
    let src = if let Expr::Literal(Literal::String(s)) = &args[0] {
        s
    } else {
        return Err("copy_dir() first argument must be a string".to_string());
    };
    
    let dst = if let Expr::Literal(Literal::String(s)) = &args[1] {
        s
    } else {
        return Err("copy_dir() second argument must be a string".to_string());
    };
    
    copy_dir_recursive(Path::new(src), Path::new(dst))
        .map_err(|e| format!("Failed to copy directory from '{}' to '{}': {}", src, dst, e))?;
    
    Ok(Expr::Literal(Literal::Null))
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);
        
        if path.is_dir() {
            copy_dir_recursive(&path, &dst_path)?;
        } else {
            fs::copy(&path, &dst_path)?;
        }
    }
    
    Ok(())
}

pub fn create_file(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("create_file() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        fs::File::create(path_str)
            .map_err(|e| format!("Failed to create file '{}': {}", path_str, e))?;
        Ok(Expr::Literal(Literal::Null))
    } else {
        Err("create_file() argument must be a string".to_string())
    }
}

pub fn remove_file(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("remove_file() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        fs::remove_file(path_str)
            .map_err(|e| format!("Failed to remove file '{}': {}", path_str, e))?;
        Ok(Expr::Literal(Literal::Null))
    } else {
        Err("remove_file() argument must be a string".to_string())
    }
}

pub fn remove_dir(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("remove_dir() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        fs::remove_dir(path_str)
            .map_err(|e| format!("Failed to remove directory '{}': {}", path_str, e))?;
        Ok(Expr::Literal(Literal::Null))
    } else {
        Err("remove_dir() argument must be a string".to_string())
    }
}

pub fn remove_dir_all(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("remove_dir_all() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        fs::remove_dir_all(path_str)
            .map_err(|e| format!("Failed to remove directory '{}': {}", path_str, e))?;
        Ok(Expr::Literal(Literal::Null))
    } else {
        Err("remove_dir_all() argument must be a string".to_string())
    }
}
