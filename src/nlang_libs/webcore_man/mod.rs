mod tcp;
mod udp;
mod tls;
mod http_server;
mod websocket;
mod http_utils;
mod router;
mod middleware;
mod sse;
mod quic;

use crate::ast::Type;
use crate::nlang_libs::common::LibraryDefinition;

pub fn create_webcore_man_lib() -> LibraryDefinition {
    let mut lib = LibraryDefinition::new("webcore_man");
    
    lib.add_function("tcp_listen", vec![Type::String, Type::Integer], Type::Integer, tcp::tcp_listen);
    lib.add_function("tcp_accept", vec![Type::Integer], Type::Integer, tcp::tcp_accept);
    lib.add_function("tcp_connect", vec![Type::String, Type::Integer], Type::Integer, tcp::tcp_connect);
    lib.add_function("tcp_read", vec![Type::Integer, Type::Integer], Type::String, tcp::tcp_read);
    lib.add_function("tcp_write", vec![Type::Integer, Type::String], Type::Integer, tcp::tcp_write);
    lib.add_function("tcp_close", vec![Type::Integer], Type::Void, tcp::tcp_close);
    lib.add_function("tcp_shutdown", vec![Type::Integer, Type::Integer], Type::Void, tcp::tcp_shutdown);
    lib.add_function("tcp_peer_addr", vec![Type::Integer], Type::String, tcp::tcp_peer_addr);
    lib.add_function("tcp_local_addr", vec![Type::Integer], Type::String, tcp::tcp_local_addr);
    lib.add_function("tcp_set_nodelay", vec![Type::Integer, Type::Boolean], Type::Void, tcp::tcp_set_nodelay);
    lib.add_function("tcp_set_reuseaddr", vec![Type::Integer, Type::Boolean], Type::Void, tcp::tcp_set_reuseaddr);
    lib.add_function("tcp_set_keepalive", vec![Type::Integer, Type::Boolean], Type::Void, tcp::tcp_set_keepalive);
    lib.add_function("tcp_set_timeout", vec![Type::Integer, Type::Integer], Type::Void, tcp::tcp_set_timeout);
    
    lib.add_function("udp_bind", vec![Type::String, Type::Integer], Type::Integer, udp::udp_bind);
    lib.add_function("udp_send_to", vec![Type::Integer, Type::String, Type::String, Type::Integer], Type::Integer, udp::udp_send_to);
    lib.add_function("udp_recv_from", vec![Type::Integer, Type::Integer], Type::String, udp::udp_recv_from);
    lib.add_function("udp_connect", vec![Type::Integer, Type::String, Type::Integer], Type::Void, udp::udp_connect);
    lib.add_function("udp_close", vec![Type::Integer], Type::Void, udp::udp_close);
    
    lib.add_function("tls_context_new", vec![], Type::Integer, tls::tls_context_new);
    lib.add_function("tls_load_cert", vec![Type::Integer, Type::String, Type::String], Type::Void, tls::tls_load_cert);
    lib.add_function("tls_wrap_server", vec![Type::Integer, Type::Integer], Type::Integer, tls::tls_wrap_server);
    lib.add_function("tls_wrap_client", vec![Type::Integer, Type::Integer, Type::String], Type::Integer, tls::tls_wrap_client);
    lib.add_function("tls_read", vec![Type::Integer, Type::Integer], Type::String, tls::tls_read);
    lib.add_function("tls_write", vec![Type::Integer, Type::String], Type::Integer, tls::tls_write);
    lib.add_function("tls_close", vec![Type::Integer], Type::Void, tls::tls_close);
    lib.add_function("tls_get_alpn", vec![Type::Integer], Type::String, tls::tls_get_alpn);
    lib.add_function("tls_set_alpn", vec![Type::Integer, Type::Array(Box::new(Type::String), 0)], Type::Void, tls::tls_set_alpn);
    
    lib.add_function("http_server_new", vec![], Type::Integer, http_server::http_server_new);
    lib.add_function("http_server_bind", vec![Type::Integer, Type::String, Type::Integer], Type::Void, http_server::http_server_bind);
    lib.add_function("http_server_enable_http1", vec![Type::Integer, Type::Boolean], Type::Void, http_server::http_server_enable_http1);
    lib.add_function("http_server_enable_http2", vec![Type::Integer, Type::Boolean], Type::Void, http_server::http_server_enable_http2);
    lib.add_function("http_server_enable_http3", vec![Type::Integer, Type::Boolean], Type::Void, http_server::http_server_enable_http3);
    lib.add_function("http_server_enable_websocket", vec![Type::Integer, Type::Boolean], Type::Void, http_server::http_server_enable_websocket);
    lib.add_function("http_server_set_max_connections", vec![Type::Integer, Type::Integer], Type::Void, http_server::http_server_set_max_connections);
    lib.add_function("http_server_set_tls", vec![Type::Integer, Type::Integer], Type::Void, http_server::http_server_set_tls);
    lib.add_function("http_server_start", vec![Type::Integer], Type::Void, http_server::http_server_start);
    lib.add_function("http_server_stop", vec![Type::Integer], Type::Void, http_server::http_server_stop);
    lib.add_function("http_server_accept", vec![Type::Integer], Type::Integer, http_server::http_server_accept);
    
    lib.add_function("http_request_get_method", vec![Type::Integer], Type::String, http_server::http_request_get_method);
    lib.add_function("http_request_get_uri", vec![Type::Integer], Type::String, http_server::http_request_get_uri);
    lib.add_function("http_request_get_version", vec![Type::Integer], Type::String, http_server::http_request_get_version);
    lib.add_function("http_request_get_header", vec![Type::Integer, Type::String], Type::String, http_server::http_request_get_header);
    lib.add_function("http_request_get_body", vec![Type::Integer], Type::String, http_server::http_request_get_body);
    lib.add_function("http_request_get_remote_addr", vec![Type::Integer], Type::String, http_server::http_request_get_remote_addr);
    lib.add_function("http_request_is_websocket", vec![Type::Integer], Type::Boolean, http_server::http_request_is_websocket);
    lib.add_function("http_request_free", vec![Type::Integer], Type::Void, http_server::http_request_free);
    
    lib.add_function("http_response_new", vec![], Type::Integer, http_server::http_response_new);
    lib.add_function("http_response_set_status", vec![Type::Integer, Type::Integer], Type::Void, http_server::http_response_set_status);
    lib.add_function("http_response_set_header", vec![Type::Integer, Type::String, Type::String], Type::Void, http_server::http_response_set_header);
    lib.add_function("http_response_set_body", vec![Type::Integer, Type::String], Type::Void, http_server::http_response_set_body);
    lib.add_function("http_response_set_file", vec![Type::Integer, Type::String], Type::Void, http_server::http_response_set_file);
    lib.add_function("http_response_send", vec![Type::Integer, Type::Integer], Type::Void, http_server::http_response_send);
    lib.add_function("http_response_free", vec![Type::Integer], Type::Void, http_server::http_response_free);
    
    lib.add_function("websocket_accept", vec![Type::Integer], Type::Integer, websocket::websocket_accept);
    lib.add_function("websocket_connect", vec![Type::String], Type::Integer, websocket::websocket_connect);
    lib.add_function("websocket_send_text", vec![Type::Integer, Type::String], Type::Void, websocket::websocket_send_text);
    lib.add_function("websocket_send_binary", vec![Type::Integer, Type::Array(Box::new(Type::U8), 0)], Type::Void, websocket::websocket_send_binary);
    lib.add_function("websocket_recv", vec![Type::Integer], Type::String, websocket::websocket_recv);
    lib.add_function("websocket_close", vec![Type::Integer, Type::Integer, Type::String], Type::Void, websocket::websocket_close);
    lib.add_function("websocket_ping", vec![Type::Integer], Type::Void, websocket::websocket_ping);
    lib.add_function("websocket_pong", vec![Type::Integer], Type::Void, websocket::websocket_pong);
    
    lib.add_function("http_parse_query", vec![Type::String], Type::Array(Box::new(Type::String), 0), http_utils::http_parse_query);
    lib.add_function("http_url_encode", vec![Type::String], Type::String, http_utils::http_url_encode);
    lib.add_function("http_url_decode", vec![Type::String], Type::String, http_utils::http_url_decode);
    lib.add_function("http_parse_cookie", vec![Type::String], Type::Array(Box::new(Type::String), 0), http_utils::http_parse_cookie);
    lib.add_function("http_build_cookie", vec![Type::String, Type::String, Type::Integer], Type::String, http_utils::http_build_cookie);
    
    lib.add_function("router_new", vec![], Type::Integer, router::router_new);
    lib.add_function("router_get", vec![Type::Integer, Type::String], Type::Void, router::router_get);
    lib.add_function("router_post", vec![Type::Integer, Type::String], Type::Void, router::router_post);
    lib.add_function("router_put", vec![Type::Integer, Type::String], Type::Void, router::router_put);
    lib.add_function("router_delete", vec![Type::Integer, Type::String], Type::Void, router::router_delete);
    lib.add_function("router_match", vec![Type::Integer, Type::String, Type::String], Type::Boolean, router::router_match);
    lib.add_function("router_get_param", vec![Type::Integer, Type::String], Type::String, router::router_get_param);
    
    lib.add_function("static_serve", vec![Type::String, Type::String], Type::Integer, middleware::static_serve);
    lib.add_function("static_serve_with_cache", vec![Type::String, Type::String, Type::Integer], Type::Integer, middleware::static_serve_with_cache);
    
    lib.add_function("middleware_cors", vec![Type::Integer], Type::Integer, middleware::middleware_cors);
    lib.add_function("middleware_logger", vec![Type::Integer], Type::Integer, middleware::middleware_logger);
    lib.add_function("middleware_compress", vec![Type::Integer], Type::Integer, middleware::middleware_compress);
    lib.add_function("middleware_timeout", vec![Type::Integer, Type::Integer], Type::Integer, middleware::middleware_timeout);
    
    lib.add_function("sse_new", vec![Type::Integer], Type::Integer, sse::sse_new);
    lib.add_function("sse_send", vec![Type::Integer, Type::String, Type::String], Type::Void, sse::sse_send);
    lib.add_function("sse_close", vec![Type::Integer], Type::Void, sse::sse_close);
    
    lib.add_function("quic_listen", vec![Type::String, Type::Integer, Type::Integer], Type::Integer, quic::quic_listen);
    lib.add_function("quic_accept", vec![Type::Integer], Type::Integer, quic::quic_accept);
    lib.add_function("quic_connect", vec![Type::String, Type::Integer], Type::Integer, quic::quic_connect);
    lib.add_function("quic_open_stream", vec![Type::Integer], Type::Integer, quic::quic_open_stream);
    lib.add_function("quic_accept_stream", vec![Type::Integer], Type::Integer, quic::quic_accept_stream);
    lib.add_function("quic_stream_send", vec![Type::Integer, Type::String], Type::Void, quic::quic_stream_send);
    lib.add_function("quic_stream_recv", vec![Type::Integer, Type::Integer], Type::String, quic::quic_stream_recv);
    lib.add_function("quic_close", vec![Type::Integer], Type::Void, quic::quic_close);
    
    lib.c_implementation = Some(include_str!("webcore_man.c").to_string());
    
    for func in &mut lib.functions {
        let impl_opt = match func.name.as_str() {
            "tcp_listen" => Some("webcore_tcp_listen({0}, {1})"),
            "tcp_accept" => Some("webcore_tcp_accept({0})"),
            "tcp_connect" => Some("webcore_tcp_connect({0}, {1})"),
            "tcp_read" => Some("webcore_tcp_read({0}, {1})"),
            "tcp_write" => Some("webcore_tcp_write({0}, {1})"),
            "tcp_close" => Some("webcore_tcp_close({0})"),
            "tcp_shutdown" => Some("webcore_tcp_shutdown({0}, {1})"),
            "tcp_peer_addr" => Some("webcore_tcp_peer_addr({0})"),
            "tcp_local_addr" => Some("webcore_tcp_local_addr({0})"),
            "tcp_set_nodelay" => Some("webcore_tcp_set_nodelay({0}, {1})"),
            "tcp_set_reuseaddr" => Some("webcore_tcp_set_reuseaddr({0}, {1})"),
            "tcp_set_keepalive" => Some("webcore_tcp_set_keepalive({0}, {1})"),
            "tcp_set_timeout" => Some("webcore_tcp_set_timeout({0}, {1})"),
            
            "udp_bind" => Some("webcore_udp_bind({0}, {1})"),
            "udp_send_to" => Some("webcore_udp_send_to({0}, {1}, {2}, {3})"),
            "udp_recv_from" => Some("webcore_udp_recv_from({0}, {1})"),
            "udp_connect" => Some("webcore_udp_connect({0}, {1}, {2})"),
            "udp_close" => Some("webcore_udp_close({0})"),
            
            "tls_context_new" => Some("webcore_tls_context_new()"),
            "tls_load_cert" => Some("webcore_tls_load_cert({0}, {1}, {2})"),
            "tls_wrap_server" => Some("webcore_tls_wrap_server({0}, {1})"),
            "tls_wrap_client" => Some("webcore_tls_wrap_client({0}, {1}, {2})"),
            "tls_read" => Some("webcore_tls_read({0}, {1})"),
            "tls_write" => Some("webcore_tls_write({0}, {1})"),
            "tls_close" => Some("webcore_tls_close({0})"),
            "tls_get_alpn" => Some("webcore_tls_get_alpn({0})"),
            "tls_set_alpn" => Some("webcore_tls_set_alpn({0}, {1})"),
            
            "http_server_new" => Some("webcore_http_server_new()"),
            "http_server_bind" => Some("webcore_http_server_bind({0}, {1}, {2})"),
            "http_server_enable_http1" => Some("webcore_http_server_enable_http1({0}, {1})"),
            "http_server_enable_http2" => Some("webcore_http_server_enable_http2({0}, {1})"),
            "http_server_enable_http3" => Some("webcore_http_server_enable_http3({0}, {1})"),
            "http_server_enable_websocket" => Some("webcore_http_server_enable_websocket({0}, {1})"),
            "http_server_set_max_connections" => Some("webcore_http_server_set_max_connections({0}, {1})"),
            "http_server_set_tls" => Some("webcore_http_server_set_tls({0}, {1})"),
            "http_server_start" => Some("webcore_http_server_start({0})"),
            "http_server_stop" => Some("webcore_http_server_stop({0})"),
            "http_server_accept" => Some("webcore_http_server_accept({0})"),
            
            "http_request_get_method" => Some("webcore_http_request_get_method({0})"),
            "http_request_get_uri" => Some("webcore_http_request_get_uri({0})"),
            "http_request_get_version" => Some("webcore_http_request_get_version({0})"),
            "http_request_get_header" => Some("webcore_http_request_get_header({0}, {1})"),
            "http_request_get_body" => Some("webcore_http_request_get_body({0})"),
            "http_request_get_remote_addr" => Some("webcore_http_request_get_remote_addr({0})"),
            "http_request_is_websocket" => Some("webcore_http_request_is_websocket({0})"),
            "http_request_free" => Some("webcore_http_request_free({0})"),
            
            "http_response_new" => Some("webcore_http_response_new({0})"),
            "http_response_set_header" => Some("webcore_http_response_set_header({0}, {1}, {2})"),
            "http_response_set_body" => Some("webcore_http_response_set_body({0}, {1})"),
            "http_response_set_file" => Some("webcore_http_response_set_file({0}, {1})"),
            "http_response_send" => Some("webcore_http_response_send({0}, {1})"),
            "http_response_free" => Some("webcore_http_response_free({0})"),
            
            "websocket_accept" => Some("webcore_websocket_accept({0})"),
            "websocket_connect" => Some("webcore_websocket_connect({0})"),
            "websocket_send_text" => Some("webcore_websocket_send_text({0}, {1})"),
            "websocket_send_binary" => Some("webcore_websocket_send_binary({0}, {1})"),
            "websocket_recv" => Some("webcore_websocket_recv({0})"),
            "websocket_close" => Some("webcore_websocket_close({0}, {1}, {2})"),
            "websocket_ping" => Some("webcore_websocket_ping({0})"),
            "websocket_pong" => Some("webcore_websocket_pong({0})"),
            
            "http_parse_query" => Some("webcore_http_parse_query({0})"),
            "http_url_encode" => Some("webcore_http_url_encode({0})"),
            "http_url_decode" => Some("webcore_http_url_decode({0})"),
            "http_parse_cookie" => Some("webcore_http_parse_cookie({0})"),
            "http_build_cookie" => Some("webcore_http_build_cookie({0}, {1}, {2})"),
            
            "router_new" => Some("webcore_router_new()"),
            "router_get" => Some("webcore_router_get({0}, {1})"),
            "router_post" => Some("webcore_router_post({0}, {1})"),
            "router_put" => Some("webcore_router_put({0}, {1})"),
            "router_delete" => Some("webcore_router_delete({0}, {1})"),
            "router_match" => Some("webcore_router_match({0}, {1}, {2})"),
            "router_get_param" => Some("webcore_router_get_param({0}, {1})"),
            
            "static_serve" => Some("webcore_static_serve({0}, {1})"),
            "static_serve_with_cache" => Some("webcore_static_serve_with_cache({0}, {1}, {2})"),
            
            "middleware_cors" => Some("webcore_middleware_cors({0})"),
            "middleware_logger" => Some("webcore_middleware_logger({0})"),
            "middleware_compress" => Some("webcore_middleware_compress({0})"),
            "middleware_timeout" => Some("webcore_middleware_timeout({0}, {1})"),
            
            "sse_new" => Some("webcore_sse_new({0})"),
            "sse_send" => Some("webcore_sse_send({0}, {1}, {2})"),
            "sse_close" => Some("webcore_sse_close({0})"),
            
            "quic_listen" => Some("webcore_quic_listen({0}, {1}, {2})"),
            "quic_accept" => Some("webcore_quic_accept({0})"),
            "quic_connect" => Some("webcore_quic_connect({0}, {1})"),
            "quic_open_stream" => Some("webcore_quic_open_stream({0})"),
            "quic_accept_stream" => Some("webcore_quic_accept_stream({0})"),
            "quic_stream_send" => Some("webcore_quic_stream_send({0}, {1})"),
            "quic_stream_recv" => Some("webcore_quic_stream_recv({0}, {1})"),
            "quic_close" => Some("webcore_quic_close({0})"),
            
            _ => None,
        };
        
        if let Some(c_impl) = impl_opt {
            func.c_implementation = Some(c_impl.to_string());
        }
    }
    
    lib
}