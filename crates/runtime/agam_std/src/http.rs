//! # High-Throughput HTTP/1.1 Protocol Engine & Router (`agam_std::http`)
//!
//! Provides ergonomic HTTP request/response abstractions, route matching,
//! response serialization, and zero-copy header parsing anchored in `httparse`.

use crate::net::HttpMethod;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt;

/// Nyāya-grounded structured diagnostic error for HTTP operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpError {
    pub cause: String,
    pub context: String,
    pub remedy: String,
}

impl HttpError {
    pub fn new(
        cause: impl Into<String>,
        context: impl Into<String>,
        remedy: impl Into<String>,
    ) -> Self {
        Self {
            cause: cause.into(),
            context: context.into(),
            remedy: remedy.into(),
        }
    }
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HttpError: {}\n  Context: {}\n  Remedy: {}",
            self.cause, self.context, self.remedy
        )
    }
}

impl std::error::Error for HttpError {}

/// Standard HTTP Status Code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HttpStatus(pub u16);

impl HttpStatus {
    pub const OK: HttpStatus = HttpStatus(200);
    pub const CREATED: HttpStatus = HttpStatus(201);
    pub const ACCEPTED: HttpStatus = HttpStatus(202);
    pub const NO_CONTENT: HttpStatus = HttpStatus(204);
    pub const MOVED_PERMANENTLY: HttpStatus = HttpStatus(301);
    pub const FOUND: HttpStatus = HttpStatus(302);
    pub const BAD_REQUEST: HttpStatus = HttpStatus(400);
    pub const UNAUTHORIZED: HttpStatus = HttpStatus(401);
    pub const FORBIDDEN: HttpStatus = HttpStatus(403);
    pub const NOT_FOUND: HttpStatus = HttpStatus(404);
    pub const METHOD_NOT_ALLOWED: HttpStatus = HttpStatus(405);
    pub const CONFLICT: HttpStatus = HttpStatus(409);
    pub const UNPROCESSABLE_ENTITY: HttpStatus = HttpStatus(422);
    pub const TOO_MANY_REQUESTS: HttpStatus = HttpStatus(429);
    pub const INTERNAL_SERVER_ERROR: HttpStatus = HttpStatus(500);
    pub const NOT_IMPLEMENTED: HttpStatus = HttpStatus(501);
    pub const BAD_GATEWAY: HttpStatus = HttpStatus(502);
    pub const SERVICE_UNAVAILABLE: HttpStatus = HttpStatus(503);

    pub fn canonical_reason(&self) -> &'static str {
        match self.0 {
            200 => "OK",
            201 => "Created",
            202 => "Accepted",
            204 => "No Content",
            301 => "Moved Permanently",
            302 => "Found",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            409 => "Conflict",
            422 => "Unprocessable Entity",
            429 => "Too Many Requests",
            500 => "Internal Server Error",
            501 => "Not Implemented",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            _ => "Unknown Status",
        }
    }
}

impl fmt::Display for HttpStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.0, self.canonical_reason())
    }
}

/// Incoming HTTP/1.1 Request Representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpRequest {
    /// Construct a new HTTP request.
    pub fn new(method: HttpMethod, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    /// Set a request header.
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers
            .insert(name.into().to_lowercase(), value.into());
        self
    }

    /// Set request payload body.
    pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }

    /// Parse an HTTP/1.1 byte stream using zero-copy `httparse`.
    pub fn parse(buf: &[u8]) -> Result<Option<(Self, usize)>, HttpError> {
        let mut header_storage = [httparse::EMPTY_HEADER; 64];
        let mut parsed_req = httparse::Request::new(&mut header_storage);

        let status = parsed_req.parse(buf).map_err(|e| {
            HttpError::new(
                "Failed to parse HTTP/1.1 request wire format",
                e.to_string(),
                "Verify connecting client adheres to RFC 7230 HTTP/1.1 specification",
            )
        })?;

        let header_len = match status {
            httparse::Status::Complete(len) => len,
            httparse::Status::Partial => return Ok(None),
        };

        let raw_method = parsed_req.method.unwrap_or("GET");
        let method = match raw_method {
            "GET" => HttpMethod::Get,
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "DELETE" => HttpMethod::Delete,
            "PATCH" => HttpMethod::Patch,
            "HEAD" => HttpMethod::Head,
            "OPTIONS" => HttpMethod::Options,
            other => {
                return Err(HttpError::new(
                    "Unsupported HTTP request method",
                    format!("Received unassigned method verb: '{}'", other),
                    "Use standard HTTP methods: GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS",
                ));
            }
        };

        let path = parsed_req.path.unwrap_or("/").to_string();

        let mut headers = HashMap::new();
        let mut content_length: usize = 0;

        for h in parsed_req.headers {
            let name = h.name.to_lowercase();
            let val = std::str::from_utf8(h.value)
                .map_err(|_| {
                    HttpError::new(
                        "Header value contains invalid non-UTF-8 bytes",
                        format!("Header name: '{}'", name),
                        "Sanitize HTTP headers to ASCII/UTF-8 strings",
                    )
                })?
                .trim()
                .to_string();

            if name == "content-length" {
                content_length = val.parse::<usize>().unwrap_or(0);
            }
            headers.insert(name, val);
        }

        let total_expected_len = header_len + content_length;
        if buf.len() < total_expected_len {
            return Ok(None);
        }

        let body = buf[header_len..total_expected_len].to_vec();

        Ok(Some((
            Self {
                method,
                path,
                headers,
                body,
            },
            total_expected_len,
        )))
    }
}

