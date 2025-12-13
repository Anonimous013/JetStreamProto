//! Multi-Hop Configuration
//! 
//! Defines configuration structures for multi-hop tunnel chains with support
//! for YAML/JSON serialization and validation.

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use crate::multihop::Result;

/// Top-level multi-hop configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiHopConfig {
    /// Enable multi-hop tunneling
    pub enabled: bool,
    
    /// Chain of hops to establish
    pub chain: Vec<HopConfig>,
    
    /// Connection timeout for each hop (seconds)
    #[serde(default = "default_hop_timeout")]
    pub hop_timeout_secs: u64,
    
    /// Enable automatic failover
    #[serde(default = "default_auto_failover")]
    pub auto_failover: bool,
    
    /// Health check interval (seconds)
    #[serde(default = "default_health_check_interval")]
    pub health_check_interval_secs: u64,
}

fn default_hop_timeout() -> u64 { 10 }
fn default_auto_failover() -> bool { true }
fn default_health_check_interval() -> u64 { 30 }

impl Default for MultiHopConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            chain: Vec::new(),
            hop_timeout_secs: 10,
            auto_failover: true,
            health_check_interval_secs: 30,
        }
    }
}

impl MultiHopConfig {
    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.enabled && self.chain.is_empty() {
            return Err(crate::multihop::MultiHopError::Config(
                "Multi-hop enabled but no hops configured".to_string()
            ));
        }
        
        // Validate each hop
        for (idx, hop) in self.chain.iter().enumerate() {
            hop.validate().map_err(|e| {
                crate::multihop::MultiHopError::Config(
                    format!("Hop {} validation failed: {}", idx, e)
                )
            })?;
        }
        
        Ok(())
    }
    
    /// Load configuration from YAML file
    pub fn from_yaml_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }
    
    /// Load configuration from YAML string
    pub fn from_yaml_str(yaml: &str) -> Result<Self> {
        let config: Self = serde_yaml::from_str(yaml)?;
        config.validate()?;
        Ok(config)
    }
    
    /// Save configuration to YAML file
    pub fn to_yaml_file(&self, path: &str) -> Result<()> {
        let yaml = serde_yaml::to_string(self)?;
        std::fs::write(path, yaml)?;
        Ok(())
    }
}

/// Individual hop configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum HopConfig {
    /// WireGuard hop (entry or exit)
    WireGuard(WireGuardConfig),
    
    /// Shadowsocks hop with obfuscation
    Shadowsocks(ShadowsocksConfig),
    
    /// XRay VLESS/TLS hop
    XRay(XRayConfig),
    
    /// WireGuard exit node
    #[serde(rename = "wireguard_exit")]
    WireGuardExit(WireGuardConfig),
}

impl HopConfig {
    /// Validate hop configuration
    pub fn validate(&self) -> Result<()> {
        match self {
            HopConfig::WireGuard(cfg) | HopConfig::WireGuardExit(cfg) => cfg.validate(),
            HopConfig::Shadowsocks(cfg) => cfg.validate(),
            HopConfig::XRay(cfg) => cfg.validate(),
        }
    }
    
    /// Get hop type name
    pub fn hop_type(&self) -> &str {
        match self {
            HopConfig::WireGuard(_) => "wireguard",
            HopConfig::Shadowsocks(_) => "shadowsocks",
            HopConfig::XRay(_) => "xray",
            HopConfig::WireGuardExit(_) => "wireguard_exit",
        }
    }
}

/// WireGuard hop configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireGuardConfig {
    /// Remote endpoint address
    pub endpoint: String,
    
    /// Private key (base64 encoded)
    pub private_key: String,
    
    /// Peer public key (base64 encoded)
    pub peer_public_key: String,
    
    /// Allowed IPs (CIDR notation)
    #[serde(default = "default_allowed_ips")]
    pub allowed_ips: Vec<String>,
    
    /// Persistent keepalive interval (seconds, 0 = disabled)
    #[serde(default)]
    pub persistent_keepalive: u16,
    
    /// Local listen port (0 = auto)
    #[serde(default)]
    pub listen_port: u16,
}

fn default_allowed_ips() -> Vec<String> {
    vec!["0.0.0.0/0".to_string(), "::/0".to_string()]
}

impl WireGuardConfig {
    pub fn validate(&self) -> Result<()> {
        // Validate endpoint
        let _: SocketAddr = self.endpoint.parse()
            .map_err(|_| crate::multihop::MultiHopError::Config(
                format!("Invalid WireGuard endpoint: {}", self.endpoint)
            ))?;
        
        // Validate keys are not empty
        if self.private_key.is_empty() {
            return Err(crate::multihop::MultiHopError::Config(
                "WireGuard private_key cannot be empty".to_string()
            ));
        }
        
        if self.peer_public_key.is_empty() {
            return Err(crate::multihop::MultiHopError::Config(
                "WireGuard peer_public_key cannot be empty".to_string()
            ));
        }
        
        Ok(())
    }
}

