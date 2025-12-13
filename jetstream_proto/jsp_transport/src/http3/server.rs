//! HTTP/3 Server

use super::{Request, Response, Frame, FrameType};
use bytes::Bytes;
use std::sync::Arc;

/// HTTP/3 request handler
pub type RequestHandler = Arc<dyn Fn(Request) -> Response + Send + Sync>;

/// HTTP/3 server
pub struct Http3Server {
    handler: RequestHandler,
}

impl Http3Server {
    /// Create a new HTTP/3 server
    pub fn new(handler: RequestHandler) -> Self {
        Self { handler }
    }

    /// Handle incoming frame
    pub fn handle_frame(&self, frame: Frame) -> super::Result<Frame> {
        match frame.frame_type {
            FrameType::Headers => {
                // Parse request from headers
                let request = self.parse_request(frame.payload)?;
                
                // Handle request
                let response = (self.handler)(request);
                
                // Encode response
                Ok(self.encode_response(response))
            }
            FrameType::Data => {
                // Handle data frame
                Ok(Frame::data(Bytes::from("ACK")))
            }
            _ => {
                // Other frame types
                Ok(Frame::settings())
            }
        }
    }

    /// Parse request from headers frame
    fn parse_request(&self, _payload: Bytes) -> super::Result<Request> {
        // Simplified: in production, parse QPACK headers
        Ok(Request::new("GET", "/"))
    }

    /// Encode response to frame
    fn encode_response(&self, response: Response) -> Frame {
        // Simplified: in production, encode with QPACK
        let mut payload = Vec::new();
        payload.extend_from_slice(format!(":status: {}\r\n", response.status).as_bytes());
        
        for (key, value) in &response.headers {
            payload.extend_from_slice(format!("{}: {}\r\n", key, value).as_bytes());
        }
        
        payload.extend_from_slice(b"\r\n");
        payload.extend_from_slice(&response.body);
        
        Frame::headers(Bytes::from(payload))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http3_server() {
        let handler = Arc::new(|_req: Request| {
            Response::ok().text("Hello HTTP/3")
        });

        let server = Http3Server::new(handler);
        
        let request_frame = Frame::headers(Bytes::from("GET / HTTP/3"));
        let response_frame = server.handle_frame(request_frame).unwrap();
        
        assert_eq!(response_frame.frame_type, FrameType::Headers);
    }
}
