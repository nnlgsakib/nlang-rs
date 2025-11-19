use crate::ast::{Expr, Literal};
use std::io::{self, Write};
use sha2::{Digest, Sha256};
use hex;
use rand::RngCore;

// Helper function to extract string value from expression
fn extract_string_value(expr: &Expr) -> Result<String, String> {
    match expr {
        Expr::Literal(Literal::String(s)) => Ok(s.clone()),
        _ => Err("Expected string literal".to_string()),
    }
}

// Helper function to extract integer value from expression
fn extract_integer_value(expr: &Expr) -> Result<i64, String> {
    match expr {
        Expr::Literal(Literal::Integer(i)) => Ok(*i),
        _ => Err("Expected integer literal".to_string()),
    }
}

// Helper function to extract float value from expression
#[allow(dead_code)]
fn extract_float_value(expr: &Expr) -> Result<f64, String> {
    match expr {
        Expr::Literal(Literal::Float(f)) => Ok(*f),
        _ => Err("Expected float literal".to_string()),
    }
}

// Helper function to extract boolean value from expression
#[allow(dead_code)]
fn extract_boolean_value(expr: &Expr) -> Result<bool, String> {
    match expr {
        Expr::Literal(Literal::Boolean(b)) => Ok(*b),
        _ => Err("Expected boolean literal".to_string()),
    }
}

// Helper function to convert any expression to string for printing
fn expr_to_string(expr: &Expr) -> Result<String, String> {
    match expr {
        Expr::Literal(Literal::String(s)) => Ok(s.clone()),
        Expr::Literal(Literal::Integer(i)) => Ok(i.to_string()),
        Expr::Literal(Literal::Float(f)) => Ok(f.to_string()),
        Expr::Literal(Literal::Boolean(b)) => Ok(b.to_string()),
        Expr::Literal(Literal::Null) => Ok("null".to_string()),
        Expr::ArrayLiteral { elements } => {
            let mut parts = Vec::new();
            for element in elements {
                parts.push(expr_to_string(element)?);
            }
            Ok(format!("[{}]", parts.join(", ")))
        }
        _ => Err(format!("Cannot convert expression to string: {:?}", expr)),
    }
}

// Built-in function implementations
pub fn builtin_print(args: &[Expr]) -> Result<Expr, String> {
    if !args.is_empty() {
        if let Expr::Literal(Literal::String(format_str)) = &args[0] {
            if args.len() > 1 && format_str.contains("[]") {
                // Placeholder logic
                let parts: Vec<&str> = format_str.split("[]").collect();
                let mut arg_idx = 1;
                for (i, part) in parts.iter().enumerate() {
                    print!("{}", part);
                    if i < parts.len() - 1 {
                        if arg_idx < args.len() {
                            print!("{}", expr_to_string(&args[arg_idx])?);
                            arg_idx += 1;
                        } else {
                            print!("[]");
                        }
                    }
                }
                // Print remaining arguments if any
                while arg_idx < args.len() {
                    print!(" {}", expr_to_string(&args[arg_idx])?);
                    arg_idx += 1;
                }
            } else {
                // Old logic: print all args space-separated
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        print!(" ");
                    }
                    print!("{}", expr_to_string(arg)?);
                }
            }
        } else {
            // First arg not a string, print all args space-separated
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    print!(" ");
                }
                print!("{}", expr_to_string(arg)?);
            }
        }
    }
    io::stdout().flush().map_err(|e| format!("IO error: {}", e))?;
    Ok(Expr::Literal(Literal::Null))
}

pub fn builtin_println(args: &[Expr]) -> Result<Expr, String> {
    if !args.is_empty() {
        if let Expr::Literal(Literal::String(format_str)) = &args[0] {
            if args.len() > 1 && format_str.contains("[]") {
                // Placeholder logic
                let parts: Vec<&str> = format_str.split("[]").collect();
                let mut arg_idx = 1;
                for (i, part) in parts.iter().enumerate() {
                    print!("{}", part);
                    if i < parts.len() - 1 {
                        if arg_idx < args.len() {
                            print!("{}", expr_to_string(&args[arg_idx])?);
                            arg_idx += 1;
                        } else {
                            print!("[]");
                        }
                    }
                }
                // Print remaining arguments if any
                while arg_idx < args.len() {
                    print!(" {}", expr_to_string(&args[arg_idx])?);
                    arg_idx += 1;
                }
            } else {
                // Old logic: print all args space-separated
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        print!(" ");
                    }
                    print!("{}", expr_to_string(arg)?);
                }
            }
        } else {
            // First arg not a string, print all args space-separated
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    print!(" ");
                }
                print!("{}", expr_to_string(arg)?);
            }
        }
    }
    println!();
    Ok(Expr::Literal(Literal::Null))
}

