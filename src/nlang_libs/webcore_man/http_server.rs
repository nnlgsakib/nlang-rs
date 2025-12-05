use crate::ast::{Expr, Literal};
use std::net::TcpListener;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct HttpServer {
    pub listener: Option<TcpListener>,
    pub bind_addr: String,
    pub bind_port: i64,
    pub http1_enabled: bool,
    pub http2_enabled: bool,
    pub http3_enabled: bool,
    pub websocket_enabled: bool,
    pub max_connections: i64,
    pub tls_context: i64,
    pub running: bool,
}

pub struct HttpRequest {
    pub method: String,
    pub uri: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub remote_addr: String,
    pub socket_fd: i64,
}

pub struct HttpResponse {
    pub status_code: i64,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub file_path: Option<String>,
}

lazy_static::lazy_static! {
    pub static ref SERVERS: Arc<Mutex<Vec<HttpServer>>> = Arc::new(Mutex::new(Vec::new()));
    pub static ref REQUESTS: Arc<Mutex<Vec<HttpRequest>>> = Arc::new(Mutex::new(Vec::new()));
    pub static ref RESPONSES: Arc<Mutex<Vec<HttpResponse>>> = Arc::new(Mutex::new(Vec::new()));
}

pub fn http_server_new(_args: &[Expr]) -> Result<Expr, String> {
    let server = HttpServer {
        listener: None,
        bind_addr: String::from("0.0.0.0"),
        bind_port: 8080,
        http1_enabled: true,
        http2_enabled: false,
        http3_enabled: false,
        websocket_enabled: false,
        max_connections: 100,
        tls_context: -1,
        running: false,
    };
    
    let mut servers = SERVERS.lock().unwrap();
    servers.push(server);
    let id = servers.len() - 1;
    
    Ok(Expr::Literal(Literal::Integer(id as i64)))
}

