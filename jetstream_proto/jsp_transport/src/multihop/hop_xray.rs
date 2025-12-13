//! XRay Hop Implementation
//! 
//! Implements XRay VLESS/TLS hop with dynamic configuration generation.

use async_trait::async_trait;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::multihop::{
    config::XRayConfig,
    hop::{Hop, HopHealth, HopStatus, HopStats},
    Result,
};

/// XRay hop implementation
pub struct XRayHop {
    /// Configuration
    config: XRayConfig,
    
    /// Current status
    status: HopStatus,
    
    /// Statistics
    stats: HopStats,
    
    /// Local endpoint (SOCKS5 proxy)
    local_addr: SocketAddr,
    
    /// Remote endpoint (XRay server)
    remote_addr: SocketAddr,
    
    /// XRay process handle
    process: Option<tokio::process::Child>,
    
    /// Local SOCKS5 connection
    local_conn: Option<Arc<Mutex<tokio::net::TcpStream>>>,
}

impl XRayHop {
    /// Create a new XRay hop
    pub fn new(config: XRayConfig) -> Result<Self> {
        config.validate()?;
        
        let remote_addr: SocketAddr = config.endpoint.parse()
            .map_err(|_| crate::multihop::MultiHopError::Config(
                format!("Invalid XRay endpoint: {}", config.endpoint)
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
            process: None,
            local_conn: None,
        })
    }
    
    /// Generate XRay configuration JSON
    fn generate_xray_config(&self) -> String {
        let transport = if self.config.websocket {
            format!(r#"
                "streamSettings": {{
                    "network": "ws",
                    "security": "{}",
                    "wsSettings": {{
                        "path": "{}"
                    }}
                }}"#,
                if self.config.tls { "tls" } else { "none" },
                self.config.ws_path
            )
        } else {
            format!(r#"
                "streamSettings": {{
                    "network": "tcp",
                    "security": "{}"
                }}"#,
                if self.config.tls { "tls" } else { "none" }
            )
        };
        
        format!(r#"{{
            "log": {{
                "loglevel": "warning"
            }},
            "inbounds": [{{
                "port": {},
                "protocol": "socks",
                "settings": {{
                    "udp": true
                }}
            }}],
            "outbounds": [{{
                "protocol": "vless",
                "settings": {{
                    "vnext": [{{
                        "address": "{}",
                        "port": {},
                        "users": [{{
                            "id": "{}",
                            "encryption": "none"
                        }}]
                    }}]
                }},
                {}
            }}]
        }}"#,
            self.local_addr.port(),
            self.remote_addr.ip(),
            self.remote_addr.port(),
            self.config.uuid,
            transport
        )
    }
}

#[async_trait]
impl Hop for XRayHop {
    async fn start(&mut self) -> Result<()> {
        self.status = HopStatus::Starting;
        
        tracing::info!(
            endpoint = %self.config.endpoint,
            server_name = %self.config.server_name,
            tls = self.config.tls,
            websocket = self.config.websocket,
            "Starting XRay hop"
        );
        
        // Generate XRay config
        let config_json = self.generate_xray_config();
        
        // Write config to temp file
        let config_path = std::env::temp_dir().join(format!("xray_config_{}.json", std::process::id()));
        std::fs::write(&config_path, config_json)?;
        
        // Start XRay process
        let mut cmd = tokio::process::Command::new("xray");
        cmd.arg("-c").arg(&config_path);
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());
        
        let child = cmd.spawn()
            .map_err(|e| crate::multihop::MultiHopError::Hop(
                format!("Failed to start XRay process: {}. Make sure 'xray' is installed and in PATH.", e)
            ))?;
        
        self.process = Some(child);
        
        // Wait for XRay to start (give it 2 seconds)
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Connect to local SOCKS5 proxy
        let stream = tokio::net::TcpStream::connect(self.local_addr).await
            .map_err(|e| crate::multihop::MultiHopError::Hop(
                format!("Failed to connect to XRay SOCKS5 proxy: {}", e)
            ))?;
        
        self.local_addr = stream.local_addr()?;
        self.local_conn = Some(Arc::new(Mutex::new(stream)));
        
        self.stats.started_at = Some(std::time::Instant::now());
        self.status = HopStatus::Running;
        
        tracing::info!(
            local_addr = %self.local_addr,
            remote_addr = %self.remote_addr,
            "XRay hop started"
        );
        
        Ok(())
    }
    
    async fn stop(&mut self) -> Result<()> {
        self.status = HopStatus::Stopping;
        
        tracing::info!("Stopping XRay hop");
        
        // Drop connection
        self.local_conn = None;
        
        // Kill XRay process
        if let Some(mut process) = self.process.take() {
            let _ = process.kill().await;
        }
        
        self.status = HopStatus::Stopped;
        
        Ok(())
    }
    
    async fn health_check(&self) -> Result<HopHealth> {
        if self.status != HopStatus::Running {
            return Ok(HopHealth::Unhealthy);
        }
        
        // Check if process is still running
        if self.process.is_some() && self.local_conn.is_some() {
            Ok(HopHealth::Healthy)
        } else {
            Ok(HopHealth::Unhealthy)
        }
    }
    
    async fn send(&self, data: &[u8]) -> Result<usize> {
        use tokio::io::AsyncWriteExt;
        
        let conn = self.local_conn.as_ref()
            .ok_or_else(|| crate::multihop::MultiHopError::Hop(
                "XRay connection not established".to_string()
            ))?;
        
        let mut stream = conn.lock().await;
        stream.write_all(data).await?;
        
        Ok(data.len())
    }
    
    async fn recv(&self, buf: &mut [u8]) -> Result<usize> {
        use tokio::io::AsyncReadExt;
        
        let conn = self.local_conn.as_ref()
            .ok_or_else(|| crate::multihop::MultiHopError::Hop(
                "XRay connection not established".to_string()
            ))?;
        
        let mut stream = conn.lock().await;
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
        "xray"
    }
    
    fn supports_udp(&self) -> bool {
        true
    }
    
    fn supports_tcp(&self) -> bool {
        true
    }
}

impl Drop for XRayHop {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.start_kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xray_hop_creation() {
        let config = XRayConfig {
            endpoint: "127.0.0.1:443".to_string(),
            server_name: "example.com".to_string(),
            uuid: "test-uuid-1234".to_string(),
            tls: true,
            websocket: false,
            ws_path: "/".to_string(),
            local_port: 0,
        };
        
        let hop = XRayHop::new(config);
        assert!(hop.is_ok());
        
        let hop = hop.unwrap();
        assert_eq!(hop.hop_type(), "xray");
        assert_eq!(hop.status(), HopStatus::Stopped);
    }
    
    #[test]
    fn test_xray_config_generation() {
        let config = XRayConfig {
            endpoint: "127.0.0.1:443".to_string(),
            server_name: "example.com".to_string(),
            uuid: "test-uuid".to_string(),
            tls: true,
            websocket: true,
            ws_path: "/ws".to_string(),
            local_port: 1080,
        };
        
        let hop = XRayHop::new(config).unwrap();
        let json = hop.generate_xray_config();
        
        assert!(json.contains("vless"));
        assert!(json.contains("test-uuid"));
        assert!(json.contains("ws"));
        assert!(json.contains("/ws"));
    }
}