/// Outgoing HTTP/1.1 Response Representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: HttpStatus,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// 200 OK Response.
    pub fn ok() -> Self {
        Self {
            status: HttpStatus::OK,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    /// 404 Not Found Response.
    pub fn not_found() -> Self {
        let mut resp = Self {
            status: HttpStatus::NOT_FOUND,
            headers: HashMap::new(),
            body: b"404 Not Found\n".to_vec(),
        };
        resp.headers
            .insert("content-type".into(), "text/plain; charset=utf-8".into());
        resp
    }

    /// 400 Bad Request Response.
    pub fn bad_request(msg: impl Into<String>) -> Self {
        let text = msg.into();
        let mut resp = Self {
            status: HttpStatus::BAD_REQUEST,
            headers: HashMap::new(),
            body: text.into_bytes(),
        };
        resp.headers
            .insert("content-type".into(), "text/plain; charset=utf-8".into());
        resp
    }

    /// 500 Internal Server Error Response.
    pub fn server_error(msg: impl Into<String>) -> Self {
        let text = msg.into();
        let mut resp = Self {
            status: HttpStatus::INTERNAL_SERVER_ERROR,
            headers: HashMap::new(),
            body: text.into_bytes(),
        };
        resp.headers
            .insert("content-type".into(), "text/plain; charset=utf-8".into());
        resp
    }

    /// Plaintext response with UTF-8 content type.
    pub fn text(body: impl Into<String>) -> Self {
        let text = body.into();
        let mut resp = Self::ok();
        resp.headers
            .insert("content-type".into(), "text/plain; charset=utf-8".into());
        resp.body = text.into_bytes();
        resp
    }

    /// JSON serialized response.
    pub fn json<T: Serialize>(value: &T) -> Result<Self, HttpError> {
        let payload = serde_json::to_vec(value).map_err(|e| {
            HttpError::new(
                "Failed to serialize response value into JSON",
                e.to_string(),
                "Ensure struct fields implement serde Serialize",
            )
        })?;

        let mut resp = Self::ok();
        resp.headers
            .insert("content-type".into(), "application/json".into());
        resp.body = payload;
        Ok(resp)
    }

    /// Add custom header.
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers
            .insert(name.into().to_lowercase(), value.into());
        self
    }

    /// Encode response into valid HTTP/1.1 wire protocol byte stream.
    pub fn encode(&self) -> Vec<u8> {
        let status_line = format!(
            "HTTP/1.1 {} {}\r\n",
            self.status.0,
            self.status.canonical_reason()
        );

        let mut header_block = String::new();
        header_block.push_str(&status_line);

        let mut headers = self.headers.clone();
        if !headers.contains_key("content-length") {
            headers.insert("content-length".into(), self.body.len().to_string());
        }
        if !headers.contains_key("connection") {
            headers.insert("connection".into(), "keep-alive".into());
        }

        for (name, val) in &headers {
            header_block.push_str(&format!("{}: {}\r\n", name, val));
        }
        header_block.push_str("\r\n");

        let mut out = Vec::with_capacity(header_block.len() + self.body.len());
        out.extend_from_slice(header_block.as_bytes());
        out.extend_from_slice(&self.body);
        out
    }
}

