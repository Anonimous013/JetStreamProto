//! WebRTC Transport Module
//! 
//! Provides WebRTC data channel transport for better NAT traversal and browser compatibility.

pub mod ice;
pub mod data_channel;
pub mod signaling;
pub mod transport;
pub mod config;

pub use transport::WebRTCTransport;
pub use config::WebRTCConfig;
pub use ice::{IceCandidate, IceTransportPolicy};
pub use data_channel::DataChannel;

/// WebRTC transport error types
#[derive(Debug, thiserror::Error)]
pub enum WebRTCError {
    #[error("ICE connection failed: {0}")]
    IceConnectionFailed(String),
    
    #[error("Data channel error: {0}")]
    DataChannelError(String),
    
    #[error("Signaling error: {0}")]
    SignalingError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
}