pub fn http_server_bind(args: &[Expr]) -> Result<Expr, String> {
    let server_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("http_server_bind: first argument must be server id".to_string()),
    };
    
    let addr = match &args[1] {
        Expr::Literal(Literal::String(s)) => s.clone(),
        _ => return Err("http_server_bind: second argument must be address string".to_string()),
    };
    
    let port = match &args[2] {
        Expr::Literal(Literal::Integer(p)) => *p,
        _ => return Err("http_server_bind: third argument must be port integer".to_string()),
    };
    
    let mut servers = SERVERS.lock().unwrap();
    if server_id >= servers.len() {
        return Err("http_server_bind: invalid server id".to_string());
    }
    
    servers[server_id].bind_addr = addr;
    servers[server_id].bind_port = port;
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn http_server_enable_http1(args: &[Expr]) -> Result<Expr, String> {
    let server_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("http_server_enable_http1: first argument must be server id".to_string()),
    };
    
    let enable = match &args[1] {
        Expr::Literal(Literal::Boolean(b)) => *b,
        _ => return Err("http_server_enable_http1: second argument must be boolean".to_string()),
    };
    
    let mut servers = SERVERS.lock().unwrap();
    if server_id >= servers.len() {
        return Err("http_server_enable_http1: invalid server id".to_string());
    }
    
    servers[server_id].http1_enabled = enable;
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn http_server_enable_http2(args: &[Expr]) -> Result<Expr, String> {
    let server_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("http_server_enable_http2: first argument must be server id".to_string()),
    };
    
    let enable = match &args[1] {
        Expr::Literal(Literal::Boolean(b)) => *b,
        _ => return Err("http_server_enable_http2: second argument must be boolean".to_string()),
    };
    
    let mut servers = SERVERS.lock().unwrap();
    if server_id >= servers.len() {
        return Err("http_server_enable_http2: invalid server id".to_string());
    }
    
    servers[server_id].http2_enabled = enable;
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn http_server_enable_http3(args: &[Expr]) -> Result<Expr, String> {
    let server_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("http_server_enable_http3: first argument must be server id".to_string()),
    };
    
    let enable = match &args[1] {
        Expr::Literal(Literal::Boolean(b)) => *b,
        _ => return Err("http_server_enable_http3: second argument must be boolean".to_string()),
    };
    
    let mut servers = SERVERS.lock().unwrap();
    if server_id >= servers.len() {
        return Err("http_server_enable_http3: invalid server id".to_string());
    }
    
    servers[server_id].http3_enabled = enable;
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn http_server_enable_websocket(args: &[Expr]) -> Result<Expr, String> {
    let server_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("http_server_enable_websocket: first argument must be server id".to_string()),
    };
    
    let enable = match &args[1] {
        Expr::Literal(Literal::Boolean(b)) => *b,
        _ => return Err("http_server_enable_websocket: second argument must be boolean".to_string()),
    };
    
    let mut servers = SERVERS.lock().unwrap();
    if server_id >= servers.len() {
        return Err("http_server_enable_websocket: invalid server id".to_string());
    }
    
    servers[server_id].websocket_enabled = enable;
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn http_server_set_max_connections(args: &[Expr]) -> Result<Expr, String> {
    let server_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("http_server_set_max_connections: first argument must be server id".to_string()),
    };
    
    let max_conn = match &args[1] {
        Expr::Literal(Literal::Integer(n)) => *n,
        _ => return Err("http_server_set_max_connections: second argument must be integer".to_string()),
    };
    
    let mut servers = SERVERS.lock().unwrap();
    if server_id >= servers.len() {
        return Err("http_server_set_max_connections: invalid server id".to_string());
    }
    
    servers[server_id].max_connections = max_conn;
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn http_server_set_tls(args: &[Expr]) -> Result<Expr, String> {
    let server_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("http_server_set_tls: first argument must be server id".to_string()),
    };
    
    let tls_ctx = match &args[1] {
        Expr::Literal(Literal::Integer(ctx)) => *ctx,
        _ => return Err("http_server_set_tls: second argument must be TLS context id".to_string()),
    };
    
    let mut servers = SERVERS.lock().unwrap();
    if server_id >= servers.len() {
        return Err("http_server_set_tls: invalid server id".to_string());
    }
    
    servers[server_id].tls_context = tls_ctx;
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn http_server_start(args: &[Expr]) -> Result<Expr, String> {
    let server_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("http_server_start: argument must be server id".to_string()),
    };
    
    let mut servers = SERVERS.lock().unwrap();
    if server_id >= servers.len() {
        return Err("http_server_start: invalid server id".to_string());
    }
    
    let addr = format!("{}:{}", servers[server_id].bind_addr, servers[server_id].bind_port);
    let listener = TcpListener::bind(&addr)
        .map_err(|e| format!("http_server_start: failed to bind to {}: {}", addr, e))?;
    
    servers[server_id].listener = Some(listener);
    servers[server_id].running = true;
    
    println!("HTTP server started on {}", addr);
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn http_server_stop(args: &[Expr]) -> Result<Expr, String> {
    let server_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("http_server_stop: argument must be server id".to_string()),
    };
    
    let mut servers = SERVERS.lock().unwrap();
    if server_id >= servers.len() {
        return Err("http_server_stop: invalid server id".to_string());
    }
    
    servers[server_id].listener = None;
    servers[server_id].running = false;
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn http_server_accept(args: &[Expr]) -> Result<Expr, String> {
    use std::io::{BufRead, BufReader, Read};
    
    let server_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("http_server_accept: argument must be server id".to_string()),
    };
    
    let servers = SERVERS.lock().unwrap();
    if server_id >= servers.len() {
        return Err("http_server_accept: invalid server id".to_string());
    }
    
    let listener = match &servers[server_id].listener {
        Some(l) => l,
        None => return Err("http_server_accept: server not started".to_string()),
    };
    
    let (stream, remote_addr) = listener.accept()
        .map_err(|e| format!("http_server_accept: failed to accept connection: {}", e))?;
    
    drop(servers);
    
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut request_line = String::new();
    reader.read_line(&mut request_line)
        .map_err(|e| format!("http_server_accept: failed to read request line: {}", e))?;
    
    let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
    if parts.len() < 3 {
        return Err("http_server_accept: invalid HTTP request".to_string());
    }
    
    let method = parts[0].to_string();
    let uri = parts[1].to_string();
    let version = parts[2].to_string();
    
    let mut headers = HashMap::new();
    let mut content_length = 0;
    
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)
            .map_err(|e| format!("http_server_accept: failed to read header: {}", e))?;
        
        if line.trim().is_empty() {
            break;
        }
        
        if let Some(colon_pos) = line.find(':') {
            let (name, value) = line.split_at(colon_pos);
            let name = name.trim().to_lowercase();
            let value = value[1..].trim().to_string();
            
            if name == "content-length" {
                content_length = value.parse().unwrap_or(0);
            }
            
            headers.insert(name, value);
        }
    }
    
    let mut body = String::new();
    if content_length > 0 {
        let mut body_buffer = vec![0u8; content_length];
        reader.read_exact(&mut body_buffer)
            .map_err(|e| format!("http_server_accept: failed to read body: {}", e))?;
        body = String::from_utf8_lossy(&body_buffer).to_string();
    }
    
    let socket_fd = Box::into_raw(Box::new(stream)) as i64;
    
    let request = HttpRequest {
        method,
        uri,
        version,
        headers,
        body,
        remote_addr: remote_addr.to_string(),
        socket_fd,
    };
    
    let mut requests = REQUESTS.lock().unwrap();
    requests.push(request);
    let req_id = requests.len() - 1;
    
    Ok(Expr::Literal(Literal::Integer(req_id as i64)))
}

