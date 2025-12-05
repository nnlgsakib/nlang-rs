use crate::ast::{Expr, Literal};
use std::net::TcpStream;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use sha1::{Sha1, Digest};
use base64::{Engine as _, engine::general_purpose};

lazy_static::lazy_static! {
    static ref WEBSOCKETS: Arc<Mutex<Vec<WebSocketConnection>>> = Arc::new(Mutex::new(Vec::new()));
}

const WEBSOCKET_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OpCode {
    Continuation = 0x0,
    Text = 0x1,
    Binary = 0x2,
    Close = 0x8,
    Ping = 0x9,
    Pong = 0xA,
}

pub struct WebSocketConnection {
    pub stream: TcpStream,
    pub is_client: bool,
    pub connected: bool,
}

#[allow(dead_code)]
pub struct WebSocketFrame {
    pub fin: bool,
    pub opcode: OpCode,
    pub masked: bool,
    pub payload: Vec<u8>,
}

fn compute_websocket_accept(key: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update(WEBSOCKET_GUID.as_bytes());
    let result = hasher.finalize();
    general_purpose::STANDARD.encode(result)
}

fn write_websocket_frame(stream: &mut TcpStream, opcode: OpCode, payload: &[u8], mask: bool) -> Result<(), String> {
    let mut frame = Vec::new();
    
    frame.push(0x80 | (opcode as u8));
    
    let payload_len = payload.len();
    if payload_len < 126 {
        let mut len_byte = payload_len as u8;
        if mask {
            len_byte |= 0x80;
        }
        frame.push(len_byte);
    } else if payload_len < 65536 {
        let mut len_byte = 126u8;
        if mask {
            len_byte |= 0x80;
        }
        frame.push(len_byte);
        frame.push((payload_len >> 8) as u8);
        frame.push((payload_len & 0xFF) as u8);
    } else {
        let mut len_byte = 127u8;
        if mask {
            len_byte |= 0x80;
        }
        frame.push(len_byte);
        for i in (0..8).rev() {
            frame.push(((payload_len >> (i * 8)) & 0xFF) as u8);
        }
    }
    
    if mask {
        let mask_key: [u8; 4] = rand::random();
        frame.extend_from_slice(&mask_key);
        
        let mut masked_payload = payload.to_vec();
        for (i, byte) in masked_payload.iter_mut().enumerate() {
            *byte ^= mask_key[i % 4];
        }
        frame.extend_from_slice(&masked_payload);
    } else {
        frame.extend_from_slice(payload);
    }
    
    stream.write_all(&frame)
        .map_err(|e| format!("Failed to write WebSocket frame: {}", e))
}

fn read_websocket_frame(stream: &mut TcpStream) -> Result<WebSocketFrame, String> {
    let mut header = [0u8; 2];
    stream.read_exact(&mut header)
        .map_err(|e| format!("Failed to read WebSocket frame header: {}", e))?;
    
    let fin = (header[0] & 0x80) != 0;
    let opcode_raw = header[0] & 0x0F;
    let masked = (header[1] & 0x80) != 0;
    let mut payload_len = (header[1] & 0x7F) as u64;
    
    if payload_len == 126 {
        let mut len_bytes = [0u8; 2];
        stream.read_exact(&mut len_bytes)
            .map_err(|e| format!("Failed to read extended payload length: {}", e))?;
        payload_len = u16::from_be_bytes(len_bytes) as u64;
    } else if payload_len == 127 {
        let mut len_bytes = [0u8; 8];
        stream.read_exact(&mut len_bytes)
            .map_err(|e| format!("Failed to read extended payload length: {}", e))?;
        payload_len = u64::from_be_bytes(len_bytes);
    }
    
    let mask_key = if masked {
        let mut key = [0u8; 4];
        stream.read_exact(&mut key)
            .map_err(|e| format!("Failed to read mask key: {}", e))?;
        Some(key)
    } else {
        None
    };
    
    let mut payload = vec![0u8; payload_len as usize];
    stream.read_exact(&mut payload)
        .map_err(|e| format!("Failed to read payload: {}", e))?;
    
    if let Some(key) = mask_key {
        for (i, byte) in payload.iter_mut().enumerate() {
            *byte ^= key[i % 4];
        }
    }
    
    let opcode = match opcode_raw {
        0x0 => OpCode::Continuation,
        0x1 => OpCode::Text,
        0x2 => OpCode::Binary,
        0x8 => OpCode::Close,
        0x9 => OpCode::Ping,
        0xA => OpCode::Pong,
        _ => return Err(format!("Unknown WebSocket opcode: {}", opcode_raw)),
    };
    
    Ok(WebSocketFrame {
        fin,
        opcode,
        masked,
        payload,
    })
}

