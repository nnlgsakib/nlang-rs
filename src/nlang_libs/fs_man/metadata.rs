use crate::ast::{Expr, Literal};
use std::path::Path;
use std::time::SystemTime;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub fn stat(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("stat() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        let path = Path::new(path_str);
        let metadata = std::fs::metadata(path)
            .map_err(|e| format!("Failed to get metadata for '{}': {}", path_str, e))?;
        
        let size = metadata.len() as i64;
        let is_file = metadata.is_file() as i64;
        let is_dir = metadata.is_dir() as i64;
        
        let modified = metadata.modified()
            .map_err(|e| format!("Failed to get modified time: {}", e))?;
        let modified_secs = modified.duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| format!("Invalid system time: {}", e))?
            .as_secs() as i64;
        
        #[cfg(unix)]
        let permissions = metadata.permissions().mode() as i64;
        #[cfg(not(unix))]
        let permissions = if metadata.permissions().readonly() { 0o444 } else { 0o666 };
        
        let stat_array = vec![
            Expr::Literal(Literal::Integer(size)),
            Expr::Literal(Literal::Integer(is_file)),
            Expr::Literal(Literal::Integer(is_dir)),
            Expr::Literal(Literal::Integer(modified_secs)),
            Expr::Literal(Literal::Integer(permissions)),
        ];
        
        Ok(Expr::ArrayLiteral { elements: stat_array })
    } else {
        Err("stat() argument must be a string".to_string())
    }
}

pub fn size(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("size() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        let path = Path::new(path_str);
        let metadata = std::fs::metadata(path)
            .map_err(|e| format!("Failed to get size for '{}': {}", path_str, e))?;
        Ok(Expr::Literal(Literal::Integer(metadata.len() as i64)))
    } else {
        Err("size() argument must be a string".to_string())
    }
}

pub fn last_modified(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("last_modified() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        let path = Path::new(path_str);
        let metadata = std::fs::metadata(path)
            .map_err(|e| format!("Failed to get metadata for '{}': {}", path_str, e))?;
        
        let modified = metadata.modified()
            .map_err(|e| format!("Failed to get modified time: {}", e))?;
        let duration = modified.duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| format!("Invalid system time: {}", e))?;
        
        let timestamp = duration.as_secs() as f64 + duration.subsec_nanos() as f64 / 1_000_000_000.0;
        Ok(Expr::Literal(Literal::Float(timestamp)))
    } else {
        Err("last_modified() argument must be a string".to_string())
    }
}

pub fn permissions(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("permissions() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        let path = Path::new(path_str);
        let metadata = std::fs::metadata(path)
            .map_err(|e| format!("Failed to get permissions for '{}': {}", path_str, e))?;
        
        #[cfg(unix)]
        {
            let mode = metadata.permissions().mode();
            Ok(Expr::Literal(Literal::Integer(mode as i64)))
        }
        
        #[cfg(not(unix))]
        {
            let readonly = metadata.permissions().readonly();
            let mode = if readonly { 0o444 } else { 0o666 };
            Ok(Expr::Literal(Literal::Integer(mode)))
        }
    } else {
        Err("permissions() argument must be a string".to_string())
    }
}

pub fn set_permissions(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 2 {
        return Err("set_permissions() takes exactly 2 arguments".to_string());
    }
    
    let path_str = if let Expr::Literal(Literal::String(s)) = &args[0] {
        s
    } else {
        return Err("set_permissions() first argument must be a string".to_string());
    };
    
    let mode = if let Expr::Literal(Literal::Integer(m)) = &args[1] {
        *m as u32
    } else {
        return Err("set_permissions() second argument must be an integer".to_string());
    };
    
    let path = Path::new(path_str);
    
    #[cfg(unix)]
    {
        use std::fs::Permissions;
        let perms = Permissions::from_mode(mode);
        std::fs::set_permissions(path, perms)
            .map_err(|e| format!("Failed to set permissions for '{}': {}", path_str, e))?;
    }
    
    #[cfg(not(unix))]
    {
        let mut perms = std::fs::metadata(path)
            .map_err(|e| format!("Failed to get metadata for '{}': {}", path_str, e))?
            .permissions();
        
        let readonly = (mode & 0o200) == 0;
        perms.set_readonly(readonly);
        
        std::fs::set_permissions(path, perms)
            .map_err(|e| format!("Failed to set permissions for '{}': {}", path_str, e))?;
    }
    
    Ok(Expr::Literal(Literal::Null))
}