pub fn builtin_input(args: &[Expr]) -> Result<Expr, String> {
    if args.len() > 1 {
        return Err("input() takes 0 or 1 arguments".to_string());
    }

    if let Some(prompt_expr) = args.get(0) {
        print!("{}", expr_to_string(prompt_expr)?);
        io::stdout().flush().map_err(|e| format!("IO error: {}", e))?;
    }

    let mut input = String::new();
    io::stdin().read_line(&mut input)
        .map_err(|e| format!("IO error: {}", e))?;
    
    // Remove trailing newline
    if input.ends_with('\n') {
        input.pop();
        if input.ends_with('\r') {
            input.pop();
        }
    }
    
    Ok(Expr::Literal(Literal::String(input)))
}

pub fn builtin_len_string(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("len() takes exactly 1 argument".to_string());
    }
    
    match &args[0] {
        Expr::Literal(Literal::String(s)) => Ok(Expr::Literal(Literal::Integer(s.len() as i64))),
        _ => Err("len() argument must be a string".to_string()),
    }
}

pub fn builtin_len_array(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("len() takes exactly 1 argument".to_string());
    }

    match &args[0] {
        Expr::ArrayLiteral { elements } => Ok(Expr::Literal(Literal::Integer(elements.len() as i64))),
        _ => Err("len() argument must be an array".to_string()),
    }
}

pub fn builtin_sha256_string(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 { return Err("sha256() takes exactly 1 argument".to_string()); }
    if let Expr::Literal(Literal::String(s)) = &args[0] {
        let mut hasher = Sha256::new();
        hasher.update(s.as_bytes());
        let result = hasher.finalize();
        Ok(Expr::Literal(Literal::String(hex::encode(result))))
    } else {
        Err("sha256() argument must be a string".to_string())
    }
}

pub fn builtin_sha256_array(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 { return Err("sha256() takes exactly 1 argument".to_string()); }
    if let Expr::ArrayLiteral { elements } = &args[0] {
        let mut buf: Vec<u8> = Vec::with_capacity(elements.len());
        for e in elements {
            match e {
                Expr::Literal(Literal::Integer(i)) => buf.push((*i as i64 & 0xFF) as u8),
                _ => return Err("sha256() array must contain integers".to_string()),
            }
        }
        let mut hasher = Sha256::new();
        hasher.update(&buf);
        let result = hasher.finalize();
        Ok(Expr::Literal(Literal::String(hex::encode(result))))
    } else {
        Err("sha256() argument must be an array literal".to_string())
    }
}

pub fn builtin_sha256_random(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 { return Err("sha256_random() takes exactly 1 argument".to_string()); }
    let n = match &args[0] { Expr::Literal(Literal::Integer(i)) => *i, _ => return Err("sha256_random(n) expects integer length".to_string()) };
    if n <= 0 { return Err("sha256_random length must be > 0".to_string()); }
    let mut buf = vec![0u8; n as usize];
    rand::rngs::OsRng.fill_bytes(&mut buf);
    let mut hasher = Sha256::new();
    hasher.update(&buf);
    let result = hasher.finalize();
    Ok(Expr::Literal(Literal::String(hex::encode(result))))
}

pub fn builtin_int(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("int() takes exactly 1 argument".to_string());
    }
    
    let text = extract_string_value(&args[0])?;
    let parsed = text.trim().parse::<i64>()
        .map_err(|_| format!("Cannot convert '{}' to integer", text))?;
    
    Ok(Expr::Literal(Literal::Integer(parsed)))
}

pub fn builtin_int_from_float(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("int() takes exactly 1 argument".to_string());
    }
    
    let num = extract_float_value(&args[0])?;
    Ok(Expr::Literal(Literal::Integer(num as i64)))
}

pub fn builtin_str(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("str() takes exactly 1 argument".to_string());
    }
    
    let num = extract_integer_value(&args[0])?;
    Ok(Expr::Literal(Literal::String(num.to_string())))
}

pub fn builtin_str_i8(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("str() takes exactly 1 argument".to_string());
    }
    
    let num = extract_integer_value(&args[0])?;
    Ok(Expr::Literal(Literal::String(num.to_string())))
}

