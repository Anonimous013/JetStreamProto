use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use anyhow::Result;
use tracing::{info, warn, error};
use jsp_core::types::turn::TurnMessage;
use jsp_core::types::header::{Header, FRAME_TYPE_TURN};
use bytes::BytesMut;

/// Allocation entry for a client
struct Allocation {
    client_addr: SocketAddr,
    relay_addr: SocketAddr,
    #[allow(dead_code)]
    allocation_id: u64,
    created_at: Instant,
    lifetime: Duration,
    permissions: Vec<SocketAddr>, // Allowed peer addresses
}

/// TURN Relay Server
pub struct TurnServer {
    socket: Arc<UdpSocket>,
    allocations: Arc<Mutex<HashMap<u64, Allocation>>>,
    next_allocation_id: Arc<Mutex<u64>>,
    relay_port_range: (u16, u16),
    next_relay_port: Arc<Mutex<u16>>,
}

impl TurnServer {
    pub async fn new(bind_addr: &str, relay_port_range: (u16, u16)) -> Result<Self> {
        let socket = UdpSocket::bind(bind_addr).await?;
        info!("TURN server listening on {}", bind_addr);
        
        Ok(Self {
            socket: Arc::new(socket),
            allocations: Arc::new(Mutex::new(HashMap::new())),
            next_allocation_id: Arc::new(Mutex::new(1)),
            relay_port_range,
            next_relay_port: Arc::new(Mutex::new(relay_port_range.0)),
        })
    }
    
    pub async fn run(&self) -> Result<()> {
        let mut buf = vec![0u8; 65536];
        
        loop {
            let (len, src) = self.socket.recv_from(&mut buf).await?;
            let data = &buf[..len];
            
            // Clone Arc references for the spawned task
            let socket = self.socket.clone();
            let allocations = self.allocations.clone();
            let next_allocation_id = self.next_allocation_id.clone();
            let next_relay_port = self.next_relay_port.clone();
            let relay_port_range = self.relay_port_range;
            let data = data.to_vec();
            
            tokio::spawn(async move {
                if let Err(e) = Self::handle_packet(
                    socket,
                    allocations,
                    next_allocation_id,
                    next_relay_port,
                    relay_port_range,
                    src,
                    data,
                ).await {
                    error!("Error handling packet from {}: {}", src, e);
                }
            });
        }
    }
    
    async fn handle_packet(
        socket: Arc<UdpSocket>,
        allocations: Arc<Mutex<HashMap<u64, Allocation>>>,
        next_allocation_id: Arc<Mutex<u64>>,
        next_relay_port: Arc<Mutex<u16>>,
        relay_port_range: (u16, u16),
        src: SocketAddr,
        data: Vec<u8>,
    ) -> Result<()> {
        // Parse header
        if data.len() < 2 {
            return Ok(());
        }
        
        let header_len = u16::from_be_bytes([data[0], data[1]]) as usize;
        if data.len() < 2 + header_len {
            return Ok(());
        }
        
        let header: Header = serde_cbor::from_slice(&data[2..2 + header_len])?;
        
        if header.msg_type != FRAME_TYPE_TURN {
            return Ok(());
        }
        
        let payload = &data[2 + header_len..];
        let turn_msg = TurnMessage::from_bytes(payload)?;
        
        match turn_msg {
            TurnMessage::Allocate { requested_lifetime } => {
                Self::handle_allocate(
                    socket,
                    allocations,
                    next_allocation_id,
                    next_relay_port,
                    relay_port_range,
                    src,
                    requested_lifetime,
                ).await?;
            }
            
            TurnMessage::Send { allocation_id, peer_addr, data } => {
                Self::handle_send(socket, allocations, src, allocation_id, peer_addr, data).await?;
            }
            
            TurnMessage::Refresh { allocation_id, lifetime } => {
                Self::handle_refresh(socket, allocations, src, allocation_id, lifetime).await?;
            }
            
            TurnMessage::CreatePermission { allocation_id, peer_addr } => {
                Self::handle_create_permission(socket, allocations, src, allocation_id, peer_addr).await?;
            }
            
            _ => {
                warn!("Unexpected TURN message from {}: {:?}", src, turn_msg);
            }
        }
        
        Ok(())
    }
    
