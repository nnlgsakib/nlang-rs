use crate::ast::{Expr, Literal};

pub fn http_parse_query(_args: &[Expr]) -> Result<Expr, String> {
    Ok(Expr::ArrayLiteral { elements: vec![] })
}

pub fn http_url_encode(args: &[Expr]) -> Result<Expr, String> {
    let input = match &args[0] {
        Expr::Literal(Literal::String(s)) => s,
        _ => return Err("http_url_encode: argument must be a string".to_string()),
    };
    
    let encoded = urlencoding::encode(input).to_string();
    Ok(Expr::Literal(Literal::String(encoded)))
}

pub fn http_url_decode(args: &[Expr]) -> Result<Expr, String> {
    let input = match &args[0] {
        Expr::Literal(Literal::String(s)) => s,
        _ => return Err("http_url_decode: argument must be a string".to_string()),
    };
    
    let decoded = urlencoding::decode(input)
        .map_err(|e| format!("http_url_decode: failed to decode: {}", e))?
        .to_string();
    
    Ok(Expr::Literal(Literal::String(decoded)))
}

pub fn http_parse_cookie(_args: &[Expr]) -> Result<Expr, String> {
    Ok(Expr::ArrayLiteral { elements: vec![] })
}

pub fn http_build_cookie(args: &[Expr]) -> Result<Expr, String> {
    let name = match &args[0] {
        Expr::Literal(Literal::String(s)) => s,
        _ => return Err("http_build_cookie: first argument must be cookie name".to_string()),
    };
    
    let value = match &args[1] {
        Expr::Literal(Literal::String(s)) => s,
        _ => return Err("http_build_cookie: second argument must be cookie value".to_string()),
    };
    
    let max_age = match &args[2] {
        Expr::Literal(Literal::Integer(n)) => *n,
        _ => return Err("http_build_cookie: third argument must be max age".to_string()),
    };
    
    let cookie = if max_age > 0 {
        format!("{}={}; Max-Age={}; HttpOnly; Secure; SameSite=Strict", name, value, max_age)
    } else {
        format!("{}={}; HttpOnly; Secure; SameSite=Strict", name, value)
    };
    
    Ok(Expr::Literal(Literal::String(cookie)))
}