pub fn builtin_str_i16(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("str() takes exactly 1 argument".to_string());
    }
    
    let num = extract_integer_value(&args[0])?;
    Ok(Expr::Literal(Literal::String(num.to_string())))
}

pub fn builtin_str_i32(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("str() takes exactly 1 argument".to_string());
    }
    
    let num = extract_integer_value(&args[0])?;
    Ok(Expr::Literal(Literal::String(num.to_string())))
}

pub fn builtin_str_i64(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("str() takes exactly 1 argument".to_string());
    }
    
    let num = extract_integer_value(&args[0])?;
    Ok(Expr::Literal(Literal::String(num.to_string())))
}

pub fn builtin_str_isize(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("str() takes exactly 1 argument".to_string());
    }
    
    let num = extract_integer_value(&args[0])?;
    Ok(Expr::Literal(Literal::String(num.to_string())))
}

pub fn builtin_str_u8(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("str() takes exactly 1 argument".to_string());
    }
    
    let num = extract_integer_value(&args[0])?;
    Ok(Expr::Literal(Literal::String(num.to_string())))
}

pub fn builtin_str_u16(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("str() takes exactly 1 argument".to_string());
    }
    
    let num = extract_integer_value(&args[0])?;
    Ok(Expr::Literal(Literal::String(num.to_string())))
}

pub fn builtin_str_u32(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("str() takes exactly 1 argument".to_string());
    }
    
    let num = extract_integer_value(&args[0])?;
    Ok(Expr::Literal(Literal::String(num.to_string())))
}

pub fn builtin_str_u64(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("str() takes exactly 1 argument".to_string());
    }
    
    let num = extract_integer_value(&args[0])?;
    Ok(Expr::Literal(Literal::String(num.to_string())))
}

pub fn builtin_str_usize(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("str() takes exactly 1 argument".to_string());
    }
    
    let num = extract_integer_value(&args[0])?;
    Ok(Expr::Literal(Literal::String(num.to_string())))
}

pub fn builtin_str_from_bool(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("str() takes exactly 1 argument".to_string());
    }
    
    let bool_val = extract_boolean_value(&args[0])?;
    Ok(Expr::Literal(Literal::String(bool_val.to_string())))
}

pub fn builtin_str_from_float(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("str() takes exactly 1 argument".to_string());
    }
    
    let num = extract_float_value(&args[0])?;
    Ok(Expr::Literal(Literal::String(num.to_string())))
}

pub fn builtin_float(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("float() takes exactly 1 argument".to_string());
    }
    
    let text = extract_string_value(&args[0])?;
    let parsed = text.trim().parse::<f64>()
        .map_err(|_| format!("Cannot convert '{}' to float", text))?;
    
    Ok(Expr::Literal(Literal::Float(parsed)))
}

pub fn builtin_abs(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("abs() takes exactly 1 argument".to_string());
    }
    
    let num = extract_integer_value(&args[0])?;
    Ok(Expr::Literal(Literal::Integer(num.abs())))
}

pub fn builtin_abs_float(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("abs_float() takes exactly 1 argument".to_string());
    }
    
    let num = extract_float_value(&args[0])?;
    Ok(Expr::Literal(Literal::Float(num.abs())))
}

pub fn builtin_max(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 2 {
        return Err("max() takes exactly 2 arguments".to_string());
    }
    
    let a = extract_integer_value(&args[0])?;
    let b = extract_integer_value(&args[1])?;
    Ok(Expr::Literal(Literal::Integer(a.max(b))))
}

pub fn builtin_min(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 2 {
        return Err("min() takes exactly 2 arguments".to_string());
    }
    
    let a = extract_integer_value(&args[0])?;
    let b = extract_integer_value(&args[1])?;
    Ok(Expr::Literal(Literal::Integer(a.min(b))))
}

pub fn builtin_pow(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 2 {
        return Err("pow() takes exactly 2 arguments".to_string());
    }
    
    let base = extract_integer_value(&args[0])?;
    let exp = extract_integer_value(&args[1])?;
    
    if exp < 0 {
        return Err("Negative exponents not supported for integer power".to_string());
    }
    
    let result = base.pow(exp as u32);
    Ok(Expr::Literal(Literal::Integer(result)))
}

pub fn builtin_bool(args: &[Expr]) -> Result<Expr, String> {
    if args.len() != 1 {
        return Err("bool() takes exactly 1 argument".to_string());
    }
    
    let num = extract_integer_value(&args[0])?;
    Ok(Expr::Literal(Literal::Boolean(num != 0)))
}