    async fn handle_allocate(
        socket: Arc<UdpSocket>,
        allocations: Arc<Mutex<HashMap<u64, Allocation>>>,
        next_allocation_id: Arc<Mutex<u64>>,
        next_relay_port: Arc<Mutex<u16>>,
        relay_port_range: (u16, u16),
        client_addr: SocketAddr,
        requested_lifetime: u32,
    ) -> Result<()> {
        // Allocate relay port
        let mut port_guard = next_relay_port.lock().await;
        let relay_port = *port_guard;
        *port_guard += 1;
        if *port_guard > relay_port_range.1 {
            *port_guard = relay_port_range.0;
        }
        drop(port_guard);
        
        // Create relay address
        let local_ip = socket.local_addr()?.ip();
        let relay_addr = SocketAddr::new(local_ip, relay_port);
        
        // Generate allocation ID
        let mut id_guard = next_allocation_id.lock().await;
        let allocation_id = *id_guard;
        *id_guard += 1;
        drop(id_guard);
        
        // Create allocation
        let allocation = Allocation {
            client_addr,
            relay_addr,
            allocation_id,
            created_at: Instant::now(),
            lifetime: Duration::from_secs(requested_lifetime as u64),
            permissions: Vec::new(),
        };
        
        allocations.lock().await.insert(allocation_id, allocation);
        
        info!("Allocated relay {} for client {} (ID: {})", relay_addr, client_addr, allocation_id);
        
        // Send success response
        let response = TurnMessage::AllocateSuccess {
            relay_addr,
            lifetime: requested_lifetime,
            allocation_id,
        };
        
        Self::send_turn_message(&socket, client_addr, response).await?;
        
        Ok(())
    }
    
    async fn handle_send(
        socket: Arc<UdpSocket>,
        allocations: Arc<Mutex<HashMap<u64, Allocation>>>,
        client_addr: SocketAddr,
        allocation_id: u64,
        peer_addr: SocketAddr,
        data: Vec<u8>,
    ) -> Result<()> {
        let allocs = allocations.lock().await;
        
        if let Some(allocation) = allocs.get(&allocation_id) {
            if allocation.client_addr != client_addr {
                warn!("Allocation mismatch: {} != {}", allocation.client_addr, client_addr);
                return Ok(());
            }
            
            // Check permission
            if !allocation.permissions.contains(&peer_addr) {
                warn!("Permission denied for {} to send to {}", client_addr, peer_addr);
                return Ok(());
            }
            
            // Relay data to peer
            info!("Relaying {} bytes from {} to {} via {}", data.len(), client_addr, peer_addr, allocation.relay_addr);
            
            let data_msg = TurnMessage::Data {
                peer_addr: allocation.relay_addr,
                data,
            };
            
            Self::send_turn_message(&socket, peer_addr, data_msg).await?;
        } else {
            warn!("Allocation {} not found", allocation_id);
        }
        
        Ok(())
    }
    
    async fn handle_refresh(
        socket: Arc<UdpSocket>,
        allocations: Arc<Mutex<HashMap<u64, Allocation>>>,
        client_addr: SocketAddr,
        allocation_id: u64,
        lifetime: u32,
    ) -> Result<()> {
        let mut allocs = allocations.lock().await;
        
        if let Some(allocation) = allocs.get_mut(&allocation_id) {
            if allocation.client_addr != client_addr {
                return Ok(());
            }
            
            allocation.lifetime = Duration::from_secs(lifetime as u64);
            allocation.created_at = Instant::now();
            
            info!("Refreshed allocation {} for {}", allocation_id, client_addr);
            
            let response = TurnMessage::RefreshSuccess { lifetime };
            Self::send_turn_message(&socket, client_addr, response).await?;
        }
        
        Ok(())
    }
    
    async fn handle_create_permission(
        socket: Arc<UdpSocket>,
        allocations: Arc<Mutex<HashMap<u64, Allocation>>>,
        client_addr: SocketAddr,
        allocation_id: u64,
        peer_addr: SocketAddr,
    ) -> Result<()> {
        let mut allocs = allocations.lock().await;
        
        if let Some(allocation) = allocs.get_mut(&allocation_id) {
            if allocation.client_addr != client_addr {
                return Ok(());
            }
            
            if !allocation.permissions.contains(&peer_addr) {
                allocation.permissions.push(peer_addr);
                info!("Created permission for {} to receive from {}", client_addr, peer_addr);
            }
            
            let response = TurnMessage::PermissionSuccess;
            Self::send_turn_message(&socket, client_addr, response).await?;
        }
        
        Ok(())
    }
    
    async fn send_turn_message(socket: &UdpSocket, dest: SocketAddr, msg: TurnMessage) -> Result<()> {
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
        
        socket.send_to(&packet, dest).await?;
        
        Ok(())
    }
}
