use crate::ast::{Expr, Literal};
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;
use quinn::{Endpoint, ServerConfig, ClientConfig, Connection, SendStream, RecvStream};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};

lazy_static::lazy_static! {
    static ref QUIC_ENDPOINTS: Arc<Mutex<Vec<QuicEndpoint>>> = Arc::new(Mutex::new(Vec::new()));
    static ref QUIC_CONNECTIONS: Arc<Mutex<Vec<QuicConnectionWrapper>>> = Arc::new(Mutex::new(Vec::new()));
    static ref QUIC_STREAMS: Arc<Mutex<Vec<QuicStreamWrapper>>> = Arc::new(Mutex::new(Vec::new()));
}

pub struct QuicEndpoint {
    pub endpoint: Endpoint,
    pub is_server: bool,
}

pub struct QuicConnectionWrapper {
    pub connection: Connection,
}

pub struct QuicStreamWrapper {
    pub send: Option<SendStream>,
    pub recv: Option<RecvStream>,
}

fn generate_self_signed_cert() -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), String> {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])
        .map_err(|e| format!("Failed to generate certificate: {}", e))?;
    
    let key = PrivateKeyDer::try_from(cert.key_pair.serialize_der())
        .map_err(|e| format!("Failed to serialize private key: {}", e))?;
    
    let cert_der = CertificateDer::from(cert.cert.der().to_vec());
    
    Ok((vec![cert_der], key))
}

fn configure_server() -> Result<ServerConfig, String> {
    let (certs, key) = generate_self_signed_cert()?;
    
    let server_config = ServerConfig::with_single_cert(certs, key)
        .map_err(|e| format!("Failed to create server config: {}", e))?;
    
    Ok(server_config)
}

fn configure_client() -> Result<ClientConfig, String> {
    let mut roots = rustls::RootCertStore::empty();
    
    let certs_result = rustls_native_certs::load_native_certs();
    for cert in certs_result.certs {
        roots.add(cert).map_err(|e| format!("Failed to add cert: {}", e))?;
    }
    
    let mut crypto = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    
    crypto.alpn_protocols = vec![b"h3".to_vec(), b"h3-29".to_vec()];
    
    let client_config = ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(crypto)
            .map_err(|e| format!("Failed to create QUIC client config: {}", e))?
    ));
    
    Ok(client_config)
}

pub fn quic_listen(args: &[Expr]) -> Result<Expr, String> {
    let host = match &args[0] {
        Expr::Literal(Literal::String(s)) => s.clone(),
        _ => return Err("quic_listen: first argument must be host string".to_string()),
    };
    
    let port = match &args[1] {
        Expr::Literal(Literal::Integer(p)) => *p as u16,
        _ => return Err("quic_listen: second argument must be port integer".to_string()),
    };
    
    let _tls_ctx_id = match &args[2] {
        Expr::Literal(Literal::Integer(id)) => *id,
        _ => return Err("quic_listen: third argument must be TLS context id".to_string()),
    };
    
    let server_config = configure_server()?;
    
    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .map_err(|e| format!("quic_listen: invalid address: {}", e))?;
    
    let endpoint = Endpoint::server(server_config, addr)
        .map_err(|e| format!("quic_listen: failed to create endpoint: {}", e))?;
    
    let quic_endpoint = QuicEndpoint {
        endpoint,
        is_server: true,
    };
    
    let mut endpoints = QUIC_ENDPOINTS.lock().unwrap();
    endpoints.push(quic_endpoint);
    let id = endpoints.len() - 1;
    
    Ok(Expr::Literal(Literal::Integer(id as i64)))
}

