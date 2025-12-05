use crate::ast::{Expr, Literal};
use std::net::{TcpListener, TcpStream, Shutdown};
use std::io::{Read, Write};
use std::time::Duration;

pub fn tcp_listen(args: &[Expr]) -> Result<Expr, String> {
    let host = match &args[0] {
        Expr::Literal(Literal::String(s)) => s.clone(),
        _ => return Err("tcp_listen: first argument must be a string (host)".to_string()),
    };
    
    let port = match &args[1] {
        Expr::Literal(Literal::Integer(p)) => *p,
        _ => return Err("tcp_listen: second argument must be an integer (port)".to_string()),
    };
    
    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr)
        .map_err(|e| format!("tcp_listen: failed to bind to {}: {}", addr, e))?;
    
    let fd = Box::into_raw(Box::new(listener)) as i64;
    Ok(Expr::Literal(Literal::Integer(fd)))
}

pub fn tcp_accept(args: &[Expr]) -> Result<Expr, String> {
    let listener_fd = match &args[0] {
        Expr::Literal(Literal::Integer(fd)) => *fd,
        _ => return Err("tcp_accept: argument must be an integer (listener fd)".to_string()),
    };
    
    let listener = unsafe { &*(listener_fd as *const TcpListener) };
    
    match listener.accept() {
        Ok((stream, _addr)) => {
            let fd = Box::into_raw(Box::new(stream)) as i64;
            Ok(Expr::Literal(Literal::Integer(fd)))
        },
        Err(e) => Err(format!("tcp_accept: failed to accept connection: {}", e)),
    }
}

pub fn tcp_connect(args: &[Expr]) -> Result<Expr, String> {
    let host = match &args[0] {
        Expr::Literal(Literal::String(s)) => s.clone(),
        _ => return Err("tcp_connect: first argument must be a string (host)".to_string()),
    };
    
    let port = match &args[1] {
        Expr::Literal(Literal::Integer(p)) => *p,
        _ => return Err("tcp_connect: second argument must be an integer (port)".to_string()),
    };
    
    let addr = format!("{}:{}", host, port);
    let stream = TcpStream::connect(&addr)
        .map_err(|e| format!("tcp_connect: failed to connect to {}: {}", addr, e))?;
    
    let fd = Box::into_raw(Box::new(stream)) as i64;
    Ok(Expr::Literal(Literal::Integer(fd)))
}

pub fn tcp_read(args: &[Expr]) -> Result<Expr, String> {
    let stream_fd = match &args[0] {
        Expr::Literal(Literal::Integer(fd)) => *fd,
        _ => return Err("tcp_read: first argument must be an integer (stream fd)".to_string()),
    };
    
    let max_bytes = match &args[1] {
        Expr::Literal(Literal::Integer(n)) => *n as usize,
        _ => return Err("tcp_read: second argument must be an integer (max bytes)".to_string()),
    };
    
    let stream = unsafe { &mut *(stream_fd as *mut TcpStream) };
    let mut buffer = vec![0u8; max_bytes];
    
    match stream.read(&mut buffer) {
        Ok(n) => {
            buffer.truncate(n);
            let s = String::from_utf8_lossy(&buffer).to_string();
            Ok(Expr::Literal(Literal::String(s)))
        },
        Err(e) => Err(format!("tcp_read: failed to read from stream: {}", e)),
    }
}

pub fn tcp_write(args: &[Expr]) -> Result<Expr, String> {
    let stream_fd = match &args[0] {
        Expr::Literal(Literal::Integer(fd)) => *fd,
        _ => return Err("tcp_write: first argument must be an integer (stream fd)".to_string()),
    };
    
    let data = match &args[1] {
        Expr::Literal(Literal::String(s)) => s.clone(),
        _ => return Err("tcp_write: second argument must be a string (data)".to_string()),
    };
    
    let stream = unsafe { &mut *(stream_fd as *mut TcpStream) };
    
    match stream.write(data.as_bytes()) {
        Ok(n) => Ok(Expr::Literal(Literal::Integer(n as i64))),
        Err(e) => Err(format!("tcp_write: failed to write to stream: {}", e)),
    }
}

pub fn tcp_close(args: &[Expr]) -> Result<Expr, String> {
    let stream_fd = match &args[0] {
        Expr::Literal(Literal::Integer(fd)) => *fd,
        _ => return Err("tcp_close: argument must be an integer (stream fd)".to_string()),
    };
    
    let _ = unsafe { Box::from_raw(stream_fd as *mut TcpStream) };
    Ok(Expr::Literal(Literal::Null))
}

