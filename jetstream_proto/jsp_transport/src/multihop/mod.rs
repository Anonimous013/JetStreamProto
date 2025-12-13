//! Multi-Hop Tunnel Manager
//! 
//! This module provides a managed multi-hop VPN tunnel system for JetStreamProto,
//! enabling 4-layer anonymity (WireGuard → Shadowsocks → XRay → WireGuard exit)
//! while maintaining high performance through optimized routing and buffer management.

pub mod config;
pub mod engine;
pub mod hop;
pub mod router;
pub mod buffer_pool;
pub mod metrics;
pub mod controller;

// Hop implementations
pub mod hop_wireguard;
pub mod hop_shadowsocks;
pub mod hop_xray;

// Re-exports
pub use config::{MultiHopConfig, HopConfig};
pub use engine::MultiHopEngine;
pub use hop::{Hop, HopHealth, HopStatus};
pub use router::Router;
pub use controller::Controller;

/// Multi-hop module errors
#[derive(Debug, thiserror::Error)]
pub enum MultiHopError {
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Hop error: {0}")]
    Hop(String),
    
    #[error("Routing error: {0}")]
    Routing(String),
    
    #[error("Engine error: {0}")]
    Engine(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_yaml::Error),
}

pub type Result<T> = std::result::Result<T, MultiHopError>;
