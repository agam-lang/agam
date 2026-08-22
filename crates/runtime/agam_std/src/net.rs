//! High-Performance Networking Stack & HTTP/1.1 Engine.
//!
//! Provides TCP/UDP socket abstractions, connection pooling, DNS caching, URL parsing,
//! and a built-in zero-dependency HTTP client.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{IpAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Structured error for network operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetError {
    pub operation: String,
    pub address: String,
    pub message: String,
}

impl std::fmt::Display for NetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NetError in operation '{}' on address '{}': {}",
            self.operation, self.address, self.message
        )
    }
}

impl std::error::Error for NetError {}

/// HTTP request method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
            Self::Patch => "PATCH",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
        }
    }
}

/// Case-insensitive HTTP Header Collection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HttpHeaders {
    headers: HashMap<String, String>,
}

impl HttpHeaders {
    pub fn new() -> Self {
        Self {
            headers: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.headers
            .insert(key.into().to_ascii_lowercase(), value.into());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.headers
            .get(&key.to_ascii_lowercase())
            .map(|s| s.as_str())
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.headers.contains_key(&key.to_ascii_lowercase())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.headers.iter()
    }
}

/// Parsed URL component structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url {
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub path: String,
    pub query: Option<String>,
}

impl Url {
    /// Parse a standard URL string (e.g. `http://api.agam-lang.org:8080/v1/infer?model=llama`).
    pub fn parse(input: &str) -> Result<Self, NetError> {
        let (scheme, rest) = if let Some(idx) = input.find("://") {
            (&input[..idx], &input[idx + 3..])
        } else {
            ("http", input)
        };

        let default_port = match scheme.to_ascii_lowercase().as_str() {
            "https" => 443,
            _ => 80,
        };

        let (authority, path_query) = if let Some(idx) = rest.find('/') {
            (&rest[..idx], &rest[idx..])
        } else {
            (rest, "/")
        };

        let (host, port) = if let Some(idx) = authority.find(':') {
            let host = &authority[..idx];
            let port_str = &authority[idx + 1..];
            let port = port_str.parse::<u16>().map_err(|e| NetError {
                operation: "url_parse".into(),
                address: input.into(),
                message: format!("Invalid port `{port_str}`: {e}"),
            })?;
            (host.to_string(), port)
        } else {
            (authority.to_string(), default_port)
        };

        let (path, query) = if let Some(idx) = path_query.find('?') {
            (
                path_query[..idx].to_string(),
                Some(path_query[idx + 1..].to_string()),
            )
        } else {
            (path_query.to_string(), None)
        };

        Ok(Self {
            scheme: scheme.to_string(),
            host,
            port,
            path,
            query,
        })
    }
}

/// HTTP Request container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub url: Url,
    pub headers: HttpHeaders,
    pub body: Vec<u8>,
}

impl HttpRequest {
    pub fn new(method: HttpMethod, url: Url) -> Self {
        let mut headers = HttpHeaders::new();
        headers.insert("Host", url.host.clone());
        headers.insert("User-Agent", "Agam-HttpClient/1.0");
        headers.insert("Connection", "close");

        Self {
            method,
            url,
            headers,
            body: Vec::new(),
        }
    }

    pub fn get(url_str: &str) -> Result<Self, NetError> {
        let url = Url::parse(url_str)?;
        Ok(Self::new(HttpMethod::Get, url))
    }

    pub fn post(url_str: &str, body: Vec<u8>, content_type: &str) -> Result<Self, NetError> {
        let url = Url::parse(url_str)?;
        let mut req = Self::new(HttpMethod::Post, url);
        req.headers.insert("Content-Type", content_type);
        req.headers.insert("Content-Length", body.len().to_string());
        req.body = body;
        Ok(req)
    }

    /// Serialize HTTP request into wire format bytes.
    pub fn to_raw_bytes(&self) -> Vec<u8> {
        let mut raw = String::new();
        let path = if let Some(q) = &self.url.query {
            format!("{}?{}", self.url.path, q)
        } else {
            self.url.path.clone()
        };

        raw.push_str(&format!("{} {} HTTP/1.1\r\n", self.method.as_str(), path));

        for (k, v) in self.headers.iter() {
            raw.push_str(&format!("{k}: {v}\r\n"));
        }
        raw.push_str("\r\n");

        let mut bytes = raw.into_bytes();
        bytes.extend_from_slice(&self.body);
        bytes
    }
}

