use crate::ast::{Expr, Literal};
use std::env;

pub fn temp_dir(args: &[Expr]) -> Result<Expr, String> {
    if !args.is_empty() {
        return Err("temp_dir() takes no arguments".to_string());
    }
    
    let temp = env::temp_dir();
    Ok(Expr::Literal(Literal::String(temp.to_string_lossy().to_string())))
}

pub fn current_dir(args: &[Expr]) -> Result<Expr, String> {
    if !args.is_empty() {
        return Err("current_dir() takes no arguments".to_string());
    }
    
    let cwd = env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;
    Ok(Expr::Literal(Literal::String(cwd.to_string_lossy().to_string())))
}

pub fn set_current_dir(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("set_current_dir() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        env::set_current_dir(path_str)
            .map_err(|e| format!("Failed to set current directory to '{}': {}", path_str, e))?;
        Ok(Expr::Literal(Literal::Null))
    } else {
        Err("set_current_dir() argument must be a string".to_string())
    }
}

pub fn home_dir(args: &[Expr]) -> Result<Expr, String> {
    if !args.is_empty() {
        return Err("home_dir() takes no arguments".to_string());
    }
    
    let home = if cfg!(windows) {
        env::var("USERPROFILE").or_else(|_| env::var("HOMEDRIVE")
            .and_then(|drive| env::var("HOMEPATH")
                .map(|path| format!("{}{}", drive, path))))
            .ok()
    } else {
        env::var("HOME").ok()
    };
    
    match home {
        Some(h) => Ok(Expr::Literal(Literal::String(h))),
        None => Ok(Expr::Literal(Literal::String(String::new()))),
    }
}

pub fn create_temp_file(args: &[Expr]) -> Result<Expr, String> {
    if args.len() > 1 {
        return Err("create_temp_file() takes 0 or 1 arguments".to_string());
    }
    
    let prefix = if args.len() == 1 {
        if let Expr::Literal(Literal::String(s)) = &args[0] {
            s.as_str()
        } else {
            return Err("create_temp_file() argument must be a string".to_string());
        }
    } else {
        "tmp"
    };
    
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    
    let temp_path = env::temp_dir().join(format!("{}_{}", prefix, timestamp));
    std::fs::File::create(&temp_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;
    
    Ok(Expr::Literal(Literal::String(temp_path.to_string_lossy().to_string())))
}

pub fn create_temp_dir(args: &[Expr]) -> Result<Expr, String> {
    if args.len() > 1 {
        return Err("create_temp_dir() takes 0 or 1 arguments".to_string());
    }
    
    let prefix = if args.len() == 1 {
        if let Expr::Literal(Literal::String(s)) = &args[0] {
            s.as_str()
        } else {
            return Err("create_temp_dir() argument must be a string".to_string());
        }
    } else {
        "tmp"
    };
    
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    
    let temp_path = env::temp_dir().join(format!("{}_{}", prefix, timestamp));
    std::fs::create_dir(&temp_path)
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;
    
    Ok(Expr::Literal(Literal::String(temp_path.to_string_lossy().to_string())))
}

pub fn get_path_separator(_args: &[Expr]) -> Result<Expr, String> {
    let sep = if cfg!(windows) { "\\" } else { "/" };
    Ok(Expr::Literal(Literal::String(sep.to_string())))
}

pub fn expand_tilde(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("expand_tilde() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        if path_str.starts_with("~") {
            let home = if cfg!(windows) {
                env::var("USERPROFILE").or_else(|_| env::var("HOMEDRIVE")
                    .and_then(|drive| env::var("HOMEPATH")
                        .map(|path| format!("{}{}", drive, path))))
                    .unwrap_or_default()
            } else {
                env::var("HOME").unwrap_or_default()
            };
            
            if home.is_empty() {
                return Ok(Expr::Literal(Literal::String(path_str.clone())));
            }
            
            let expanded = if path_str.len() == 1 {
                home
            } else {
                let rest = &path_str[1..];
                if rest.starts_with('/') || rest.starts_with('\\') {
                    format!("{}{}", home, rest)
                } else {
                    format!("{}/{}", home, rest)
                }
            };
            
            Ok(Expr::Literal(Literal::String(expanded)))
        } else {
            Ok(Expr::Literal(Literal::String(path_str.clone())))
        }
    } else {
        Err("expand_tilde() argument must be a string".to_string())
    }
}