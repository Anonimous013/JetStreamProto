use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use dashmap::DashMap;
use anyhow::Result;
use crate::balancer::LoadBalancer;

/// Gateway Proxy
pub struct Proxy {
    /// Public facing socket
    pub socket: Arc<UdpSocket>,
    /// Load balancer for backend selection
    balancer: Arc<LoadBalancer>,
    /// Session map: Client Addr -> Backend Addr
    sessions: Arc<DashMap<SocketAddr, SocketAddr>>,
    /// Reverse map: Backend Addr -> Client Addr (for simple NAT)
    /// Note: In a real gateway, we'd need a more complex mapping (ConnectionId based)
    /// or use a proper NAT table with port mapping.
    /// For this prototype, we'll assume 1:1 mapping or use ConnectionId if possible.
    /// But since we are a transparent UDP proxy, we just forward packets.
    /// The issue is how to route replies back to the correct client if multiple clients use the same backend.
    /// The backend sees the Gateway's address.
    /// So we need to map (BackendAddr, GatewayPort) -> ClientAddr.
    /// But Gateway uses one port.
    /// So we need to map (BackendAddr) -> ClientAddr? No, that doesn't work for multiple clients.
    /// 
    /// Solution:
    /// 1. Client -> Gateway: Gateway picks Backend. Stores (ClientAddr) -> BackendAddr.
    ///    Gateway forwards to Backend.
    ///    Backend sees Source = GatewayAddr.
    /// 2. Backend -> Gateway: Gateway receives from Backend.
    ///    How does Gateway know which Client to forward to?
    ///    
    ///    Option A: ConnectionId parsing.
    ///    Gateway parses header, extracts ConnectionId.
    ///    Maintains Map: ConnectionId -> ClientAddr.
    ///    
    ///    Option B: Ephemeral ports (NAT).
    ///    For each Client, Gateway opens a new UDP socket to talk to Backend.
    ///    Client -> Gateway(5000) -> Gateway(RandomPort) -> Backend(8000).
    ///    Backend replies to Gateway(RandomPort).
    ///    Gateway(RandomPort) knows it belongs to Client.
    ///    
    ///    Option B is cleaner for a "transparent" proxy without deep packet inspection.
    ///    Let's implement Option B.
    client_proxies: Arc<DashMap<SocketAddr, Arc<UdpSocket>>>,
}

impl Proxy {
    pub async fn new(bind_addr: &str, balancer: Arc<LoadBalancer>) -> Result<Self> {
        let socket = Arc::new(UdpSocket::bind(bind_addr).await?);
        Ok(Self {
            socket,
            balancer,
            sessions: Arc::new(DashMap::new()),
            client_proxies: Arc::new(DashMap::new()),
        })
    }

    pub async fn run(&self) -> Result<()> {
        tracing::info!("Gateway listening on {}", self.socket.local_addr()?);
        
        let mut buf = [0u8; 65535];
        
        loop {
            let (len, client_addr) = self.socket.recv_from(&mut buf).await?;
            let data = &buf[..len];
            
            // Check if we have a proxy socket for this client
            let proxy_socket = if let Some(socket) = self.client_proxies.get(&client_addr) {
                socket.clone()
            } else {
                // New session
                // 1. Select backend
                let backend_addr = self.balancer.select_backend(client_addr).await;
                tracing::info!("New session: {} -> {}", client_addr, backend_addr);
                
                // 2. Create new ephemeral socket for this client-backend pair
                let new_socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
                new_socket.connect(backend_addr).await?;
                
                // 3. Store mappings
                self.sessions.insert(client_addr, backend_addr);
                self.client_proxies.insert(client_addr, new_socket.clone());
                
                // 4. Spawn task to handle responses from backend
                let socket_clone = new_socket.clone();
                let main_socket = self.socket.clone();
                let client_addr_clone = client_addr;
                
                tokio::spawn(async move {
                    let mut buf = [0u8; 65535];
                    loop {
                        match socket_clone.recv(&mut buf).await {
                            Ok(len) => {
                                let data = &buf[..len];
                                if let Err(e) = main_socket.send_to(data, client_addr_clone).await {
                                    tracing::error!("Failed to forward to client {}: {}", client_addr_clone, e);
                                }
                            }
                            Err(e) => {
                                tracing::error!("Backend connection error for client {}: {}", client_addr_clone, e);
                                break;
                            }
                        }
                    }
                });
                
                new_socket
            };
            
            // Forward to backend via proxy socket
            if let Err(e) = proxy_socket.send(data).await {
                tracing::error!("Failed to forward to backend: {}", e);
                // If error, maybe remove session?
                self.client_proxies.remove(&client_addr);
                self.sessions.remove(&client_addr);
            }
        }
    }
}