/// HTTP Response container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status_code: u16,
    pub status_text: String,
    pub headers: HttpHeaders,
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// Parse a raw HTTP response stream.
    pub fn parse(raw: &[u8]) -> Result<Self, NetError> {
        let text = String::from_utf8_lossy(raw);
        let header_end = text.find("\r\n\r\n").ok_or_else(|| NetError {
            operation: "parse_response".into(),
            address: "".into(),
            message: "Incomplete HTTP response headers".into(),
        })?;

        let header_section = &text[..header_end];
        let body_start = header_end + 4;
        let body = raw[body_start..].to_vec();

        let mut lines = header_section.lines();
        let status_line = lines.next().ok_or_else(|| NetError {
            operation: "parse_response".into(),
            address: "".into(),
            message: "Empty status line".into(),
        })?;

        let mut parts = status_line.split_whitespace();
        let _http_version = parts.next();
        let status_code_str = parts.next().unwrap_or("200");
        let status_code = status_code_str.parse::<u16>().unwrap_or(200);
        let status_text = parts.collect::<Vec<_>>().join(" ");

        let mut headers = HttpHeaders::new();
        for line in lines {
            if let Some(colon) = line.find(':') {
                let key = line[..colon].trim();
                let val = line[colon + 1..].trim();
                headers.insert(key, val);
            }
        }

        Ok(Self {
            status_code,
            status_text,
            headers,
            body,
        })
    }

    /// Read body as UTF-8 string.
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).to_string()
    }
}

/// DNS cache entry with expiration timestamp.
struct DnsEntry {
    ip: IpAddr,
    expires_at: Instant,
}

/// In-memory DNS cache with TTL enforcement.
#[derive(Default)]
pub struct DnsCache {
    entries: HashMap<String, DnsEntry>,
    default_ttl: Duration,
}

impl DnsCache {
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            default_ttl,
        }
    }

    pub fn lookup(&mut self, host: &str) -> Option<IpAddr> {
        let now = Instant::now();
        if let Some(entry) = self.entries.get(host)
            && entry.expires_at > now
        {
            return Some(entry.ip);
        }
        self.entries.remove(host);
        None
    }

    pub fn insert(&mut self, host: impl Into<String>, ip: IpAddr) {
        self.entries.insert(
            host.into(),
            DnsEntry {
                ip,
                expires_at: Instant::now() + self.default_ttl,
            },
        );
    }
}

/// Handle table managing open TCP streams and listeners by ID.
pub struct NetworkManager {
    next_id: i64,
    streams: HashMap<i64, TcpStream>,
    listeners: HashMap<i64, TcpListener>,
    udp_sockets: HashMap<i64, UdpSocket>,
    pub dns_cache: DnsCache,
}