/// Shadowsocks hop configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowsocksConfig {
    /// Remote endpoint address
    pub endpoint: String,
    
    /// Password
    pub password: String,
    
    /// Encryption method
    #[serde(default = "default_ss_method")]
    pub method: String,
    
    /// Obfuscation plugin (none, tls, http)
    #[serde(default = "default_obfs")]
    pub obfs: String,
    
    /// Enable UDP relay
    #[serde(default = "default_udp_relay")]
    pub udp_relay: bool,
    
    /// Local listen port (0 = auto)
    #[serde(default)]
    pub local_port: u16,
}

fn default_ss_method() -> String { "aes-256-gcm".to_string() }
fn default_obfs() -> String { "none".to_string() }
fn default_udp_relay() -> bool { false }

impl ShadowsocksConfig {
    pub fn validate(&self) -> Result<()> {
        // Validate endpoint
        let _: SocketAddr = self.endpoint.parse()
            .map_err(|_| crate::multihop::MultiHopError::Config(
                format!("Invalid Shadowsocks endpoint: {}", self.endpoint)
            ))?;
        
        // Validate password
        if self.password.is_empty() {
            return Err(crate::multihop::MultiHopError::Config(
                "Shadowsocks password cannot be empty".to_string()
            ));
        }
        
        // Validate method
        let valid_methods = [
            "aes-256-gcm", "aes-128-gcm", 
            "chacha20-ietf-poly1305", "xchacha20-ietf-poly1305"
        ];
        if !valid_methods.contains(&self.method.as_str()) {
            return Err(crate::multihop::MultiHopError::Config(
                format!("Invalid Shadowsocks method: {}", self.method)
            ));
        }
        
        // Validate obfs
        let valid_obfs = ["none", "tls", "http"];
        if !valid_obfs.contains(&self.obfs.as_str()) {
            return Err(crate::multihop::MultiHopError::Config(
                format!("Invalid obfs plugin: {}", self.obfs)
            ));
        }
        
        Ok(())
    }
}

/// XRay hop configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XRayConfig {
    /// Remote endpoint address
    pub endpoint: String,
    
    /// Server name (SNI)
    pub server_name: String,
    
    /// UUID for VLESS
    pub uuid: String,
    
    /// Enable TLS
    #[serde(default = "default_tls")]
    pub tls: bool,
    
    /// Use WebSocket transport
    #[serde(default)]
    pub websocket: bool,
    
    /// WebSocket path (if websocket = true)
    #[serde(default = "default_ws_path")]
    pub ws_path: String,
    
    /// Local listen port (0 = auto)
    #[serde(default)]
    pub local_port: u16,
}

fn default_tls() -> bool { true }
fn default_ws_path() -> String { "/".to_string() }

impl XRayConfig {
    pub fn validate(&self) -> Result<()> {
        // Validate endpoint
        let _: SocketAddr = self.endpoint.parse()
            .map_err(|_| crate::multihop::MultiHopError::Config(
                format!("Invalid XRay endpoint: {}", self.endpoint)
            ))?;
        
        // Validate UUID
        if self.uuid.is_empty() {
            return Err(crate::multihop::MultiHopError::Config(
                "XRay UUID cannot be empty".to_string()
            ));
        }
        
        // Validate server_name
        if self.server_name.is_empty() {
            return Err(crate::multihop::MultiHopError::Config(
                "XRay server_name cannot be empty".to_string()
            ));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wireguard_config_validation() {
        let config = WireGuardConfig {
            endpoint: "127.0.0.1:51820".to_string(),
            private_key: "test_private_key".to_string(),
            peer_public_key: "test_public_key".to_string(),
            allowed_ips: vec!["0.0.0.0/0".to_string()],
            persistent_keepalive: 25,
            listen_port: 0,
        };
        
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_shadowsocks_config_validation() {
        let config = ShadowsocksConfig {
            endpoint: "127.0.0.1:8388".to_string(),
            password: "test_password".to_string(),
            method: "aes-256-gcm".to_string(),
            obfs: "tls".to_string(),
            udp_relay: false,
            local_port: 0,
        };
        
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_xray_config_validation() {
        let config = XRayConfig {
            endpoint: "127.0.0.1:443".to_string(),
            server_name: "example.com".to_string(),
            uuid: "test-uuid-1234".to_string(),
            tls: true,
            websocket: false,
            ws_path: "/".to_string(),
            local_port: 0,
        };
        
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_multihop_config_yaml() {
        let yaml = r#"
enabled: true
hop_timeout_secs: 10
auto_failover: true
health_check_interval_secs: 30
chain:
  - type: wireguard
    endpoint: "127.0.0.1:51820"
    private_key: "test_key"
    peer_public_key: "test_peer_key"
  - type: shadowsocks
    endpoint: "127.0.0.1:8388"
    password: "test_password"
    method: "aes-256-gcm"
"#;
        
        let config = MultiHopConfig::from_yaml_str(yaml);
        assert!(config.is_ok());
        
        let config = config.unwrap();
        assert_eq!(config.chain.len(), 2);
    }
}
