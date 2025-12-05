use crate::ast::{Expr, Literal};
use native_tls::{TlsAcceptor, TlsConnector, TlsStream};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::io::{Read, Write};

lazy_static::lazy_static! {
    static ref TLS_CONTEXTS: Arc<Mutex<Vec<TlsContext>>> = Arc::new(Mutex::new(Vec::new()));
    static ref TLS_STREAMS: Arc<Mutex<Vec<TlsStreamWrapper>>> = Arc::new(Mutex::new(Vec::new()));
}

pub struct TlsContext {
    pub acceptor: Option<TlsAcceptor>,
    pub connector: Option<TlsConnector>,
    pub cert_path: String,
    pub key_path: String,
    pub alpn_protocols: Vec<String>,
}

pub struct TlsStreamWrapper {
    pub stream: TlsStream<TcpStream>,
    pub alpn_selected: String,
}

pub fn tls_context_new(_args: &[Expr]) -> Result<Expr, String> {
    let context = TlsContext {
        acceptor: None,
        connector: Some(TlsConnector::new().map_err(|e| format!("Failed to create TLS connector: {}", e))?),
        cert_path: String::new(),
        key_path: String::new(),
        alpn_protocols: vec!["h2".to_string(), "http/1.1".to_string()],
    };
    
    let mut contexts = TLS_CONTEXTS.lock().unwrap();
    contexts.push(context);
    let id = contexts.len() - 1;
    
    Ok(Expr::Literal(Literal::Integer(id as i64)))
}

pub fn tls_load_cert(args: &[Expr]) -> Result<Expr, String> {
    let ctx_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("tls_load_cert: first argument must be context id".to_string()),
    };
    
    let cert_path = match &args[1] {
        Expr::Literal(Literal::String(s)) => s.clone(),
        _ => return Err("tls_load_cert: second argument must be certificate path".to_string()),
    };
    
    let key_path = match &args[2] {
        Expr::Literal(Literal::String(s)) => s.clone(),
        _ => return Err("tls_load_cert: third argument must be key path".to_string()),
    };
    
    let mut contexts = TLS_CONTEXTS.lock().unwrap();
    if ctx_id >= contexts.len() {
        return Err("tls_load_cert: invalid context id".to_string());
    }
    
    contexts[ctx_id].cert_path = cert_path.clone();
    contexts[ctx_id].key_path = key_path.clone();
    
    use std::fs::File;
    use std::io::Read as _;
    
    let mut cert_file = File::open(&cert_path)
        .map_err(|e| format!("Failed to open certificate file: {}", e))?;
    let mut cert_data = Vec::new();
    cert_file.read_to_end(&mut cert_data)
        .map_err(|e| format!("Failed to read certificate: {}", e))?;
    
    let mut key_file = File::open(&key_path)
        .map_err(|e| format!("Failed to open key file: {}", e))?;
    let mut key_data = Vec::new();
    key_file.read_to_end(&mut key_data)
        .map_err(|e| format!("Failed to read key: {}", e))?;
    
    let _cert = native_tls::Certificate::from_pem(&cert_data)
        .map_err(|e| format!("Failed to parse certificate: {}", e))?;
    
    let identity = native_tls::Identity::from_pkcs8(&cert_data, &key_data)
        .map_err(|e| format!("Failed to create identity (trying PKCS8): {}", e))
        .or_else(|_| {
            native_tls::Identity::from_pkcs12(&cert_data, "")
                .map_err(|e| format!("Failed to create identity (PKCS12): {}", e))
        })?;
    
    let acceptor = TlsAcceptor::new(identity)
        .map_err(|e| format!("Failed to create TLS acceptor: {}", e))?;
    
    contexts[ctx_id].acceptor = Some(acceptor);
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn tls_wrap_server(args: &[Expr]) -> Result<Expr, String> {
    let ctx_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("tls_wrap_server: first argument must be context id".to_string()),
    };
    
    let socket_fd = match &args[1] {
        Expr::Literal(Literal::Integer(fd)) => *fd,
        _ => return Err("tls_wrap_server: second argument must be socket fd".to_string()),
    };
    
    let contexts = TLS_CONTEXTS.lock().unwrap();
    if ctx_id >= contexts.len() {
        return Err("tls_wrap_server: invalid context id".to_string());
    }
    
    let acceptor = match &contexts[ctx_id].acceptor {
        Some(acc) => acc.clone(),
        None => return Err("tls_wrap_server: TLS acceptor not initialized (call tls_load_cert first)".to_string()),
    };
    
    let tcp_stream = unsafe { &*(socket_fd as *const TcpStream) };
    let cloned_stream = tcp_stream.try_clone()
        .map_err(|e| format!("tls_wrap_server: failed to clone stream: {}", e))?;
    
    drop(contexts);
    
    let tls_stream = acceptor.accept(cloned_stream)
        .map_err(|e| format!("tls_wrap_server: TLS handshake failed: {}", e))?;
    
    let wrapper = TlsStreamWrapper {
        stream: tls_stream,
        alpn_selected: "http/1.1".to_string(),
    };
    
    let mut streams = TLS_STREAMS.lock().unwrap();
    streams.push(wrapper);
    let id = streams.len() - 1;
    
    Ok(Expr::Literal(Literal::Integer(id as i64)))
}