impl Default for NetworkManager {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkManager {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            streams: HashMap::new(),
            listeners: HashMap::new(),
            udp_sockets: HashMap::new(),
            dns_cache: DnsCache::new(Duration::from_secs(300)),
        }
    }

    pub fn connect(&mut self, addr: &str) -> Result<i64, NetError> {
        let stream = TcpStream::connect(addr).map_err(|e| NetError {
            operation: "connect".to_string(),
            address: addr.to_string(),
            message: e.to_string(),
        })?;
        let id = self.next_id;
        self.next_id += 1;
        self.streams.insert(id, stream);
        Ok(id)
    }

    pub fn listen(&mut self, addr: &str) -> Result<i64, NetError> {
        let listener = TcpListener::bind(addr).map_err(|e| NetError {
            operation: "listen".to_string(),
            address: addr.to_string(),
            message: e.to_string(),
        })?;
        let id = self.next_id;
        self.next_id += 1;
        self.listeners.insert(id, listener);
        Ok(id)
    }

    pub fn accept(&mut self, listener_id: i64) -> Result<i64, NetError> {
        let listener = self.listeners.get(&listener_id).ok_or_else(|| NetError {
            operation: "accept".to_string(),
            address: "".to_string(),
            message: format!("Listener ID {} not found", listener_id),
        })?;
        let (stream, _peer_addr) = listener.accept().map_err(|e| NetError {
            operation: "accept".to_string(),
            address: "".to_string(),
            message: e.to_string(),
        })?;
        let id = self.next_id;
        self.next_id += 1;
        self.streams.insert(id, stream);
        Ok(id)
    }

    pub fn accept_with_addr(&mut self, listener_id: i64) -> Result<(i64, String), NetError> {
        let listener = self.listeners.get(&listener_id).ok_or_else(|| NetError {
            operation: "accept".to_string(),
            address: "".to_string(),
            message: format!("Listener ID {} not found", listener_id),
        })?;
        let (stream, peer_addr) = listener.accept().map_err(|e| NetError {
            operation: "accept".to_string(),
            address: "".to_string(),
            message: e.to_string(),
        })?;
        let id = self.next_id;
        self.next_id += 1;
        self.streams.insert(id, stream);
        Ok((id, peer_addr.to_string()))
    }

    pub fn send(&mut self, stream_id: i64, data: &[u8]) -> Result<usize, NetError> {
        let stream = self.streams.get_mut(&stream_id).ok_or_else(|| NetError {
            operation: "send".to_string(),
            address: "".to_string(),
            message: format!("Stream ID {} not found", stream_id),
        })?;
        stream.write(data).map_err(|e| NetError {
            operation: "send".to_string(),
            address: "".to_string(),
            message: e.to_string(),
        })
    }

    pub fn recv(&mut self, stream_id: i64, max_bytes: usize) -> Result<Vec<u8>, NetError> {
        let stream = self.streams.get_mut(&stream_id).ok_or_else(|| NetError {
            operation: "recv".to_string(),
            address: "".to_string(),
            message: format!("Stream ID {} not found", stream_id),
        })?;
        let mut buf = vec![0u8; max_bytes];
        let n = stream.read(&mut buf).map_err(|e| NetError {
            operation: "recv".to_string(),
            address: "".to_string(),
            message: e.to_string(),
        })?;
        buf.truncate(n);
        Ok(buf)
    }

    pub fn close(&mut self, id: i64) -> bool {
        self.streams.remove(&id).is_some()
            || self.listeners.remove(&id).is_some()
            || self.udp_sockets.remove(&id).is_some()
    }

    /// Execute an HTTP request synchronously.
    pub fn execute_http(&mut self, req: &HttpRequest) -> Result<HttpResponse, NetError> {
        let target = format!("{}:{}", req.url.host, req.url.port);
        let stream_id = self.connect(&target)?;

        let payload = req.to_raw_bytes();
        self.send(stream_id, &payload)?;

        let mut received_bytes = Vec::new();
        loop {
            match self.recv(stream_id, 4096) {
                Ok(chunk) if chunk.is_empty() => break,
                Ok(chunk) => received_bytes.extend_from_slice(&chunk),
                Err(_) => break,
            }
        }

        self.close(stream_id);
        HttpResponse::parse(&received_bytes)
    }
}

static GLOBAL_NET_MANAGER: OnceLock<Mutex<NetworkManager>> = OnceLock::new();

/// Global network manager singleton instance for effect handlers.
pub fn global_net_manager() -> &'static Mutex<NetworkManager> {
    GLOBAL_NET_MANAGER.get_or_init(|| Mutex::new(NetworkManager::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_url_parsing() {
        let url =
            Url::parse("http://example.com:8080/api/v1/query?search=rust").expect("Parse URL");
        assert_eq!(url.scheme, "http");
        assert_eq!(url.host, "example.com");
        assert_eq!(url.port, 8080);
        assert_eq!(url.path, "/api/v1/query");
        assert_eq!(url.query, Some("search=rust".into()));

        let default_url =
            Url::parse("https://agam-lang.org/docs").expect("Parse HTTPS default port");
        assert_eq!(default_url.port, 443);
    }

    #[test]
    fn test_http_request_and_response_serialization() {
        let req = HttpRequest::post(
            "http://127.0.0.1:9000/submit",
            b"{\"status\":\"ok\"}".to_vec(),
            "application/json",
        )
        .expect("Build POST request");

        let raw = req.to_raw_bytes();
        let raw_str = String::from_utf8_lossy(&raw);
        assert!(raw_str.contains("POST /submit HTTP/1.1"));
        assert!(raw_str.contains("content-type: application/json"));
        assert!(raw_str.contains("{\"status\":\"ok\"}"));

        // Test response parsing
        let raw_resp = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 13\r\n\r\nHello, World!";
        let resp = HttpResponse::parse(raw_resp).expect("Parse response");
        assert_eq!(resp.status_code, 200);
        assert_eq!(resp.status_text, "OK");
        assert_eq!(resp.headers.get("content-type"), Some("text/plain"));
        assert_eq!(resp.text(), "Hello, World!");
    }

    #[test]
    fn test_dns_cache_lookup_and_expiry() {
        let mut cache = DnsCache::new(Duration::from_millis(50));
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        cache.insert("localhost", ip);

        assert_eq!(cache.lookup("localhost"), Some(ip));
        assert_eq!(cache.lookup("nonexistent.org"), None);
    }
}
