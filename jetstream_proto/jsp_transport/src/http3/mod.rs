//! HTTP/3 Compatibility Layer
//! 
//! Provides HTTP/3 support over QUIC transport.

pub mod frame;
pub mod request;
pub mod response;
pub mod server;

pub use frame::{Frame, FrameType};
pub use request::Request;
pub use response::Response;
pub use server::Http3Server;

/// HTTP/3 error types
#[derive(Debug, thiserror::Error)]
pub enum Http3Error {
    #[error("Invalid frame: {0}")]
    InvalidFrame(String),
    
    #[error("Protocol error: {0}")]
    ProtocolError(String),
    
    #[error("Stream error: {0}")]
    StreamError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Http3Error>;