pub fn websocket_accept(args: &[Expr]) -> Result<Expr, String> {
    let req_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("websocket_accept: argument must be request id".to_string()),
    };
    
    use super::http_server::REQUESTS;
    let requests = REQUESTS.lock().unwrap();
    if req_id >= requests.len() {
        return Err("websocket_accept: invalid request id".to_string());
    }
    
    let ws_key = requests[req_id].headers.get("sec-websocket-key")
        .ok_or_else(|| "websocket_accept: missing Sec-WebSocket-Key header".to_string())?;
    
    let accept_key = compute_websocket_accept(ws_key);
    
    let socket_fd = requests[req_id].socket_fd;
    drop(requests);
    
    let stream = unsafe { &mut *(socket_fd as *mut TcpStream) };
    
    let handshake_response = format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {}\r\n\
         \r\n",
        accept_key
    );
    
    stream.write_all(handshake_response.as_bytes())
        .map_err(|e| format!("websocket_accept: failed to send handshake: {}", e))?;
    
    let cloned_stream = stream.try_clone()
        .map_err(|e| format!("websocket_accept: failed to clone stream: {}", e))?;
    
    let connection = WebSocketConnection {
        stream: cloned_stream,
        is_client: false,
        connected: true,
    };
    
    let mut websockets = WEBSOCKETS.lock().unwrap();
    websockets.push(connection);
    let ws_id = websockets.len() - 1;
    
    Ok(Expr::Literal(Literal::Integer(ws_id as i64)))
}

pub fn websocket_connect(args: &[Expr]) -> Result<Expr, String> {
    let url = match &args[0] {
        Expr::Literal(Literal::String(s)) => s,
        _ => return Err("websocket_connect: argument must be URL string".to_string()),
    };
    
    let url = url.trim_start_matches("ws://").trim_start_matches("wss://");
    let parts: Vec<&str> = url.splitn(2, '/').collect();
    let host_port = parts[0];
    let path = if parts.len() > 1 { format!("/{}", parts[1]) } else { "/".to_string() };
    
    let (host, port) = if let Some(colon_pos) = host_port.find(':') {
        let (h, p) = host_port.split_at(colon_pos);
        (h.to_string(), p[1..].parse::<u16>().unwrap_or(80))
    } else {
        (host_port.to_string(), 80)
    };
    
    let mut stream = TcpStream::connect(format!("{}:{}", host, port))
        .map_err(|e| format!("websocket_connect: failed to connect: {}", e))?;
    
    let key = general_purpose::STANDARD.encode(rand::random::<[u8; 16]>());
    
    let handshake = format!(
        "GET {} HTTP/1.1\r\n\
         Host: {}\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Key: {}\r\n\
         Sec-WebSocket-Version: 13\r\n\
         \r\n",
        path, host, key
    );
    
    stream.write_all(handshake.as_bytes())
        .map_err(|e| format!("websocket_connect: failed to send handshake: {}", e))?;
    
    let mut response = Vec::new();
    let mut buf = [0u8; 1];
    loop {
        stream.read_exact(&mut buf)
            .map_err(|e| format!("websocket_connect: failed to read response: {}", e))?;
        response.push(buf[0]);
        if response.len() >= 4 && &response[response.len()-4..] == b"\r\n\r\n" {
            break;
        }
    }
    
    let response_str = String::from_utf8_lossy(&response);
    if !response_str.contains("101") {
        return Err("websocket_connect: server did not accept WebSocket upgrade".to_string());
    }
    
    let connection = WebSocketConnection {
        stream,
        is_client: true,
        connected: true,
    };
    
    let mut websockets = WEBSOCKETS.lock().unwrap();
    websockets.push(connection);
    let ws_id = websockets.len() - 1;
    
    Ok(Expr::Literal(Literal::Integer(ws_id as i64)))
}