pub fn http_request_get_method(args: &[Expr]) -> Result<Expr, String> {
    let req_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("http_request_get_method: argument must be request id".to_string()),
    };
    
    let requests = REQUESTS.lock().unwrap();
    if req_id >= requests.len() {
        return Err("http_request_get_method: invalid request id".to_string());
    }
    
    Ok(Expr::Literal(Literal::String(requests[req_id].method.clone())))
}

pub fn http_request_get_uri(args: &[Expr]) -> Result<Expr, String> {
    let req_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("http_request_get_uri: argument must be request id".to_string()),
    };
    
    let requests = REQUESTS.lock().unwrap();
    if req_id >= requests.len() {
        return Err("http_request_get_uri: invalid request id".to_string());
    }
    
    Ok(Expr::Literal(Literal::String(requests[req_id].uri.clone())))
}

pub fn http_request_get_version(args: &[Expr]) -> Result<Expr, String> {
    let req_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("http_request_get_version: argument must be request id".to_string()),
    };
    
    let requests = REQUESTS.lock().unwrap();
    if req_id >= requests.len() {
        return Err("http_request_get_version: invalid request id".to_string());
    }
    
    Ok(Expr::Literal(Literal::String(requests[req_id].version.clone())))
}

pub fn http_request_get_header(args: &[Expr]) -> Result<Expr, String> {
    let req_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("http_request_get_header: first argument must be request id".to_string()),
    };
    
    let name = match &args[1] {
        Expr::Literal(Literal::String(s)) => s.to_lowercase(),
        _ => return Err("http_request_get_header: second argument must be header name".to_string()),
    };
    
    let requests = REQUESTS.lock().unwrap();
    if req_id >= requests.len() {
        return Err("http_request_get_header: invalid request id".to_string());
    }
    
    let value = requests[req_id].headers.get(&name).cloned().unwrap_or_default();
    Ok(Expr::Literal(Literal::String(value)))
}

pub fn http_request_get_body(args: &[Expr]) -> Result<Expr, String> {
    let req_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("http_request_get_body: argument must be request id".to_string()),
    };
    
    let requests = REQUESTS.lock().unwrap();
    if req_id >= requests.len() {
        return Err("http_request_get_body: invalid request id".to_string());
    }
    
    Ok(Expr::Literal(Literal::String(requests[req_id].body.clone())))
}

pub fn http_request_get_remote_addr(args: &[Expr]) -> Result<Expr, String> {
    let req_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("http_request_get_remote_addr: argument must be request id".to_string()),
    };
    
    let requests = REQUESTS.lock().unwrap();
    if req_id >= requests.len() {
        return Err("http_request_get_remote_addr: invalid request id".to_string());
    }
    
    Ok(Expr::Literal(Literal::String(requests[req_id].remote_addr.clone())))
}

pub fn http_request_is_websocket(args: &[Expr]) -> Result<Expr, String> {
    let req_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("http_request_is_websocket: argument must be request id".to_string()),
    };
    
    let requests = REQUESTS.lock().unwrap();
    if req_id >= requests.len() {
        return Err("http_request_is_websocket: invalid request id".to_string());
    }
    
    let is_ws = requests[req_id].headers.get("upgrade")
        .map(|v| v.to_lowercase() == "websocket")
        .unwrap_or(false);
    
    Ok(Expr::Literal(Literal::Boolean(is_ws)))
}

pub fn http_request_free(_args: &[Expr]) -> Result<Expr, String> {
    Ok(Expr::Literal(Literal::Null))
}

pub fn http_response_new(args: &[Expr]) -> Result<Expr, String> {
    let status_code = if args.is_empty() {
        200
    } else {
        match &args[0] {
            Expr::Literal(Literal::Integer(code)) => *code,
            _ => return Err("http_response_new: argument must be status code integer".to_string()),
        }
    };
    
    let response = HttpResponse {
        status_code,
        headers: HashMap::new(),
        body: String::new(),
        file_path: None,
    };
    
    let mut responses = RESPONSES.lock().unwrap();
    responses.push(response);
    let resp_id = responses.len() - 1;
    
    Ok(Expr::Literal(Literal::Integer(resp_id as i64)))
}

