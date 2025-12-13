//! WebRTC Configuration

use serde::{Deserialize, Serialize};

/// ICE transport policy
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum IceTransportPolicy {
    /// Use all available candidates (host, srflx, relay)
    All,
    /// Only use relay candidates (TURN)
    Relay,
}

impl Default for IceTransportPolicy {
    fn default() -> Self {
        Self::All
    }
}

/// Bundle policy for media
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BundlePolicy {
    /// Bundle all media on single transport
    MaxBundle,
    /// Use separate transports
    MaxCompat,
}

impl Default for BundlePolicy {
    fn default() -> Self {
        Self::MaxBundle
    }
}

/// TURN server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnServer {
    pub urls: Vec<String>,
    pub username: Option<String>,
    pub credential: Option<String>,
}

/// WebRTC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebRTCConfig {
    /// STUN servers for NAT discovery
    pub stun_servers: Vec<String>,
    
    /// TURN servers for relay
    pub turn_servers: Vec<TurnServer>,
    
    /// ICE transport policy
    pub ice_transport_policy: IceTransportPolicy,
    
    /// Bundle policy
    pub bundle_policy: BundlePolicy,
    
    /// Enable ICE TCP candidates
    pub enable_ice_tcp: bool,
    
    /// Data channel label
    pub data_channel_label: String,
    
    /// Ordered delivery
    pub ordered: bool,
    
    /// Max retransmits (None = reliable)
    pub max_retransmits: Option<u16>,
}

impl Default for WebRTCConfig {
    fn default() -> Self {
        Self {
            stun_servers: vec![
                "stun:stun.l.google.com:19302".to_string(),
                "stun:stun1.l.google.com:19302".to_string(),
            ],
            turn_servers: vec![],
            ice_transport_policy: IceTransportPolicy::All,
            bundle_policy: BundlePolicy::MaxBundle,
            enable_ice_tcp: true,
            data_channel_label: "jetstream_proto".to_string(),
            ordered: true,
            max_retransmits: None, // Reliable by default
        }
    }
}

impl WebRTCConfig {
    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.stun_servers.is_empty() && self.turn_servers.is_empty() {
            return Err("At least one STUN or TURN server required".to_string());
        }
        
        if self.data_channel_label.is_empty() {
            return Err("Data channel label cannot be empty".to_string());
        }
        
        Ok(())
    }
    
    /// Create config for relay-only mode (maximum privacy)
    pub fn relay_only() -> Self {
        Self {
            ice_transport_policy: IceTransportPolicy::Relay,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = WebRTCConfig::default();
        assert!(config.validate().is_ok());
        assert!(!config.stun_servers.is_empty());
        assert_eq!(config.ice_transport_policy, IceTransportPolicy::All);
    }
    
    #[test]
    fn test_relay_only() {
        let config = WebRTCConfig::relay_only();
        assert_eq!(config.ice_transport_policy, IceTransportPolicy::Relay);
    }
}
