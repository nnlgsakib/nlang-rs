use crate::ast::{Expr, Literal};
use std::net::UdpSocket;

pub fn udp_bind(args: &[Expr]) -> Result<Expr, String> {
    let host = match &args[0] {
        Expr::Literal(Literal::String(s)) => s.clone(),
        _ => return Err("udp_bind: first argument must be a string (host)".to_string()),
    };
    
    let port = match &args[1] {
        Expr::Literal(Literal::Integer(p)) => *p,
        _ => return Err("udp_bind: second argument must be an integer (port)".to_string()),
    };
    
    let addr = format!("{}:{}", host, port);
    let socket = UdpSocket::bind(&addr)
        .map_err(|e| format!("udp_bind: failed to bind to {}: {}", addr, e))?;
    
    let fd = Box::into_raw(Box::new(socket)) as i64;
    Ok(Expr::Literal(Literal::Integer(fd)))
}

pub fn udp_send_to(args: &[Expr]) -> Result<Expr, String> {
    let socket_fd = match &args[0] {
        Expr::Literal(Literal::Integer(fd)) => *fd,
        _ => return Err("udp_send_to: first argument must be an integer (socket fd)".to_string()),
    };
    
    let data = match &args[1] {
        Expr::Literal(Literal::String(s)) => s.clone(),
        _ => return Err("udp_send_to: second argument must be a string (data)".to_string()),
    };
    
    let host = match &args[2] {
        Expr::Literal(Literal::String(s)) => s.clone(),
        _ => return Err("udp_send_to: third argument must be a string (host)".to_string()),
    };
    
    let port = match &args[3] {
        Expr::Literal(Literal::Integer(p)) => *p,
        _ => return Err("udp_send_to: fourth argument must be an integer (port)".to_string()),
    };
    
    let socket = unsafe { &*(socket_fd as *const UdpSocket) };
    let addr = format!("{}:{}", host, port);
    
    match socket.send_to(data.as_bytes(), &addr) {
        Ok(n) => Ok(Expr::Literal(Literal::Integer(n as i64))),
        Err(e) => Err(format!("udp_send_to: failed to send: {}", e)),
    }
}

pub fn udp_recv_from(args: &[Expr]) -> Result<Expr, String> {
    let socket_fd = match &args[0] {
        Expr::Literal(Literal::Integer(fd)) => *fd,
        _ => return Err("udp_recv_from: first argument must be an integer (socket fd)".to_string()),
    };
    
    let max_bytes = match &args[1] {
        Expr::Literal(Literal::Integer(n)) => *n as usize,
        _ => return Err("udp_recv_from: second argument must be an integer (max bytes)".to_string()),
    };
    
    let socket = unsafe { &*(socket_fd as *const UdpSocket) };
    let mut buffer = vec![0u8; max_bytes];
    
    match socket.recv_from(&mut buffer) {
        Ok((n, _addr)) => {
            buffer.truncate(n);
            let s = String::from_utf8_lossy(&buffer).to_string();
            Ok(Expr::Literal(Literal::String(s)))
        },
        Err(e) => Err(format!("udp_recv_from: failed to receive: {}", e)),
    }
}

pub fn udp_connect(args: &[Expr]) -> Result<Expr, String> {
    let socket_fd = match &args[0] {
        Expr::Literal(Literal::Integer(fd)) => *fd,
        _ => return Err("udp_connect: first argument must be an integer (socket fd)".to_string()),
    };
    
    let host = match &args[1] {
        Expr::Literal(Literal::String(s)) => s.clone(),
        _ => return Err("udp_connect: second argument must be a string (host)".to_string()),
    };
    
    let port = match &args[2] {
        Expr::Literal(Literal::Integer(p)) => *p,
        _ => return Err("udp_connect: third argument must be an integer (port)".to_string()),
    };
    
    let socket = unsafe { &*(socket_fd as *const UdpSocket) };
    let addr = format!("{}:{}", host, port);
    
    socket.connect(&addr)
        .map_err(|e| format!("udp_connect: failed to connect to {}: {}", addr, e))?;
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn udp_close(args: &[Expr]) -> Result<Expr, String> {
    let socket_fd = match &args[0] {
        Expr::Literal(Literal::Integer(fd)) => *fd,
        _ => return Err("udp_close: argument must be an integer (socket fd)".to_string()),
    };
    
    let _ = unsafe { Box::from_raw(socket_fd as *mut UdpSocket) };
    Ok(Expr::Literal(Literal::Null))
}