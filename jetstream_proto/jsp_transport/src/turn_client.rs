use std::net::SocketAddr;
use anyhow::Result;
use tracing::{info, warn};
use jsp_core::types::turn::TurnMessage;
use jsp_core::types::header::{Header, FRAME_TYPE_TURN};
use bytes::BytesMut;
use crate::udp::UdpTransport;
use tokio::time::{timeout, Duration};

pub struct TurnClient {
    turn_server_addr: SocketAddr,
    allocation_id: Option<u64>,
    relay_addr: Option<SocketAddr>,
    lifetime: u32,
}

impl TurnClient {
    pub fn new(turn_server_addr: SocketAddr) -> Self {
        Self {
            turn_server_addr,
            allocation_id: None,
            relay_addr: None,
            lifetime: 600, // 10 minutes default
        }
    }
    
    /// Allocate a relay address from TURN server
    pub async fn allocate(&mut self, transport: &UdpTransport) -> Result<SocketAddr> {
        info!("Requesting TURN allocation from {}", self.turn_server_addr);
        
        let request = TurnMessage::Allocate {
            requested_lifetime: self.lifetime,
        };
        
        self.send_turn_message(transport, request).await?;
        
        // Wait for response
        let response_timeout = Duration::from_secs(5);
        let start = std::time::Instant::now();
        
        while start.elapsed() < response_timeout {
            match timeout(Duration::from_millis(500), self.recv_turn_message(transport)).await {
                Ok(Ok(msg)) => {
                    match msg {
                        TurnMessage::AllocateSuccess { relay_addr, lifetime, allocation_id } => {
                            info!("TURN allocation successful: {} (ID: {})", relay_addr, allocation_id);
                            self.allocation_id = Some(allocation_id);
                            self.relay_addr = Some(relay_addr);
                            self.lifetime = lifetime;
                            return Ok(relay_addr);
                        }
                        TurnMessage::AllocateError { code, reason } => {
                            warn!("TURN allocation failed: {} - {}", code, reason);
                            return Err(anyhow::anyhow!("Allocation failed: {}", reason));
                        }
                        _ => continue,
                    }
                }
                Ok(Err(e)) => {
                    warn!("Error receiving TURN response: {}", e);
                }
                Err(_) => continue, // timeout
            }
        }
        
        Err(anyhow::anyhow!("TURN allocation timeout"))
    }
    
    /// Create permission for peer to send data
    pub async fn create_permission(&self, transport: &UdpTransport, peer_addr: SocketAddr) -> Result<()> {
        let allocation_id = self.allocation_id.ok_or_else(|| anyhow::anyhow!("No allocation"))?;
        
        info!("Creating TURN permission for {}", peer_addr);
        
        let request = TurnMessage::CreatePermission {
            allocation_id,
            peer_addr,
        };
        
        self.send_turn_message(transport, request).await?;
        
        // Wait for response
        match timeout(Duration::from_secs(2), self.recv_turn_message(transport)).await {
            Ok(Ok(TurnMessage::PermissionSuccess)) => {
                info!("Permission created for {}", peer_addr);
                Ok(())
            }
            Ok(Ok(TurnMessage::Error { code, reason })) => {
                Err(anyhow::anyhow!("Permission failed: {} - {}", code, reason))
            }
            _ => Err(anyhow::anyhow!("Permission timeout")),
        }
    }
    
    /// Send data through TURN relay
    pub async fn send_data(&self, transport: &UdpTransport, peer_addr: SocketAddr, data: Vec<u8>) -> Result<()> {
        let allocation_id = self.allocation_id.ok_or_else(|| anyhow::anyhow!("No allocation"))?;
        
        let request = TurnMessage::Send {
            allocation_id,
            peer_addr,
            data,
        };
        
        self.send_turn_message(transport, request).await?;
        Ok(())
    }
    
    /// Refresh allocation to keep it alive
    pub async fn refresh(&self, transport: &UdpTransport) -> Result<()> {
        let allocation_id = self.allocation_id.ok_or_else(|| anyhow::anyhow!("No allocation"))?;
        
        let request = TurnMessage::Refresh {
            allocation_id,
            lifetime: self.lifetime,
        };
        
        self.send_turn_message(transport, request).await?;
        Ok(())
    }
    
    pub fn get_relay_addr(&self) -> Option<SocketAddr> {
        self.relay_addr
    }
    
    async fn send_turn_message(&self, transport: &UdpTransport, msg: TurnMessage) -> Result<()> {
        let payload = msg.to_bytes();
        
        let header = Header::new(
            0,
            FRAME_TYPE_TURN,
            0,
            0,
            0,
            0,
            Default::default(),
            None,
            Some(payload.len() as u32),
        );
        
        let header_bytes = serde_cbor::to_vec(&header)?;
        let header_len = header_bytes.len() as u16;
        
        let mut packet = BytesMut::with_capacity(2 + header_bytes.len() + payload.len());
        packet.extend_from_slice(&header_len.to_be_bytes());
        packet.extend_from_slice(&header_bytes);
        packet.extend_from_slice(&payload);
        
        transport.send_to(&packet, self.turn_server_addr).await?;
        Ok(())
    }
    
    async fn recv_turn_message(&self, transport: &UdpTransport) -> Result<TurnMessage> {
        let mut buf = vec![0u8; 65536];
        let (len, _src) = transport.recv_from(&mut buf).await?;
        
        if len < 2 {
            return Err(anyhow::anyhow!("Packet too small"));
        }
        
        let header_len = u16::from_be_bytes([buf[0], buf[1]]) as usize;
        if len < 2 + header_len {
            return Err(anyhow::anyhow!("Invalid header length"));
        }
        
        let header: Header = serde_cbor::from_slice(&buf[2..2 + header_len])?;
        
        if header.msg_type != FRAME_TYPE_TURN {
            return Err(anyhow::anyhow!("Not a TURN message"));
        }
        
        let payload = &buf[2 + header_len..len];
        let msg = TurnMessage::from_bytes(payload)?;
        
        Ok(msg)
    }
}
