//! HTTP/3 Response

use std::collections::HashMap;

/// HTTP/3 response
#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Response {
    /// Create a new response
    pub fn new(status: u16) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    /// Create a 200 OK response
    pub fn ok() -> Self {
        Self::new(200)
    }

    /// Create a 404 Not Found response
    pub fn not_found() -> Self {
        Self::new(404)
    }

    /// Create a 500 Internal Server Error response
    pub fn internal_error() -> Self {
        Self::new(500)
    }

    /// Add a header
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set body
    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }

    /// Set JSON body
    pub fn json(self, json: &str) -> Self {
        self.header("Content-Type", "application/json")
            .body(json.as_bytes().to_vec())
    }

    /// Set text body
    pub fn text(self, text: &str) -> Self {
        self.header("Content-Type", "text/plain")
            .body(text.as_bytes().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_builder() {
        let resp = Response::ok()
            .header("Content-Type", "application/json")
            .json("{\"status\":\"ok\"}");

        assert_eq!(resp.status, 200);
        assert_eq!(resp.headers.get("Content-Type").unwrap(), "application/json");
        assert!(!resp.body.is_empty());
    }

    #[test]
    fn test_response_shortcuts() {
        assert_eq!(Response::ok().status, 200);
        assert_eq!(Response::not_found().status, 404);
        assert_eq!(Response::internal_error().status, 500);
    }
}
