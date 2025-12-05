use crate::ast::{Expr, Literal};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

#[allow(dead_code)]
pub struct Route {
    pub pattern: String,
    pub method: String,
    pub params: HashMap<String, String>,
}

pub struct Router {
    pub routes: Vec<Route>,
    pub matched_params: HashMap<String, String>,
}

lazy_static::lazy_static! {
    static ref ROUTERS: Arc<Mutex<Vec<Router>>> = Arc::new(Mutex::new(Vec::new()));
}

pub fn router_new(_args: &[Expr]) -> Result<Expr, String> {
    let router = Router {
        routes: Vec::new(),
        matched_params: HashMap::new(),
    };
    
    let mut routers = ROUTERS.lock().unwrap();
    routers.push(router);
    let id = routers.len() - 1;
    
    Ok(Expr::Literal(Literal::Integer(id as i64)))
}

pub fn router_get(args: &[Expr]) -> Result<Expr, String> {
    let router_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("router_get: first argument must be router id".to_string()),
    };
    
    let pattern = match &args[1] {
        Expr::Literal(Literal::String(s)) => s.clone(),
        _ => return Err("router_get: second argument must be pattern string".to_string()),
    };
    
    let mut routers = ROUTERS.lock().unwrap();
    if router_id >= routers.len() {
        return Err("router_get: invalid router id".to_string());
    }
    
    routers[router_id].routes.push(Route {
        pattern,
        method: "GET".to_string(),
        params: HashMap::new(),
    });
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn router_post(args: &[Expr]) -> Result<Expr, String> {
    let router_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("router_post: first argument must be router id".to_string()),
    };
    
    let pattern = match &args[1] {
        Expr::Literal(Literal::String(s)) => s.clone(),
        _ => return Err("router_post: second argument must be pattern string".to_string()),
    };
    
    let mut routers = ROUTERS.lock().unwrap();
    if router_id >= routers.len() {
        return Err("router_post: invalid router id".to_string());
    }
    
    routers[router_id].routes.push(Route {
        pattern,
        method: "POST".to_string(),
        params: HashMap::new(),
    });
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn router_put(args: &[Expr]) -> Result<Expr, String> {
    let router_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("router_put: first argument must be router id".to_string()),
    };
    
    let pattern = match &args[1] {
        Expr::Literal(Literal::String(s)) => s.clone(),
        _ => return Err("router_put: second argument must be pattern string".to_string()),
    };
    
    let mut routers = ROUTERS.lock().unwrap();
    if router_id >= routers.len() {
        return Err("router_put: invalid router id".to_string());
    }
    
    routers[router_id].routes.push(Route {
        pattern,
        method: "PUT".to_string(),
        params: HashMap::new(),
    });
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn router_delete(args: &[Expr]) -> Result<Expr, String> {
    let router_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("router_delete: first argument must be router id".to_string()),
    };
    
    let pattern = match &args[1] {
        Expr::Literal(Literal::String(s)) => s.clone(),
        _ => return Err("router_delete: second argument must be pattern string".to_string()),
    };
    
    let mut routers = ROUTERS.lock().unwrap();
    if router_id >= routers.len() {
        return Err("router_delete: invalid router id".to_string());
    }
    
    routers[router_id].routes.push(Route {
        pattern,
        method: "DELETE".to_string(),
        params: HashMap::new(),
    });
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn router_match(args: &[Expr]) -> Result<Expr, String> {
    let router_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("router_match: first argument must be router id".to_string()),
    };
    
    let method = match &args[1] {
        Expr::Literal(Literal::String(s)) => s.clone(),
        _ => return Err("router_match: second argument must be method string".to_string()),
    };
    
    let path = match &args[2] {
        Expr::Literal(Literal::String(s)) => s.clone(),
        _ => return Err("router_match: third argument must be path string".to_string()),
    };
    
    let mut routers = ROUTERS.lock().unwrap();
    if router_id >= routers.len() {
        return Err("router_match: invalid router id".to_string());
    }
    
    routers[router_id].matched_params.clear();
    
    for route in &routers[router_id].routes {
        if route.method != method {
            continue;
        }
        
        let pattern_parts: Vec<&str> = route.pattern.split('/').collect();
        let path_parts: Vec<&str> = path.split('/').collect();
        
        if pattern_parts.len() != path_parts.len() {
            continue;
        }
        
        let mut matched = true;
        let mut params = HashMap::new();
        
        for (pattern_part, path_part) in pattern_parts.iter().zip(path_parts.iter()) {
            if pattern_part.starts_with(':') {
                let param_name = &pattern_part[1..];
                params.insert(param_name.to_string(), path_part.to_string());
            } else if pattern_part != path_part {
                matched = false;
                break;
            }
        }
        
        if matched {
            routers[router_id].matched_params = params;
            return Ok(Expr::Literal(Literal::Boolean(true)));
        }
    }
    
    Ok(Expr::Literal(Literal::Boolean(false)))
}

pub fn router_get_param(args: &[Expr]) -> Result<Expr, String> {
    let router_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("router_get_param: first argument must be router id".to_string()),
    };
    
    let name = match &args[1] {
        Expr::Literal(Literal::String(s)) => s,
        _ => return Err("router_get_param: second argument must be parameter name".to_string()),
    };
    
    let routers = ROUTERS.lock().unwrap();
    if router_id >= routers.len() {
        return Err("router_get_param: invalid router id".to_string());
    }
    
    let value = routers[router_id].matched_params
        .get(name)
        .cloned()
        .unwrap_or_default();
    
    Ok(Expr::Literal(Literal::String(value)))
}