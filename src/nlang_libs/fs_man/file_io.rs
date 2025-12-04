use crate::ast::{Expr, Literal};
use std::fs;
use std::io::Write as IoWrite;

pub fn read_file(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("read_file() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        let contents = fs::read_to_string(path_str)
            .map_err(|e| format!("Failed to read file '{}': {}", path_str, e))?;
        Ok(Expr::Literal(Literal::String(contents)))
    } else {
        Err("read_file() argument must be a string".to_string())
    }
}

pub fn read_bytes(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("read_bytes() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        let bytes = fs::read(path_str)
            .map_err(|e| format!("Failed to read file '{}': {}", path_str, e))?;
        
        let byte_exprs: Vec<Expr> = bytes.iter()
            .map(|&b| Expr::Literal(Literal::Integer(b as i64)))
            .collect();
        
        Ok(Expr::ArrayLiteral { elements: byte_exprs })
    } else {
        Err("read_bytes() argument must be a string".to_string())
    }
}

pub fn write_file(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 2 {
        return Err("write_file() takes exactly 2 arguments".to_string());
    }
    
    let path_str = if let Expr::Literal(Literal::String(s)) = &args[0] {
        s
    } else {
        return Err("write_file() first argument must be a string".to_string());
    };
    
    let content = if let Expr::Literal(Literal::String(s)) = &args[1] {
        s
    } else {
        return Err("write_file() second argument must be a string".to_string());
    };
    
    fs::write(path_str, content)
        .map_err(|e| format!("Failed to write file '{}': {}", path_str, e))?;
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn write_bytes(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 2 {
        return Err("write_bytes() takes exactly 2 arguments".to_string());
    }
    
    let path_str = if let Expr::Literal(Literal::String(s)) = &args[0] {
        s
    } else {
        return Err("write_bytes() first argument must be a string".to_string());
    };
    
    let bytes = if let Expr::ArrayLiteral { elements } = &args[1] {
        elements.iter().map(|e| {
            if let Expr::Literal(Literal::Integer(i)) = e {
                Ok(*i as u8)
            } else {
                Err("write_bytes() array must contain integers".to_string())
            }
        }).collect::<Result<Vec<u8>, String>>()?
    } else {
        return Err("write_bytes() second argument must be an array".to_string());
    };
    
    fs::write(path_str, bytes)
        .map_err(|e| format!("Failed to write file '{}': {}", path_str, e))?;
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn append_file(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 2 {
        return Err("append_file() takes exactly 2 arguments".to_string());
    }
    
    let path_str = if let Expr::Literal(Literal::String(s)) = &args[0] {
        s
    } else {
        return Err("append_file() first argument must be a string".to_string());
    };
    
    let content = if let Expr::Literal(Literal::String(s)) = &args[1] {
        s
    } else {
        return Err("append_file() second argument must be a string".to_string());
    };
    
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path_str)
        .map_err(|e| format!("Failed to open file '{}': {}", path_str, e))?;
    
    file.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to append to file '{}': {}", path_str, e))?;
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn read_lines(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("read_lines() takes exactly 1 argument".to_string());
    }
    
    if let Expr::Literal(Literal::String(path_str)) = &args[0] {
        let contents = fs::read_to_string(path_str)
            .map_err(|e| format!("Failed to read file '{}': {}", path_str, e))?;
        
        let lines: Vec<Expr> = contents.lines()
            .map(|line| Expr::Literal(Literal::String(line.to_string())))
            .collect();
        
        Ok(Expr::ArrayLiteral { elements: lines })
    } else {
        Err("read_lines() argument must be a string".to_string())
    }
}

pub fn truncate_file(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 2 {
        return Err("truncate_file() takes exactly 2 arguments".to_string());
    }
    
    let path_str = if let Expr::Literal(Literal::String(s)) = &args[0] {
        s
    } else {
        return Err("truncate_file() first argument must be a string".to_string());
    };
    
    let length = if let Expr::Literal(Literal::Integer(i)) = &args[1] {
        *i as u64
    } else {
        return Err("truncate_file() second argument must be an integer".to_string());
    };
    
    let file = fs::OpenOptions::new()
        .write(true)
        .open(path_str)
        .map_err(|e| format!("Failed to open file '{}': {}", path_str, e))?;
    
    file.set_len(length)
        .map_err(|e| format!("Failed to truncate file '{}': {}", path_str, e))?;
    
    Ok(Expr::Literal(Literal::Null))
}