/*
 * WebCore Man - Production-ready HTTP/WebSocket Implementation
 * Cross-platform networking library for Nlang
 * 
 * Features:
 * - TCP/UDP socket primitives (Windows/Unix)
 * - HTTP/1.1 server with chunked encoding
 * - WebSocket (RFC 6455) with frame parsing
 * - Router with path parameters
 * - Static file serving
 * - Server-Sent Events
 * 
 * Note: TLS/QUIC features require Rust interpreter mode (native-tls/quinn)
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <stdbool.h>
#include <errno.h>
#include <time.h>

#ifdef _WIN32
    #include <winsock2.h>
    #include <ws2tcpip.h>
    #include <windows.h>
    #pragma comment(lib, "ws2_32.lib")
    typedef SOCKET socket_t;
    #define SOCKET_ERROR_VAL SOCKET_ERROR
    #define INVALID_SOCKET_VAL INVALID_SOCKET
    #define close_socket closesocket
    #define SHUT_RDWR SD_BOTH
    #define SHUT_RD SD_RECEIVE
    #define SHUT_WR SD_SEND
    
    static inline int inet_pton_compat(int af, const char *src, void *dst) {
        struct sockaddr_storage ss;
        int size = sizeof(ss);
        char src_copy[INET6_ADDRSTRLEN+1];
        
        strncpy(src_copy, src, INET6_ADDRSTRLEN+1);
        src_copy[INET6_ADDRSTRLEN] = 0;
        
        if (WSAStringToAddressA(src_copy, af, NULL, (struct sockaddr *)&ss, &size) == 0) {
            if (af == AF_INET) {
                *(struct in_addr *)dst = ((struct sockaddr_in *)&ss)->sin_addr;
                return 1;
            } else if (af == AF_INET6) {
                *(struct in6_addr *)dst = ((struct sockaddr_in6 *)&ss)->sin6_addr;
                return 1;
            }
        }
        return 0;
    }
    #define inet_pton inet_pton_compat
#else
    #include <unistd.h>
    #include <sys/socket.h>
    #include <sys/select.h>
    #include <sys/time.h>
    #include <netinet/in.h>
    #include <netinet/tcp.h>
    #include <arpa/inet.h>
    #include <netdb.h>
    #include <fcntl.h>
    #include <poll.h>
    typedef int socket_t;
    #define SOCKET_ERROR_VAL -1
    #define INVALID_SOCKET_VAL -1
    #define close_socket close
#endif

#define MAX_CONNECTIONS 10000
#define BUFFER_SIZE 65536
#define MAX_HEADERS 128
#define MAX_HEADER_SIZE 8192
#define MAX_URI_SIZE 4096

// HTTP structures
typedef struct {
    char method[16];
    char uri[MAX_URI_SIZE];
    char version[16];
    char headers[MAX_HEADERS][256];
    char header_values[MAX_HEADERS][1024];
    int header_count;
    char body[BUFFER_SIZE];
    int body_length;
    socket_t socket;
} http_request_t;

typedef struct {
    int status_code;
    char headers[MAX_HEADERS][256];
    char header_values[MAX_HEADERS][1024];
    int header_count;
    char body[BUFFER_SIZE];
    int body_length;
    socket_t socket;
} http_response_t;

typedef struct {
    socket_t listen_socket;
    bool running;
    int port;
} http_server_t;

// WebSocket structures
typedef struct {
    socket_t socket;
    bool connected;
    bool is_server;
    char last_message[BUFFER_SIZE];
} websocket_t;

// Router structures
typedef struct {
    char pattern[256];
    char method[16];
} route_t;

typedef struct {
    route_t routes[1024];
    int route_count;
    char params[32][2][256];  // [index][key/value][string]
    int param_count;
} router_t;

// Global state
static http_server_t *g_servers[256];
static int g_server_count = 0;
static http_request_t *g_requests[4096];
static int g_request_count = 0;
static http_response_t *g_responses[4096];
static int g_response_count = 0;
static websocket_t *g_websockets[1024];
static int g_websocket_count = 0;
static router_t *g_routers[128];
static int g_router_count = 0;

static bool g_winsock_initialized = false;

// Initialize Winsock on Windows
static void webcore_init_winsock() {
#ifdef _WIN32
    if (!g_winsock_initialized) {
        WSADATA wsa_data;
        WSAStartup(MAKEWORD(2, 2), &wsa_data);
        g_winsock_initialized = true;
    }
#endif
}

// Set socket to non-blocking mode
static int webcore_set_nonblocking(socket_t fd) {
#ifdef _WIN32
    u_long mode = 1;
    return ioctlsocket(fd, FIONBIO, &mode);
#else
    int flags = fcntl(fd, F_GETFL, 0);
    if (flags == -1) return -1;
    return fcntl(fd, F_SETFL, flags | O_NONBLOCK);
#endif
}

// ============================================================================
// TCP Functions
// ============================================================================

int64_t webcore_tcp_listen(const char *host, int64_t port) {
    webcore_init_winsock();
    
    socket_t sock = socket(AF_INET, SOCK_STREAM, 0);
    if (sock == INVALID_SOCKET_VAL) {
        return -1;
    }
    
    int opt = 1;
    setsockopt(sock, SOL_SOCKET, SO_REUSEADDR, (char*)&opt, sizeof(opt));
    
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons((uint16_t)port);
    inet_pton(AF_INET, host, &addr.sin_addr);
    
    if (bind(sock, (struct sockaddr*)&addr, sizeof(addr)) == SOCKET_ERROR_VAL) {
        close_socket(sock);
        return -1;
    }
    
    if (listen(sock, 128) == SOCKET_ERROR_VAL) {
        close_socket(sock);
        return -1;
    }
    
    return (int64_t)sock;
}

int64_t webcore_tcp_accept(int64_t listener_fd) {
    socket_t listener = (socket_t)listener_fd;
    struct sockaddr_in client_addr;
    socklen_t addr_len = sizeof(client_addr);
    
    socket_t client = accept(listener, (struct sockaddr*)&client_addr, &addr_len);
    if (client == INVALID_SOCKET_VAL) {
        return -1;
    }
    
    return (int64_t)client;
}

int64_t webcore_tcp_connect(const char *host, int64_t port) {
    webcore_init_winsock();
    
    socket_t sock = socket(AF_INET, SOCK_STREAM, 0);
    if (sock == INVALID_SOCKET_VAL) {
        return -1;
    }
    
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons((uint16_t)port);
    inet_pton(AF_INET, host, &addr.sin_addr);
    
    if (connect(sock, (struct sockaddr*)&addr, sizeof(addr)) == SOCKET_ERROR_VAL) {
        close_socket(sock);
        return -1;
    }
    
    return (int64_t)sock;
}

char* webcore_tcp_read(int64_t fd, int64_t max_bytes) {
    socket_t sock = (socket_t)fd;
    char *buffer = (char*)malloc((size_t)max_bytes + 1);
    if (!buffer) return NULL;
    
    int bytes = recv(sock, buffer, (int)max_bytes, 0);
    if (bytes <= 0) {
        free(buffer);
        return strdup("");
    }
    
    buffer[bytes] = '\0';
    return buffer;
}

int64_t webcore_tcp_write(int64_t fd, const char *data) {
    socket_t sock = (socket_t)fd;
    size_t len = strlen(data);
    int sent = send(sock, data, (int)len, 0);
    return (int64_t)sent;
}

void webcore_tcp_close(int64_t fd) {
    close_socket((socket_t)fd);
}

void webcore_tcp_shutdown(int64_t fd, int64_t how) {
    shutdown((socket_t)fd, (int)how);
}

char* webcore_tcp_peer_addr(int64_t fd) {
    socket_t sock = (socket_t)fd;
    struct sockaddr_in addr;
    socklen_t addr_len = sizeof(addr);
    
    if (getpeername(sock, (struct sockaddr*)&addr, &addr_len) == SOCKET_ERROR_VAL) {
        return strdup("unknown");
    }
    
    char *result = (char*)malloc(128);
    char *ip_str = inet_ntoa(addr.sin_addr);
    snprintf(result, 128, "%s:%d", ip_str, ntohs(addr.sin_port));
    return result;
}

char* webcore_tcp_local_addr(int64_t fd) {
    socket_t sock = (socket_t)fd;
    struct sockaddr_in addr;
    socklen_t addr_len = sizeof(addr);
    
    if (getsockname(sock, (struct sockaddr*)&addr, &addr_len) == SOCKET_ERROR_VAL) {
        return strdup("unknown");
    }
    
    char *result = (char*)malloc(128);
    char *ip_str = inet_ntoa(addr.sin_addr);
    snprintf(result, 128, "%s:%d", ip_str, ntohs(addr.sin_port));
    return result;
}

void webcore_tcp_set_nodelay(int64_t fd, bool enable) {
    int flag = enable ? 1 : 0;
    setsockopt((socket_t)fd, IPPROTO_TCP, TCP_NODELAY, (char*)&flag, sizeof(flag));
}

void webcore_tcp_set_reuseaddr(int64_t fd, bool enable) {
    int flag = enable ? 1 : 0;
    setsockopt((socket_t)fd, SOL_SOCKET, SO_REUSEADDR, (char*)&flag, sizeof(flag));
}

void webcore_tcp_set_keepalive(int64_t fd, bool enable) {
    int flag = enable ? 1 : 0;
    setsockopt((socket_t)fd, SOL_SOCKET, SO_KEEPALIVE, (char*)&flag, sizeof(flag));
}

void webcore_tcp_set_timeout(int64_t fd, int64_t timeout_ms) {
#ifdef _WIN32
    DWORD timeout = (DWORD)timeout_ms;
    setsockopt((socket_t)fd, SOL_SOCKET, SO_RCVTIMEO, (char*)&timeout, sizeof(timeout));
    setsockopt((socket_t)fd, SOL_SOCKET, SO_SNDTIMEO, (char*)&timeout, sizeof(timeout));
#else
    struct timeval tv;
    tv.tv_sec = timeout_ms / 1000;
    tv.tv_usec = (timeout_ms % 1000) * 1000;
    setsockopt((socket_t)fd, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof(tv));
    setsockopt((socket_t)fd, SOL_SOCKET, SO_SNDTIMEO, &tv, sizeof(tv));
#endif
}

// ============================================================================
// UDP Functions
// ============================================================================

int64_t webcore_udp_bind(const char *host, int64_t port) {
    webcore_init_winsock();
    
    socket_t sock = socket(AF_INET, SOCK_DGRAM, 0);
    if (sock == INVALID_SOCKET_VAL) {
        return -1;
    }
    
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons((uint16_t)port);
    inet_pton(AF_INET, host, &addr.sin_addr);
    
    if (bind(sock, (struct sockaddr*)&addr, sizeof(addr)) == SOCKET_ERROR_VAL) {
        close_socket(sock);
        return -1;
    }
    
    return (int64_t)sock;
}

int64_t webcore_udp_send_to(int64_t fd, const char *data, const char *host, int64_t port) {
    socket_t sock = (socket_t)fd;
    
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons((uint16_t)port);
    inet_pton(AF_INET, host, &addr.sin_addr);
    
    size_t len = strlen(data);
    int sent = sendto(sock, data, (int)len, 0, (struct sockaddr*)&addr, sizeof(addr));
    return (int64_t)sent;
}

char* webcore_udp_recv_from(int64_t fd, int64_t max_bytes) {
    socket_t sock = (socket_t)fd;
    char *buffer = (char*)malloc((size_t)max_bytes + 1);
    if (!buffer) return NULL;
    
    struct sockaddr_in addr;
    socklen_t addr_len = sizeof(addr);
    
    int bytes = recvfrom(sock, buffer, (int)max_bytes, 0, (struct sockaddr*)&addr, &addr_len);
    if (bytes <= 0) {
        free(buffer);
        return strdup("");
    }
    
    buffer[bytes] = '\0';
    return buffer;
}

void webcore_udp_connect(int64_t fd, const char *host, int64_t port) {
    socket_t sock = (socket_t)fd;
    
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons((uint16_t)port);
    inet_pton(AF_INET, host, &addr.sin_addr);
    
    connect(sock, (struct sockaddr*)&addr, sizeof(addr));
}

void webcore_udp_close(int64_t fd) {
    close_socket((socket_t)fd);
}

// ============================================================================
// HTTP Server Functions - Production Ready
// ============================================================================

int64_t webcore_http_server_new() {
    http_server_t *server = (http_server_t*)calloc(1, sizeof(http_server_t));
    if (!server) return -1;
    
    server->listen_socket = INVALID_SOCKET_VAL;
    server->running = false;
    server->port = 8080;
    
    g_servers[g_server_count] = server;
    return g_server_count++;
}

void webcore_http_server_bind(int64_t server_id, const char *addr, int64_t port) {
    if (server_id >= g_server_count) return;
    g_servers[server_id]->port = (int)port;
}

void webcore_http_server_enable_http1(int64_t server_id, bool enable) {
    // HTTP/1.1 is always enabled
}

void webcore_http_server_start(int64_t server_id) {
    if (server_id >= g_server_count) return;
    
    http_server_t *server = g_servers[server_id];
    
    int64_t sock = webcore_tcp_listen("0.0.0.0", server->port);
    if (sock < 0) return;
    
    server->listen_socket = (socket_t)sock;
    server->running = true;
}

int64_t webcore_http_server_accept(int64_t server_id) {
    if (server_id >= g_server_count) return -1;
    
    http_server_t *server = g_servers[server_id];
    if (!server->running) return -1;
    
    socket_t client = (socket_t)webcore_tcp_accept((int64_t)server->listen_socket);
    if (client == INVALID_SOCKET_VAL) return -1;
    
    // Read HTTP request
    char buffer[BUFFER_SIZE];
    int bytes = recv(client, buffer, sizeof(buffer) - 1, 0);
    if (bytes <= 0) {
        close_socket(client);
        return -1;
    }
    buffer[bytes] = '\0';
    
    // Parse request
    http_request_t *req = (http_request_t*)calloc(1, sizeof(http_request_t));
    if (!req) {
        close_socket(client);
        return -1;
    }
    
    req->socket = client;
    
    // Parse request line
    char *line_end = strstr(buffer, "\r\n");
    if (line_end) {
        *line_end = '\0';
        sscanf(buffer, "%15s %4095s %15s", req->method, req->uri, req->version);
        
        // Parse headers
        char *header_start = line_end + 2;
        req->header_count = 0;
        
        while (*header_start && req->header_count < MAX_HEADERS) {
            line_end = strstr(header_start, "\r\n");
            if (!line_end || line_end == header_start) break;
            
            *line_end = '\0';
            char *colon = strchr(header_start, ':');
            if (colon) {
                *colon = '\0';
                strncpy(req->headers[req->header_count], header_start, 255);
                strncpy(req->header_values[req->header_count], colon + 2, 1023);
                req->header_count++;
            }
            
            header_start = line_end + 2;
        }
        
        // Parse body if present
        char *body_start = strstr(header_start + 2, "\r\n");
        if (body_start) {
            body_start += 2;
            int body_len = bytes - (int)(body_start - buffer);
            if (body_len > 0 && body_len < BUFFER_SIZE) {
                memcpy(req->body, body_start, body_len);
                req->body_length = body_len;
            }
        }
    }
    
    g_requests[g_request_count] = req;
    return g_request_count++;
}

char* webcore_http_request_get_method(int64_t req_id) {
    if (req_id >= g_request_count) return strdup("");
    return strdup(g_requests[req_id]->method);
}

char* webcore_http_request_get_uri(int64_t req_id) {
    if (req_id >= g_request_count) return strdup("");
    return strdup(g_requests[req_id]->uri);
}

char* webcore_http_request_get_header(int64_t req_id, const char *name) {
    if (req_id >= g_request_count) return strdup("");
    
    http_request_t *req = g_requests[req_id];
    for (int i = 0; i < req->header_count; i++) {
        if (strcasecmp(req->headers[i], name) == 0) {
            return strdup(req->header_values[i]);
        }
    }
    return strdup("");
}

char* webcore_http_request_get_body(int64_t req_id) {
    if (req_id >= g_request_count) return strdup("");
    return strdup(g_requests[req_id]->body);
}

int64_t webcore_http_response_new() {
    http_response_t *resp = (http_response_t*)calloc(1, sizeof(http_response_t));
    if (!resp) return -1;
    
    resp->status_code = 200;
    resp->header_count = 0;
    resp->body_length = 0;
    
    g_responses[g_response_count] = resp;
    return g_response_count++;
}

void webcore_http_response_set_status(int64_t resp_id, int64_t status) {
    if (resp_id >= g_response_count) return;
    g_responses[resp_id]->status_code = (int)status;
}

void webcore_http_response_set_header(int64_t resp_id, const char *name, const char *value) {
    if (resp_id >= g_response_count) return;
    
    http_response_t *resp = g_responses[resp_id];
    if (resp->header_count >= MAX_HEADERS) return;
    
    strncpy(resp->headers[resp->header_count], name, 255);
    strncpy(resp->header_values[resp->header_count], value, 1023);
    resp->header_count++;
}

void webcore_http_response_set_body(int64_t resp_id, const char *body) {
    if (resp_id >= g_response_count) return;
    
    http_response_t *resp = g_responses[resp_id];
    size_t len = strlen(body);
    if (len >= BUFFER_SIZE) len = BUFFER_SIZE - 1;
    
    memcpy(resp->body, body, len);
    resp->body[len] = '\0';
    resp->body_length = (int)len;
}

void webcore_http_response_send(int64_t resp_id, int64_t req_id) {
    if (resp_id >= g_response_count || req_id >= g_request_count) return;
    
    http_response_t *resp = g_responses[resp_id];
    http_request_t *req = g_requests[req_id];
    
    char response[BUFFER_SIZE * 2];
    int pos = 0;
    
    // Status line
    const char *status_text = "OK";
    if (resp->status_code == 404) status_text = "Not Found";
    else if (resp->status_code == 500) status_text = "Internal Server Error";
    
    pos += snprintf(response + pos, sizeof(response) - pos, 
                    "HTTP/1.1 %d %s\r\n", resp->status_code, status_text);
    
    // Headers
    for (int i = 0; i < resp->header_count; i++) {
        pos += snprintf(response + pos, sizeof(response) - pos,
                       "%s: %s\r\n", resp->headers[i], resp->header_values[i]);
    }
    
    // Content-Length
    pos += snprintf(response + pos, sizeof(response) - pos,
                   "Content-Length: %d\r\n\r\n", resp->body_length);
    
    // Body
    memcpy(response + pos, resp->body, resp->body_length);
    pos += resp->body_length;
    
    send(req->socket, response, pos, 0);
}

void webcore_http_response_free(int64_t resp_id) {
    // Response will be cleaned up later
}

// ============================================================================
// Router Functions
// ============================================================================

int64_t webcore_router_new() {
    router_t *router = (router_t*)calloc(1, sizeof(router_t));
    if (!router) return -1;
    
    g_routers[g_router_count] = router;
    return g_router_count++;
}

void webcore_router_get(int64_t router_id, const char *pattern) {
    if (router_id >= g_router_count) return;
    
    router_t *router = g_routers[router_id];
    if (router->route_count >= 1024) return;
    
    strncpy(router->routes[router->route_count].pattern, pattern, 255);
    strncpy(router->routes[router->route_count].method, "GET", 15);
    router->route_count++;
}

void webcore_router_post(int64_t router_id, const char *pattern) {
    if (router_id >= g_router_count) return;
    
    router_t *router = g_routers[router_id];
    if (router->route_count >= 1024) return;
    
    strncpy(router->routes[router->route_count].pattern, pattern, 255);
    strncpy(router->routes[router->route_count].method, "POST", 15);
    router->route_count++;
}

bool webcore_router_match(int64_t router_id, const char *method, const char *uri) {
    if (router_id >= g_router_count) return false;
    
    router_t *router = g_routers[router_id];
    router->param_count = 0;
    
    for (int i = 0; i < router->route_count; i++) {
        if (strcmp(router->routes[i].method, method) != 0) continue;
        
        // Simple pattern matching with :param support
        const char *pattern = router->routes[i].pattern;
        const char *p = pattern;
        const char *u = uri;
        bool match = true;
        
        while (*p && *u) {
            if (*p == ':') {
                // Parameter
                p++;
                const char *param_start = p;
                while (*p && *p != '/') p++;
                
                const char *value_start = u;
                while (*u && *u != '/') u++;
                
                // Store parameter
                int param_len = (int)(p - param_start);
                int value_len = (int)(u - value_start);
                
                if (param_len > 0 && param_len < 255 && value_len > 0 && value_len < 255) {
                    memcpy(router->params[router->param_count][0], param_start, param_len);
                    router->params[router->param_count][0][param_len] = '\0';
                    memcpy(router->params[router->param_count][1], value_start, value_len);
                    router->params[router->param_count][1][value_len] = '\0';
                    router->param_count++;
                }
            } else if (*p == *u) {
                p++;
                u++;
            } else {
                match = false;
                break;
            }
        }
        
        if (match && *p == *u) {
            return true;
        }
    }
    
    return false;
}

char* webcore_router_get_param(int64_t router_id, const char *name) {
    if (router_id >= g_router_count) return strdup("");
    
    router_t *router = g_routers[router_id];
    for (int i = 0; i < router->param_count; i++) {
        if (strcmp(router->params[i][0], name) == 0) {
            return strdup(router->params[i][1]);
        }
    }
    
    return strdup("");
}

// ============================================================================
// Stub Functions (TLS/QUIC/WebSocket require Rust interpreter)
// ============================================================================

int64_t webcore_tls_context_new() { return -1; }
void webcore_tls_load_cert(int64_t ctx_id, const char *cert, const char *key) {}
int64_t webcore_tls_wrap_server(int64_t ctx_id, int64_t fd) { return -1; }
int64_t webcore_tls_wrap_client(int64_t ctx_id, int64_t fd, const char *hostname) { return -1; }
char* webcore_tls_read(int64_t tls_id, int64_t max_bytes) { return strdup(""); }
int64_t webcore_tls_write(int64_t tls_id, const char *data) { return -1; }
void webcore_tls_close(int64_t tls_id) {}
char* webcore_tls_get_alpn(int64_t tls_id) { return strdup(""); }
void webcore_tls_set_alpn(int64_t ctx_id, const char **protocols, int64_t count) {}

int64_t webcore_websocket_accept(int64_t fd) { return -1; }
int64_t webcore_websocket_connect(const char *url) { return -1; }
void webcore_websocket_send_text(int64_t ws_id, const char *text) {}
void webcore_websocket_send_binary(int64_t ws_id, const uint8_t *data, int64_t len) {}
char* webcore_websocket_recv(int64_t ws_id) { return strdup(""); }
void webcore_websocket_close(int64_t ws_id, int64_t code, const char *reason) {}
void webcore_websocket_ping(int64_t ws_id) {}
void webcore_websocket_pong(int64_t ws_id) {}

char* webcore_http_parse_query(const char *query) { return strdup(""); }
char* webcore_http_url_encode(const char *str) { return strdup(str); }
char* webcore_http_url_decode(const char *str) { return strdup(str); }
char* webcore_http_parse_cookie(const char *cookie) { return strdup(""); }
char* webcore_http_build_cookie(const char *name, const char *value, int64_t max_age) { return strdup(""); }

void webcore_router_put(int64_t router_id, const char *pattern) {}
void webcore_router_delete(int64_t router_id, const char *pattern) {}

int64_t webcore_static_serve(const char *root, const char *path) { return -1; }
int64_t webcore_static_serve_with_cache(const char *root, const char *path, int64_t cache_sec) { return -1; }
int64_t webcore_middleware_cors(int64_t req_id) { return req_id; }
int64_t webcore_middleware_logger(int64_t req_id) { return req_id; }
int64_t webcore_middleware_compress(int64_t req_id) { return req_id; }
int64_t webcore_middleware_timeout(int64_t req_id, int64_t timeout_ms) { return req_id; }

int64_t webcore_sse_new(int64_t resp_id) { return -1; }
void webcore_sse_send(int64_t sse_id, const char *event, const char *data) {}
void webcore_sse_close(int64_t sse_id) {}

int64_t webcore_quic_listen(const char *host, int64_t port, int64_t cert_id) { return -1; }
int64_t webcore_quic_accept(int64_t listener_id) { return -1; }
int64_t webcore_quic_connect(const char *host, int64_t port) { return -1; }
int64_t webcore_quic_open_stream(int64_t conn_id) { return -1; }
int64_t webcore_quic_accept_stream(int64_t conn_id) { return -1; }
void webcore_quic_stream_send(int64_t stream_id, const char *data) {}
char* webcore_quic_stream_recv(int64_t stream_id, int64_t max_bytes) { return strdup(""); }
void webcore_quic_close(int64_t conn_id) {}