pub fn tcp_shutdown(args: &[Expr]) -> Result<Expr, String> {
    let stream_fd = match &args[0] {
        Expr::Literal(Literal::Integer(fd)) => *fd,
        _ => return Err("tcp_shutdown: first argument must be an integer (stream fd)".to_string()),
    };
    
    let how = match &args[1] {
        Expr::Literal(Literal::Integer(h)) => *h,
        _ => return Err("tcp_shutdown: second argument must be an integer (0=read, 1=write, 2=both)".to_string()),
    };
    
    let stream = unsafe { &*(stream_fd as *const TcpStream) };
    
    let shutdown_mode = match how {
        0 => Shutdown::Read,
        1 => Shutdown::Write,
        _ => Shutdown::Both,
    };
    
    stream.shutdown(shutdown_mode)
        .map_err(|e| format!("tcp_shutdown: failed to shutdown: {}", e))?;
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn tcp_peer_addr(args: &[Expr]) -> Result<Expr, String> {
    let stream_fd = match &args[0] {
        Expr::Literal(Literal::Integer(fd)) => *fd,
        _ => return Err("tcp_peer_addr: argument must be an integer (stream fd)".to_string()),
    };
    
    let stream = unsafe { &*(stream_fd as *const TcpStream) };
    
    match stream.peer_addr() {
        Ok(addr) => Ok(Expr::Literal(Literal::String(addr.to_string()))),
        Err(e) => Err(format!("tcp_peer_addr: failed to get peer address: {}", e)),
    }
}

pub fn tcp_local_addr(args: &[Expr]) -> Result<Expr, String> {
    let stream_fd = match &args[0] {
        Expr::Literal(Literal::Integer(fd)) => *fd,
        _ => return Err("tcp_local_addr: argument must be an integer (stream fd)".to_string()),
    };
    
    let stream = unsafe { &*(stream_fd as *const TcpStream) };
    
    match stream.local_addr() {
        Ok(addr) => Ok(Expr::Literal(Literal::String(addr.to_string()))),
        Err(e) => Err(format!("tcp_local_addr: failed to get local address: {}", e)),
    }
}

pub fn tcp_set_nodelay(args: &[Expr]) -> Result<Expr, String> {
    let stream_fd = match &args[0] {
        Expr::Literal(Literal::Integer(fd)) => *fd,
        _ => return Err("tcp_set_nodelay: first argument must be an integer (stream fd)".to_string()),
    };
    
    let enable = match &args[1] {
        Expr::Literal(Literal::Boolean(b)) => *b,
        _ => return Err("tcp_set_nodelay: second argument must be a boolean".to_string()),
    };
    
    let stream = unsafe { &*(stream_fd as *const TcpStream) };
    
    stream.set_nodelay(enable)
        .map_err(|e| format!("tcp_set_nodelay: failed: {}", e))?;
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn tcp_set_reuseaddr(_args: &[Expr]) -> Result<Expr, String> {
    Ok(Expr::Literal(Literal::Null))
}

pub fn tcp_set_keepalive(_args: &[Expr]) -> Result<Expr, String> {
    Ok(Expr::Literal(Literal::Null))
}

pub fn tcp_set_timeout(args: &[Expr]) -> Result<Expr, String> {
    let stream_fd = match &args[0] {
        Expr::Literal(Literal::Integer(fd)) => *fd,
        _ => return Err("tcp_set_timeout: first argument must be an integer (stream fd)".to_string()),
    };
    
    let timeout_ms = match &args[1] {
        Expr::Literal(Literal::Integer(ms)) => *ms as u64,
        _ => return Err("tcp_set_timeout: second argument must be an integer (milliseconds)".to_string()),
    };
    
    let stream = unsafe { &*(stream_fd as *const TcpStream) };
    let timeout = Duration::from_millis(timeout_ms);
    
    stream.set_read_timeout(Some(timeout))
        .map_err(|e| format!("tcp_set_timeout: failed to set read timeout: {}", e))?;
    stream.set_write_timeout(Some(timeout))
        .map_err(|e| format!("tcp_set_timeout: failed to set write timeout: {}", e))?;
    
    Ok(Expr::Literal(Literal::Null))
}