pub fn quic_accept(args: &[Expr]) -> Result<Expr, String> {
    let listener_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("quic_accept: argument must be listener id".to_string()),
    };
    
    let endpoints = QUIC_ENDPOINTS.lock().unwrap();
    if listener_id >= endpoints.len() {
        return Err("quic_accept: invalid listener id".to_string());
    }
    
    if !endpoints[listener_id].is_server {
        return Err("quic_accept: endpoint is not a server".to_string());
    }
    
    let endpoint = endpoints[listener_id].endpoint.clone();
    drop(endpoints);
    
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("quic_accept: failed to create runtime: {}", e))?;
    
    let connection = runtime.block_on(async {
        match endpoint.accept().await {
            Some(incoming) => {
                incoming.await.map_err(|e| format!("quic_accept: connection failed: {}", e))
            },
            None => Err("quic_accept: no incoming connections".to_string()),
        }
    })?;
    
    let wrapper = QuicConnectionWrapper { connection };
    
    let mut connections = QUIC_CONNECTIONS.lock().unwrap();
    connections.push(wrapper);
    let id = connections.len() - 1;
    
    Ok(Expr::Literal(Literal::Integer(id as i64)))
}

pub fn quic_connect(args: &[Expr]) -> Result<Expr, String> {
    let host = match &args[0] {
        Expr::Literal(Literal::String(s)) => s.clone(),
        _ => return Err("quic_connect: first argument must be host string".to_string()),
    };
    
    let port = match &args[1] {
        Expr::Literal(Literal::Integer(p)) => *p as u16,
        _ => return Err("quic_connect: second argument must be port integer".to_string()),
    };
    
    let client_config = configure_client()?;
    
    let mut endpoint = Endpoint::client("[::]:0".parse().unwrap())
        .map_err(|e| format!("quic_connect: failed to create client endpoint: {}", e))?;
    
    endpoint.set_default_client_config(client_config);
    
    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .map_err(|e| format!("quic_connect: invalid address: {}", e))?;
    
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("quic_connect: failed to create runtime: {}", e))?;
    
    let connection = runtime.block_on(async {
        endpoint.connect(addr, &host)
            .map_err(|e| format!("quic_connect: connect failed: {}", e))?
            .await
            .map_err(|e| format!("quic_connect: connection failed: {}", e))
    })?;
    
    let quic_endpoint = QuicEndpoint {
        endpoint,
        is_server: false,
    };
    
    let mut endpoints = QUIC_ENDPOINTS.lock().unwrap();
    endpoints.push(quic_endpoint);
    drop(endpoints);
    
    let wrapper = QuicConnectionWrapper { connection };
    
    let mut connections = QUIC_CONNECTIONS.lock().unwrap();
    connections.push(wrapper);
    let id = connections.len() - 1;
    
    Ok(Expr::Literal(Literal::Integer(id as i64)))
}

pub fn quic_open_stream(args: &[Expr]) -> Result<Expr, String> {
    let conn_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("quic_open_stream: argument must be connection id".to_string()),
    };
    
    let connections = QUIC_CONNECTIONS.lock().unwrap();
    if conn_id >= connections.len() {
        return Err("quic_open_stream: invalid connection id".to_string());
    }
    
    let connection = connections[conn_id].connection.clone();
    drop(connections);
    
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("quic_open_stream: failed to create runtime: {}", e))?;
    
    let (send, recv) = runtime.block_on(async {
        connection.open_bi()
            .await
            .map_err(|e| format!("quic_open_stream: failed to open stream: {}", e))
    })?;
    
    let wrapper = QuicStreamWrapper {
        send: Some(send),
        recv: Some(recv),
    };
    
    let mut streams = QUIC_STREAMS.lock().unwrap();
    streams.push(wrapper);
    let id = streams.len() - 1;
    
    Ok(Expr::Literal(Literal::Integer(id as i64)))
}

pub fn quic_accept_stream(args: &[Expr]) -> Result<Expr, String> {
    let conn_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("quic_accept_stream: argument must be connection id".to_string()),
    };
    
    let connections = QUIC_CONNECTIONS.lock().unwrap();
    if conn_id >= connections.len() {
        return Err("quic_accept_stream: invalid connection id".to_string());
    }
    
    let connection = connections[conn_id].connection.clone();
    drop(connections);
    
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("quic_accept_stream: failed to create runtime: {}", e))?;
    
    let (send, recv) = runtime.block_on(async {
        match connection.accept_bi().await {
            Ok(stream) => Ok(stream),
            Err(e) => Err(format!("quic_accept_stream: failed to accept stream: {}", e)),
        }
    })?;
    
    let wrapper = QuicStreamWrapper {
        send: Some(send),
        recv: Some(recv),
    };
    
    let mut streams = QUIC_STREAMS.lock().unwrap();
    streams.push(wrapper);
    let id = streams.len() - 1;
    
    Ok(Expr::Literal(Literal::Integer(id as i64)))
}

