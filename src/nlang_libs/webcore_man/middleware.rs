use crate::ast::{Expr, Literal};
use std::path::Path;

pub fn static_serve(args: &[Expr]) -> Result<Expr, String> {
    let root_dir = match &args[0] {
        Expr::Literal(Literal::String(s)) => s,
        _ => return Err("static_serve: first argument must be root directory".to_string()),
    };
    
    let requested_path = match &args[1] {
        Expr::Literal(Literal::String(s)) => s,
        _ => return Err("static_serve: second argument must be requested path".to_string()),
    };
    
    let clean_path = requested_path.trim_start_matches('/');
    let full_path = Path::new(root_dir).join(clean_path);
    
    if !full_path.exists() {
        return Ok(Expr::Literal(Literal::Integer(404)));
    }
    
    if !full_path.is_file() {
        return Ok(Expr::Literal(Literal::Integer(403)));
    }
    
    Ok(Expr::Literal(Literal::Integer(200)))
}

pub fn static_serve_with_cache(args: &[Expr]) -> Result<Expr, String> {
    static_serve(&args[0..2])
}

pub fn middleware_cors(args: &[Expr]) -> Result<Expr, String> {
    let req_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id,
        _ => return Err("middleware_cors: argument must be request id".to_string()),
    };
    
    Ok(Expr::Literal(Literal::Integer(req_id)))
}

pub fn middleware_logger(args: &[Expr]) -> Result<Expr, String> {
    let req_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id,
        _ => return Err("middleware_logger: argument must be request id".to_string()),
    };
    
    println!("[MIDDLEWARE] Request {}", req_id);
    
    Ok(Expr::Literal(Literal::Integer(req_id)))
}

pub fn middleware_compress(args: &[Expr]) -> Result<Expr, String> {
    let req_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id,
        _ => return Err("middleware_compress: argument must be request id".to_string()),
    };
    
    Ok(Expr::Literal(Literal::Integer(req_id)))
}

pub fn middleware_timeout(args: &[Expr]) -> Result<Expr, String> {
    let req_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id,
        _ => return Err("middleware_timeout: first argument must be request id".to_string()),
    };
    
    let _timeout_ms = match &args[1] {
        Expr::Literal(Literal::Integer(ms)) => *ms,
        _ => return Err("middleware_timeout: second argument must be timeout in milliseconds".to_string()),
    };
    
    Ok(Expr::Literal(Literal::Integer(req_id)))
}