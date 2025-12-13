//! Shadowsocks Hop Implementation
//! 
//! Implements Shadowsocks proxy hop with obfuscation support.

use async_trait::async_trait;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::multihop::{
    config::ShadowsocksConfig,
    hop::{Hop, HopHealth, HopStatus, HopStats},
    Result,
};

/// Shadowsocks hop implementation
pub struct ShadowsocksHop {
    /// Configuration
    config: ShadowsocksConfig,
    
    /// Current status
    status: HopStatus,
    
    /// Statistics
    stats: HopStats,
    
    /// Local endpoint (SOCKS5 proxy)
    local_addr: SocketAddr,
    
    /// Remote endpoint (SS server)
    remote_addr: SocketAddr,
    
    /// Local SOCKS5 server socket
    local_socket: Option<Arc<tokio::net::TcpListener>>,
    
    /// Remote connection
    remote_conn: Option<Arc<Mutex<tokio::net::TcpStream>>>,
    
    // TODO: Add shadowsocks-rust client when implementing actual SS logic
}

impl ShadowsocksHop {
    /// Create a new Shadowsocks hop
    pub fn new(config: ShadowsocksConfig) -> Result<Self> {
        config.validate()?;
        
        let remote_addr: SocketAddr = config.endpoint.parse()
            .map_err(|_| crate::multihop::MultiHopError::Config(
                format!("Invalid Shadowsocks endpoint: {}", config.endpoint)
            ))?;
        
        // Determine local address for SOCKS5 proxy
        let local_addr = if config.local_port == 0 {
            "127.0.0.1:0".parse().unwrap()
        } else {
            format!("127.0.0.1:{}", config.local_port).parse().unwrap()
        };
        
        Ok(Self {
            config,
            status: HopStatus::Stopped,
            stats: HopStats::new(),
            local_addr,
            remote_addr,
            local_socket: None,
            remote_conn: None,
        })
    }
}

#[async_trait]
impl Hop for ShadowsocksHop {
    async fn start(&mut self) -> Result<()> {
        self.status = HopStatus::Starting;
        
        tracing::info!(
            endpoint = %self.config.endpoint,
            method = %self.config.method,
            obfs = %self.config.obfs,
            "Starting Shadowsocks hop"
        );
        
        // Bind local SOCKS5 proxy
        let listener = tokio::net::TcpListener::bind(self.local_addr).await?;
        self.local_addr = listener.local_addr()?;
        self.local_socket = Some(Arc::new(listener));
        
        // TODO: Initialize shadowsocks-rust client
        // For now, this is a stub implementation
        
        // Establish connection to remote SS server
        let stream = tokio::net::TcpStream::connect(self.remote_addr).await?;
        self.remote_conn = Some(Arc::new(Mutex::new(stream)));
        
        self.stats.started_at = Some(std::time::Instant::now());
        self.status = HopStatus::Running;
        
        tracing::info!(
            local_addr = %self.local_addr,
            remote_addr = %self.remote_addr,
            "Shadowsocks hop started"
        );
        
        Ok(())
    }
    
    async fn stop(&mut self) -> Result<()> {
        self.status = HopStatus::Stopping;
        
        tracing::info!("Stopping Shadowsocks hop");
        
        // Drop connections
        self.local_socket = None;
        self.remote_conn = None;
        
        self.status = HopStatus::Stopped;
        
        Ok(())
    }
    
    async fn health_check(&self) -> Result<HopHealth> {
        if self.status != HopStatus::Running {
            return Ok(HopHealth::Unhealthy);
        }
        
        // TODO: Implement actual health check (test connection)
        // For now, just check if connection exists
        if self.remote_conn.is_some() {
            Ok(HopHealth::Healthy)
        } else {
            Ok(HopHealth::Unhealthy)
        }
    }
    
    async fn send(&self, data: &[u8]) -> Result<usize> {
        use tokio::io::AsyncWriteExt;
        
        let conn = self.remote_conn.as_ref()
            .ok_or_else(|| crate::multihop::MultiHopError::Hop(
                "Shadowsocks connection not established".to_string()
            ))?;
        
        let mut stream = conn.lock().await;
        
        // TODO: Encrypt with Shadowsocks cipher
        // For now, just forward raw data
        stream.write_all(data).await?;
        
        Ok(data.len())
    }
    
    async fn recv(&self, buf: &mut [u8]) -> Result<usize> {
        use tokio::io::AsyncReadExt;
        
        let conn = self.remote_conn.as_ref()
            .ok_or_else(|| crate::multihop::MultiHopError::Hop(
                "Shadowsocks connection not established".to_string()
            ))?;
        
        let mut stream = conn.lock().await;
        
        // TODO: Decrypt with Shadowsocks cipher
        // For now, just receive raw data
        let received = stream.read(buf).await?;
        
        Ok(received)
    }
    
    fn local_endpoint(&self) -> SocketAddr {
        self.local_addr
    }
    
    fn remote_endpoint(&self) -> SocketAddr {
        self.remote_addr
    }
    
    fn status(&self) -> HopStatus {
        self.status
    }
    
    fn stats(&self) -> &HopStats {
        &self.stats
    }
    
    fn hop_type(&self) -> &str {
        "shadowsocks"
    }
    
    fn supports_udp(&self) -> bool {
        self.config.udp_relay
    }
    
    fn supports_tcp(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shadowsocks_hop_creation() {
        let config = ShadowsocksConfig {
            endpoint: "127.0.0.1:8388".to_string(),
            password: "test_password".to_string(),
            method: "aes-256-gcm".to_string(),
            obfs: "tls".to_string(),
            udp_relay: false,
            local_port: 0,
        };
        
        let hop = ShadowsocksHop::new(config);
        assert!(hop.is_ok());
        
        let hop = hop.unwrap();
        assert_eq!(hop.hop_type(), "shadowsocks");
        assert_eq!(hop.status(), HopStatus::Stopped);
    }
}
