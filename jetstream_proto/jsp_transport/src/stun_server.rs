use crate::udp::UdpTransport;
use jsp_core::types::stun::{StunMessage, StunMessageType};
use jsp_core::types::header::{Header, FRAME_TYPE_STUN};
use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::task::JoinHandle;

/// Lightweight STUN server for NAT traversal
pub struct StunServer {
    transport: UdpTransport,
    running: Arc<AtomicBool>,
    task: Option<JoinHandle<()>>,
}

impl StunServer {
    /// Create a new STUN server
    pub async fn new(bind_addr: &str) -> Result<Self> {
        let transport = UdpTransport::bind(bind_addr).await?;
        
        Ok(Self {
            transport,
            running: Arc::new(AtomicBool::new(false)),
            task: None,
        })
    }
    
    /// Start the STUN server
    pub fn start(&mut self) {
        if self.running.load(Ordering::Relaxed) {
            return;
        }
        
        self.running.store(true, Ordering::Relaxed);
        let transport = self.transport.clone();
        let running = Arc::clone(&self.running);
        
        let task = tokio::spawn(async move {
            let mut buf = vec![0u8; 2048];
            
            while running.load(Ordering::Relaxed) {
                match transport.recv_from(&mut buf).await {
                    Ok((len, peer_addr)) => {
                        if let Err(e) = Self::handle_request(&transport, &buf[..len], peer_addr).await {
                            tracing::warn!("STUN request handling failed: {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("STUN server recv error: {}", e);
                        break;
                    }
                }
            }
            
            tracing::info!("STUN server stopped");
        });
        
        self.task = Some(task);
        tracing::info!("STUN server started");
    }
    
    /// Handle a STUN request
    async fn handle_request(transport: &UdpTransport, data: &[u8], peer_addr: SocketAddr) -> Result<()> {
        // Parse header
        if data.len() < 2 {
            return Ok(()); // Ignore malformed
        }
        
        let header_len = u16::from_be_bytes([data[0], data[1]]) as usize;
        if data.len() < 2 + header_len {
            return Ok(()); // Incomplete
        }
        
        let header_bytes = &data[2..2+header_len];
        let header: Header = serde_cbor::from_slice(header_bytes)?;
        
        // Only handle STUN frames
        if header.msg_type != FRAME_TYPE_STUN {
            return Ok(());
        }
        
        // Parse STUN message
        let payload = &data[2+header_len..];
        let stun_msg: StunMessage = serde_cbor::from_slice(payload)?;
        
        // Only respond to binding requests
        if stun_msg.msg_type != StunMessageType::BindingRequest {
            return Ok(());
        }
        
        // Create response with observed address
        let response = StunMessage::binding_response(stun_msg.transaction_id, peer_addr);
        let response_payload = serde_cbor::to_vec(&response)?;
        
        // Create response header
        let response_header = Header::new(
            0, // Control stream
            FRAME_TYPE_STUN,
            0, // No flags
            0, // No sequence for STUN
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis() as u64,
            0, // No nonce
            jsp_core::types::delivery::DeliveryMode::BestEffort,
            None, // No piggyback
            Some(response_payload.len() as u32),
        );
        
        let response_header_bytes = serde_cbor::to_vec(&response_header)?;
        let response_header_len = response_header_bytes.len() as u16;
        
        // Send response
        let mut packet = Vec::with_capacity(2 + response_header_bytes.len() + response_payload.len());
        packet.extend_from_slice(&response_header_len.to_be_bytes());
        packet.extend_from_slice(&response_header_bytes);
        packet.extend_from_slice(&response_payload);
        
        transport.send_to(&packet, peer_addr).await?;
        
        tracing::debug!(
            peer = %peer_addr,
            "STUN binding response sent"
        );
        
        Ok(())
    }
    
    /// Stop the STUN server
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
    
    /// Get the local address
    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.transport.local_addr()
    }
}

impl Drop for StunServer {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_stun_server_creation() {
        let server = StunServer::new("127.0.0.1:0").await;
        assert!(server.is_ok());
    }
}