pub fn http_response_set_status(args: &[Expr]) -> Result<Expr, String> {
    let resp_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("http_response_set_status: first argument must be response id".to_string()),
    };
    
    let status = match &args[1] {
        Expr::Literal(Literal::Integer(code)) => *code,
        _ => return Err("http_response_set_status: second argument must be status code".to_string()),
    };
    
    let mut responses = RESPONSES.lock().unwrap();
    if resp_id >= responses.len() {
        return Err("http_response_set_status: invalid response id".to_string());
    }
    
    responses[resp_id].status_code = status;
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn http_response_set_header(args: &[Expr]) -> Result<Expr, String> {
    let resp_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("http_response_set_header: first argument must be response id".to_string()),
    };
    
    let name = match &args[1] {
        Expr::Literal(Literal::String(s)) => s.clone(),
        _ => return Err("http_response_set_header: second argument must be header name".to_string()),
    };
    
    let value = match &args[2] {
        Expr::Literal(Literal::String(s)) => s.clone(),
        _ => return Err("http_response_set_header: third argument must be header value".to_string()),
    };
    
    let mut responses = RESPONSES.lock().unwrap();
    if resp_id >= responses.len() {
        return Err("http_response_set_header: invalid response id".to_string());
    }
    
    responses[resp_id].headers.insert(name, value);
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn http_response_set_body(args: &[Expr]) -> Result<Expr, String> {
    let resp_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("http_response_set_body: first argument must be response id".to_string()),
    };
    
    let body = match &args[1] {
        Expr::Literal(Literal::String(s)) => s.clone(),
        _ => return Err("http_response_set_body: second argument must be body string".to_string()),
    };
    
    let mut responses = RESPONSES.lock().unwrap();
    if resp_id >= responses.len() {
        return Err("http_response_set_body: invalid response id".to_string());
    }
    
    responses[resp_id].body = body;
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn http_response_set_file(args: &[Expr]) -> Result<Expr, String> {
    let resp_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("http_response_set_file: first argument must be response id".to_string()),
    };
    
    let file_path = match &args[1] {
        Expr::Literal(Literal::String(s)) => s.clone(),
        _ => return Err("http_response_set_file: second argument must be file path".to_string()),
    };
    
    let mut responses = RESPONSES.lock().unwrap();
    if resp_id >= responses.len() {
        return Err("http_response_set_file: invalid response id".to_string());
    }
    
    responses[resp_id].file_path = Some(file_path);
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn http_response_send(args: &[Expr]) -> Result<Expr, String> {
    use std::io::Write;
    use std::fs::File;
    use std::net::TcpStream;
    
    let resp_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("http_response_send: first argument must be response id".to_string()),
    };
    
    let req_id = match &args[1] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("http_response_send: second argument must be request id".to_string()),
    };
    
    let responses = RESPONSES.lock().unwrap();
    if resp_id >= responses.len() {
        return Err("http_response_send: invalid response id".to_string());
    }
    
    let requests = REQUESTS.lock().unwrap();
    if req_id >= requests.len() {
        return Err("http_response_send: invalid request id".to_string());
    }
    
    let response = &responses[resp_id];
    let socket_fd = requests[req_id].socket_fd;
    
    let stream = unsafe { &mut *(socket_fd as *mut TcpStream) };
    
    let status_text = match response.status_code {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Unknown",
    };
    
    let mut response_str = format!("HTTP/1.1 {} {}\r\n", response.status_code, status_text);
    
    for (name, value) in &response.headers {
        response_str.push_str(&format!("{}: {}\r\n", name, value));
    }
    
    if let Some(file_path) = &response.file_path {
        let mut file = File::open(file_path)
            .map_err(|e| format!("http_response_send: failed to open file: {}", e))?;
        
        let file_size = file.metadata()
            .map_err(|e| format!("http_response_send: failed to get file metadata: {}", e))?
            .len();
        
        response_str.push_str(&format!("Content-Length: {}\r\n\r\n", file_size));
        stream.write_all(response_str.as_bytes())
            .map_err(|e| format!("http_response_send: failed to write headers: {}", e))?;
        
        std::io::copy(&mut file, stream)
            .map_err(|e| format!("http_response_send: failed to write file: {}", e))?;
    } else {
        response_str.push_str(&format!("Content-Length: {}\r\n\r\n", response.body.len()));
        response_str.push_str(&response.body);
        
        stream.write_all(response_str.as_bytes())
            .map_err(|e| format!("http_response_send: failed to write response: {}", e))?;
    }
    
    stream.flush()
        .map_err(|e| format!("http_response_send: failed to flush stream: {}", e))?;
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn http_response_free(_args: &[Expr]) -> Result<Expr, String> {
    Ok(Expr::Literal(Literal::Null))
}