pub fn quic_stream_send(args: &[Expr]) -> Result<Expr, String> {
    let stream_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("quic_stream_send: first argument must be stream id".to_string()),
    };
    
    let data = match &args[1] {
        Expr::Literal(Literal::String(s)) => s.clone(),
        _ => return Err("quic_stream_send: second argument must be data string".to_string()),
    };
    
    let mut streams = QUIC_STREAMS.lock().unwrap();
    if stream_id >= streams.len() {
        return Err("quic_stream_send: invalid stream id".to_string());
    }
    
    let mut send_stream = streams[stream_id].send.take()
        .ok_or_else(|| "quic_stream_send: send stream not available".to_string())?;
    
    drop(streams);
    
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("quic_stream_send: failed to create runtime: {}", e))?;
    
    send_stream = runtime.block_on(async move {
        send_stream.write_all(data.as_bytes())
            .await
            .map_err(|e| format!("quic_stream_send: write failed: {}", e))?;
        
        send_stream.finish()
            .map_err(|e| format!("quic_stream_send: finish failed: {}", e))?;
        
        Ok::<SendStream, String>(send_stream)
    })?;
    
    let mut streams = QUIC_STREAMS.lock().unwrap();
    if stream_id < streams.len() {
        streams[stream_id].send = Some(send_stream);
    }
    
    Ok(Expr::Literal(Literal::Null))
}

pub fn quic_stream_recv(args: &[Expr]) -> Result<Expr, String> {
    let stream_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("quic_stream_recv: first argument must be stream id".to_string()),
    };
    
    let max_bytes = match &args[1] {
        Expr::Literal(Literal::Integer(n)) => *n as usize,
        _ => return Err("quic_stream_recv: second argument must be max bytes".to_string()),
    };
    
    let mut streams = QUIC_STREAMS.lock().unwrap();
    if stream_id >= streams.len() {
        return Err("quic_stream_recv: invalid stream id".to_string());
    }
    
    let mut recv_stream = streams[stream_id].recv.take()
        .ok_or_else(|| "quic_stream_recv: recv stream not available".to_string())?;
    
    drop(streams);
    
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("quic_stream_recv: failed to create runtime: {}", e))?;
    
    let (data, recv_stream_result) = runtime.block_on(async move {
        let mut buffer = vec![0u8; max_bytes];
        
        match recv_stream.read(&mut buffer).await {
            Ok(Some(n)) => {
                buffer.truncate(n);
                Ok((buffer, recv_stream))
            },
            Ok(None) => Ok((Vec::new(), recv_stream)),
            Err(e) => Err(format!("quic_stream_recv: read failed: {}", e)),
        }
    })?;
    
    let text = String::from_utf8_lossy(&data).to_string();
    
    let mut streams = QUIC_STREAMS.lock().unwrap();
    if stream_id < streams.len() {
        streams[stream_id].recv = Some(recv_stream_result);
    }
    
    Ok(Expr::Literal(Literal::String(text)))
}

pub fn quic_close(args: &[Expr]) -> Result<Expr, String> {
    let conn_id = match &args[0] {
        Expr::Literal(Literal::Integer(id)) => *id as usize,
        _ => return Err("quic_close: argument must be connection id".to_string()),
    };
    
    let connections = QUIC_CONNECTIONS.lock().unwrap();
    if conn_id >= connections.len() {
        return Err("quic_close: invalid connection id".to_string());
    }
    
    let connection = connections[conn_id].connection.clone();
    drop(connections);
    
    connection.close(0u32.into(), b"closed by application");
    
    Ok(Expr::Literal(Literal::Null))
}