pub fn tls_wrap_client(args: &[Expr]) -> Result<Expr, String> {
    let ctx_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("tls_wrap_client: first argument must be context id".to_string()),
    };
    
    let socket_fd = match &args[1] {
        Expr::Literal(Literal::Integer(fd)) => *fd,
        _ => return Err("tls_wrap_client: second argument must be socket fd".to_string()),
    };
    
    let hostname = match &args[2] {
        Expr::Literal(Literal::String(s)) => s.clone(),
        _ => return Err("tls_wrap_client: third argument must be hostname".to_string()),
    };
    
    let contexts = TLS_CONTEXTS.lock().unwrap();
    if ctx_id >= contexts.len() {
        return Err("tls_wrap_client: invalid context id".to_string());
    }
    
    let connector = match &contexts[ctx_id].connector {
        Some(conn) => conn.clone(),
        None => return Err("tls_wrap_client: TLS connector not initialized".to_string()),
    };
    
    let tcp_stream = unsafe { &*(socket_fd as *const TcpStream) };
    let cloned_stream = tcp_stream.try_clone()
        .map_err(|e| format!("tls_wrap_client: failed to clone stream: {}", e))?;
    
    drop(contexts);
    
    let tls_stream = connector.connect(&hostname, cloned_stream)
        .map_err(|e| format!("tls_wrap_client: TLS handshake failed: {}", e))?;
    
    let wrapper = TlsStreamWrapper {
        stream: tls_stream,
        alpn_selected: "http/1.1".to_string(),
    };
    
    let mut streams = TLS_STREAMS.lock().unwrap();
    streams.push(wrapper);
    let id = streams.len() - 1;
    
    Ok(Expr::Literal(Literal::Integer(id as i64)))
}

pub fn tls_read(args: &[Expr]) -> Result<Expr, String> {
    let tls_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("tls_read: first argument must be TLS stream id".to_string()),
    };
    
    let max_bytes = match &args[1] {
        Expr::Literal(Literal::Integer(n)) => *n as usize,
        _ => return Err("tls_read: second argument must be max bytes".to_string()),
    };
    
    let mut streams = TLS_STREAMS.lock().unwrap();
    if tls_id >= streams.len() {
        return Err("tls_read: invalid TLS stream id".to_string());
    }
    
    let mut buffer = vec![0u8; max_bytes];
    let bytes_read = streams[tls_id].stream.read(&mut buffer)
        .map_err(|e| format!("tls_read: failed to read: {}", e))?;
    
    buffer.truncate(bytes_read);
    let s = String::from_utf8_lossy(&buffer).to_string();
    
    Ok(Expr::Literal(Literal::String(s)))
}

pub fn tls_write(args: &[Expr]) -> Result<Expr, String> {
    let tls_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("tls_write: first argument must be TLS stream id".to_string()),
    };
    
    let data = match &args[1] {
        Expr::Literal(Literal::String(s)) => s.clone(),
        _ => return Err("tls_write: second argument must be data string".to_string()),
    };
    
    let mut streams = TLS_STREAMS.lock().unwrap();
    if tls_id >= streams.len() {
        return Err("tls_write: invalid TLS stream id".to_string());
    }
    
    let bytes_written = streams[tls_id].stream.write(data.as_bytes())
        .map_err(|e| format!("tls_write: failed to write: {}", e))?;
    
    Ok(Expr::Literal(Literal::Integer(bytes_written as i64)))
}

pub fn tls_close(args: &[Expr]) -> Result<Expr, String> {
    let tls_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("tls_close: argument must be TLS stream id".to_string()),
    };
    
    let mut streams = TLS_STREAMS.lock().unwrap();
    if tls_id >= streams.len() {
        return Err("tls_close: invalid TLS stream id".to_string());
    }
    
    let _ = streams[tls_id].stream.shutdown();
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn tls_get_alpn(args: &[Expr]) -> Result<Expr, String> {
    let tls_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("tls_get_alpn: argument must be TLS stream id".to_string()),
    };
    
    let streams = TLS_STREAMS.lock().unwrap();
    if tls_id >= streams.len() {
        return Err("tls_get_alpn: invalid TLS stream id".to_string());
    }
    
    Ok(Expr::Literal(Literal::String(streams[tls_id].alpn_selected.clone())))
}

pub fn tls_set_alpn(args: &[Expr]) -> Result<Expr, String> {
    let ctx_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("tls_set_alpn: first argument must be context id".to_string()),
    };
    
    let protocols = match &args[1] {
        Expr::ArrayLiteral { elements } => {
            let mut protos = Vec::new();
            for elem in elements {
                if let Expr::Literal(Literal::String(s)) = elem {
                    protos.push(s.clone());
                }
            }
            protos
        },
        _ => return Err("tls_set_alpn: second argument must be array of protocol strings".to_string()),
    };
    
    let mut contexts = TLS_CONTEXTS.lock().unwrap();
    if ctx_id >= contexts.len() {
        return Err("tls_set_alpn: invalid context id".to_string());
    }
    
    contexts[ctx_id].alpn_protocols = protocols;
    
    Ok(Expr::Literal(Literal::Null))
}