pub type HttpHandler = Box<dyn Fn(&HttpRequest) -> HttpResponse + Send + Sync + 'static>;

/// Route Registry & Dispatch Engine.
#[derive(Default)]
pub struct HttpRouter {
    routes: HashMap<(HttpMethod, String), HttpHandler>,
}

impl HttpRouter {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }

    /// Register a route handler for a given HTTP method and exact path.
    pub fn route<F>(&mut self, method: HttpMethod, path: impl Into<String>, handler: F)
    where
        F: Fn(&HttpRequest) -> HttpResponse + Send + Sync + 'static,
    {
        self.routes.insert((method, path.into()), Box::new(handler));
    }

    /// Helper for GET routes.
    pub fn get<F>(&mut self, path: impl Into<String>, handler: F)
    where
        F: Fn(&HttpRequest) -> HttpResponse + Send + Sync + 'static,
    {
        self.route(HttpMethod::Get, path, handler);
    }

    /// Helper for POST routes.
    pub fn post<F>(&mut self, path: impl Into<String>, handler: F)
    where
        F: Fn(&HttpRequest) -> HttpResponse + Send + Sync + 'static,
    {
        self.route(HttpMethod::Post, path, handler);
    }

    /// Dispatch incoming request against registered route map.
    pub fn dispatch(&self, req: &HttpRequest) -> HttpResponse {
        if let Some(handler) = self.routes.get(&(req.method, req.path.clone())) {
            handler(req)
        } else {
            HttpResponse::not_found()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_request_parse_valid_stream() -> Result<(), HttpError> {
        let raw = b"POST /api/v1/compute HTTP/1.1\r\nHost: localhost:8080\r\nContent-Type: application/json\r\nContent-Length: 18\r\n\r\n{\"input_value\":42}";
        let parsed = HttpRequest::parse(raw)?;
        assert!(parsed.is_some());
        let (req, consumed) = parsed.unwrap_or_else(|| unreachable!());

        assert_eq!(consumed, raw.len());
        assert_eq!(req.method, HttpMethod::Post);
        assert_eq!(req.path, "/api/v1/compute");
        assert_eq!(
            req.headers.get("content-type").map(|s| s.as_str()),
            Some("application/json")
        );
        assert_eq!(req.body, b"{\"input_value\":42}");
        Ok(())
    }

    #[test]
    fn test_http_request_parse_partial_returns_none() -> Result<(), HttpError> {
        let raw = b"GET /index.html HTTP/1.1\r\nHost: local";
        let res = HttpRequest::parse(raw)?;
        assert!(res.is_none());
        Ok(())
    }

    #[test]
    fn test_http_response_encode_and_headers() {
        let resp =
            HttpResponse::text("Hello from Agam HTTP Engine!").with_header("x-engine", "agam-v1");

        let encoded = resp.encode();
        let encoded_str = String::from_utf8_lossy(&encoded);

        assert!(encoded_str.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(encoded_str.contains("content-type: text/plain; charset=utf-8\r\n"));
        assert!(encoded_str.contains("x-engine: agam-v1\r\n"));
        assert!(encoded_str.contains("content-length: 28\r\n"));
        assert!(encoded_str.ends_with("\r\n\r\nHello from Agam HTTP Engine!"));
    }

    #[test]
    fn test_http_router_dispatch() {
        let mut router = HttpRouter::new();
        router.get("/health", |_| HttpResponse::text("healthy"));
        router.post("/echo", |req| {
            let body_str = String::from_utf8_lossy(&req.body).to_string();
            HttpResponse::text(body_str)
        });

        let req_health = HttpRequest::new(HttpMethod::Get, "/health");
        let resp_health = router.dispatch(&req_health);
        assert_eq!(resp_health.status, HttpStatus::OK);
        assert_eq!(resp_health.body, b"healthy");

        let req_echo = HttpRequest::new(HttpMethod::Post, "/echo").with_body(b"test-payload");
        let resp_echo = router.dispatch(&req_echo);
        assert_eq!(resp_echo.status, HttpStatus::OK);
        assert_eq!(resp_echo.body, b"test-payload");

        let req_missing = HttpRequest::new(HttpMethod::Get, "/nonexistent");
        let resp_missing = router.dispatch(&req_missing);
        assert_eq!(resp_missing.status, HttpStatus::NOT_FOUND);
    }
}