pub fn websocket_send_text(args: &[Expr]) -> Result<Expr, String> {
    let ws_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("websocket_send_text: first argument must be WebSocket id".to_string()),
    };
    
    let text = match &args[1] {
        Expr::Literal(Literal::String(s)) => s,
        _ => return Err("websocket_send_text: second argument must be text string".to_string()),
    };
    
    let mut websockets = WEBSOCKETS.lock().unwrap();
    if ws_id >= websockets.len() {
        return Err("websocket_send_text: invalid WebSocket id".to_string());
    }
    
    if !websockets[ws_id].connected {
        return Err("websocket_send_text: WebSocket is not connected".to_string());
    }
    
    let is_client = websockets[ws_id].is_client;
    write_websocket_frame(&mut websockets[ws_id].stream, OpCode::Text, text.as_bytes(), is_client)?;
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn websocket_send_binary(args: &[Expr]) -> Result<Expr, String> {
    let ws_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("websocket_send_binary: first argument must be WebSocket id".to_string()),
    };
    
    let data = match &args[1] {
        Expr::ArrayLiteral { elements } => {
            let mut bytes = Vec::new();
            for elem in elements {
                if let Expr::Literal(Literal::Integer(b)) = elem {
                    bytes.push(*b as u8);
                }
            }
            bytes
        },
        _ => return Err("websocket_send_binary: second argument must be byte array".to_string()),
    };
    
    let mut websockets = WEBSOCKETS.lock().unwrap();
    if ws_id >= websockets.len() {
        return Err("websocket_send_binary: invalid WebSocket id".to_string());
    }
    
    if !websockets[ws_id].connected {
        return Err("websocket_send_binary: WebSocket is not connected".to_string());
    }
    
    let is_client = websockets[ws_id].is_client;
    write_websocket_frame(&mut websockets[ws_id].stream, OpCode::Binary, &data, is_client)?;
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn websocket_recv(args: &[Expr]) -> Result<Expr, String> {
    let ws_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("websocket_recv: argument must be WebSocket id".to_string()),
    };
    
    let mut websockets = WEBSOCKETS.lock().unwrap();
    if ws_id >= websockets.len() {
        return Err("websocket_recv: invalid WebSocket id".to_string());
    }
    
    if !websockets[ws_id].connected {
        return Err("websocket_recv: WebSocket is not connected".to_string());
    }
    
    let frame = read_websocket_frame(&mut websockets[ws_id].stream)?;
    
    match frame.opcode {
        OpCode::Text => {
            let text = String::from_utf8_lossy(&frame.payload).to_string();
            Ok(Expr::Literal(Literal::String(text)))
        },
        OpCode::Binary => {
            let text = String::from_utf8_lossy(&frame.payload).to_string();
            Ok(Expr::Literal(Literal::String(text)))
        },
        OpCode::Close => {
            websockets[ws_id].connected = false;
            Ok(Expr::Literal(Literal::Null))
        },
        OpCode::Ping => {
            let is_client = websockets[ws_id].is_client;
            write_websocket_frame(&mut websockets[ws_id].stream, OpCode::Pong, &frame.payload, is_client)?;
            websocket_recv(args)
        },
        OpCode::Pong => {
            websocket_recv(args)
        },
        _ => Ok(Expr::Literal(Literal::Null)),
    }
}

pub fn websocket_close(args: &[Expr]) -> Result<Expr, String> {
    let ws_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("websocket_close: first argument must be WebSocket id".to_string()),
    };
    
    let code = match &args[1] {
        Expr::Literal(Literal::Integer(c)) => *c as u16,
        _ => 1000u16,
    };
    
    let reason = match &args[2] {
        Expr::Literal(Literal::String(s)) => s.as_bytes(),
        _ => b"",
    };
    
    let mut websockets = WEBSOCKETS.lock().unwrap();
    if ws_id >= websockets.len() {
        return Err("websocket_close: invalid WebSocket id".to_string());
    }
    
    let mut close_payload = Vec::new();
    close_payload.push((code >> 8) as u8);
    close_payload.push((code & 0xFF) as u8);
    close_payload.extend_from_slice(reason);
    
    let is_client = websockets[ws_id].is_client;
    let _ = write_websocket_frame(&mut websockets[ws_id].stream, OpCode::Close, &close_payload, is_client);
    
    websockets[ws_id].connected = false;
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn websocket_ping(args: &[Expr]) -> Result<Expr, String> {
    let ws_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("websocket_ping: argument must be WebSocket id".to_string()),
    };
    
    let mut websockets = WEBSOCKETS.lock().unwrap();
    if ws_id >= websockets.len() {
        return Err("websocket_ping: invalid WebSocket id".to_string());
    }
    
    if !websockets[ws_id].connected {
        return Err("websocket_ping: WebSocket is not connected".to_string());
    }
    
    let is_client = websockets[ws_id].is_client;
    write_websocket_frame(&mut websockets[ws_id].stream, OpCode::Ping, &[], is_client)?;
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn websocket_pong(args: &[Expr]) -> Result<Expr, String> {
    let ws_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("websocket_pong: argument must be WebSocket id".to_string()),
    };
    
    let mut websockets = WEBSOCKETS.lock().unwrap();
    if ws_id >= websockets.len() {
        return Err("websocket_pong: invalid WebSocket id".to_string());
    }
    
    if !websockets[ws_id].connected {
        return Err("websocket_pong: WebSocket is not connected".to_string());
    }
    
    let is_client = websockets[ws_id].is_client;
    write_websocket_frame(&mut websockets[ws_id].stream, OpCode::Pong, &[], is_client)?;
    
    Ok(Expr::Literal(Literal::Null))
}