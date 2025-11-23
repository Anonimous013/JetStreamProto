use crate::udp::UdpTransport;
use jsp_core::session::Session;
use jsp_core::types::control::SessionConfig;
use jsp_core::types::connection_id::ConnectionId;
use jsp_core::types::path_validation::{PathChallenge, PathResponse};
use jsp_core::types::header::{Header, FRAME_TYPE_PATH_CHALLENGE, FRAME_TYPE_PATH_RESPONSE};
use jsp_core::compression::header_compression::HeaderCompressor;
use anyhow::Result;
use std::net::SocketAddr;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::rate_limit::GlobalRateLimiter;
use crate::ddos_protection::DdosProtection;
use crate::config::ServerConfig;
use bytes::BytesMut;

pub struct ServerConnectionState {
    pub session: Session,
    pub peer_addr: SocketAddr,
    pub last_activity: std::time::Instant,
    pub pending_challenge: Option<([u8; 8], std::time::Instant)>, // token, timestamp
    pub header_compressor: Option<HeaderCompressor>,
    pub header_decompressor: Option<HeaderCompressor>,
}

pub struct Server {
    transport: UdpTransport,
    connections: Arc<RwLock<HashMap<ConnectionId, ServerConnectionState>>>,
    addr_map: Arc<RwLock<HashMap<SocketAddr, ConnectionId>>>,
    next_session_id: Arc<RwLock<u64>>,
    config: ServerConfig,
    global_rate_limiter: Option<GlobalRateLimiter>,
    ddos_protection: Option<DdosProtection>,
    cleanup_task: Option<tokio::task::JoinHandle<()>>,
}

impl Server {
    pub async fn bind(addr: &str) -> Result<Self> {
        Self::bind_with_config(addr, ServerConfig::default()).await
    }

    pub async fn bind_with_config(addr: &str, config: ServerConfig) -> Result<Self> {
        let transport = UdpTransport::bind(addr).await?;
        
        let global_rate_limiter = if let (Some(msg_limit), Some(byte_limit)) = 
            (config.global_rate_limit_messages, config.global_rate_limit_bytes) {
            Some(GlobalRateLimiter::new(msg_limit, byte_limit))
        } else {
            None
        };
        
        let ddos_protection = Some(DdosProtection::new(config.ddos_config.clone()));
        
        tracing::info!(addr, "Server bound");
        
        let mut server = Self {
            transport,
            connections: Arc::new(RwLock::new(HashMap::new())),
            addr_map: Arc::new(RwLock::new(HashMap::new())),
            next_session_id: Arc::new(RwLock::new(1)),
            config,
            global_rate_limiter,
            ddos_protection,
            cleanup_task: None,
        };
        
        server.start_cleanup_task();
        
        Ok(server)
    }

    /// Start background task to cleanup expired sessions
    fn start_cleanup_task(&mut self) {
        let connections = self.connections.clone();
        let addr_map = self.addr_map.clone();
        let interval = self.config.cleanup_interval;
        
        let task = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            
            loop {
                ticker.tick().await;
                
                let mut connections_lock = connections.write().await;
                let mut addr_map_lock = addr_map.write().await;
                let initial_count = connections_lock.len();
                
                // Remove expired sessions
                connections_lock.retain(|id, state| {
                    let expired = state.session.is_expired();
                    if expired {
                        tracing::info!(
                            peer = %state.peer_addr,
                            connection_id = %id,
                            session_id = state.session.session_id,
                            "Session expired and removed"
                        );
                        // Remove from addr_map
                        addr_map_lock.remove(&state.peer_addr);
                    }
                    !expired
                });
                
                let removed = initial_count - connections_lock.len();
                if removed > 0 {
                    tracing::debug!(
                        removed,
                        remaining = connections_lock.len(),
                        "Session cleanup completed"
                    );
                }
            }
        });
        
        self.cleanup_task = Some(task);
    }

    /// Get the local address the server is bound to
    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.transport.local_addr()
    }

    pub async fn accept(&mut self) -> Result<(SocketAddr, Session)> {
        let mut buf = BytesMut::with_capacity(2048);
        buf.resize(2048, 0);
        let (len, src_addr) = self.transport.recv_from(&mut buf).await?;
        buf.truncate(len);
        let data = buf.freeze();
        
        // Check global rate limit
        if let Some(ref limiter) = self.global_rate_limiter {
            if !limiter.check_and_consume(len) {
                tracing::warn!(peer = %src_addr, "Global rate limit exceeded");
                return Err(anyhow::anyhow!("Global rate limit exceeded"));
            }
        }
        
        let mut connections = self.connections.write().await;
        let mut addr_map = self.addr_map.write().await;
        
        // Check if we already know this address
        if let Some(conn_id) = addr_map.get(&src_addr) {
            if let Some(_state) = connections.get(conn_id) {
                // Existing session found via address
                // Create a copy for return (simplified)
                let session_copy = Session::new(); // In real impl, we might need more state
                return Ok((src_addr, session_copy));
            }
        }
        
        // If not found by address, check if it's a ClientHello to establish new session
        let session_config = SessionConfig {
            timeout_secs: self.config.connection.session_timeout.as_secs(),
            heartbeat_interval_secs: self.config.connection.heartbeat_interval.as_secs(),
            heartbeat_timeout_count: self.config.connection.heartbeat_timeout_count,
            max_streams: self.config.connection.max_streams,
            enable_replay_protection: true,
            replay_window_size: 10000,
            max_clock_skew_secs: 300,
        };
            let mut session = Session::with_config(session_config);
            
            // Process ClientHello
            let client_hello = session.process_client_hello(&data)?;
            
            // Check DDoS protection for handshake
            if let Some(ref ddos) = self.ddos_protection {
                if !ddos.check_handshake(src_addr.ip()).await {
                    tracing::warn!(peer = %src_addr, "Handshake rejected by DDoS protection");
                    return Err(anyhow::anyhow!("Handshake rejected"));
                }
            }
            
            // Select cipher suite
            // Prefer ChaCha20-Poly1305 (0x1303), then AES-256-GCM (0x1302)
            let cipher_suite = if client_hello.cipher_suites.contains(&0x1303) {
                0x1303
            } else if client_hello.cipher_suites.contains(&0x1302) {
                0x1302
            } else {
                // Fallback to first available or default
                client_hello.cipher_suites.first().copied().unwrap_or(0x1303)
            };
            
            // Generate ServerHello
            let mut next_id = self.next_session_id.write().await;
            let session_id = *next_id;
            *next_id += 1;
            drop(next_id);
            
            let (server_hello, kyber_shared) = session.generate_server_hello(
                session_id,
                cipher_suite,
                &client_hello.kyber_public_key,
                &client_hello.supported_formats  // Pass client's supported formats
            )?;
            
            // Derive keys
            session.derive_keys_from_client_hello(&client_hello.public_key, Some(&kyber_shared));
            
            // Send ServerHello
            self.transport.send_to(&server_hello, src_addr).await?;
            
            tracing::info!(
                peer = %src_addr,
                session_id,
                "New session established"
            );
            
            // Store session
            let session_copy = Session::with_config(session_config);
            
            // Create ConnectionId (for now, generate one or use session_id if we map it)
            // In a real implementation, ConnectionId should be negotiated or derived
            // For this phase, we'll generate a new one
            let connection_id = ConnectionId::generate();
            
            let state = ServerConnectionState {
                session,
                peer_addr: src_addr,
                last_activity: std::time::Instant::now(),
                pending_challenge: None,
                header_compressor: if self.config.connection.enable_header_compression { Some(HeaderCompressor::new()) } else { None },
                header_decompressor: if self.config.connection.enable_header_compression { Some(HeaderCompressor::new()) } else { None },
            };
            
            connections.insert(connection_id, state);
            addr_map.insert(src_addr, connection_id);
            
            Ok((src_addr, session_copy))
    }

    pub async fn get_session(&self, addr: &SocketAddr) -> Option<Session> {
        let addr_map = self.addr_map.read().await;
        if let Some(conn_id) = addr_map.get(addr) {
            let connections = self.connections.read().await;
            connections.get(conn_id).map(|_s| Session::new())
        } else {
            None
        }
    }

    pub async fn send_to(&mut self, data: &[u8], addr: SocketAddr) -> Result<()> {
        // Check global rate limit
        if let Some(ref limiter) = self.global_rate_limiter {
            if !limiter.check_and_consume(data.len()) {
                tracing::warn!(peer = %addr, "Global rate limit exceeded");
                return Err(anyhow::anyhow!("Global rate limit exceeded"));
            }
        }
        
        self.transport.send_to(data, addr).await?;
        Ok(())
    }

    pub async fn recv_from(&mut self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        let (len, addr) = self.transport.recv_from(buf).await?;
        
        // Check DDoS protection
        if let Some(ref ddos) = self.ddos_protection {
            if !ddos.check_packet(addr.ip(), len).await {
                // Packet rejected
                // We return a length of 0 to indicate no data, or we could loop and receive next
                // For simplicity, let's just drop it and return 0 length, caller should handle
                // But recv_from expects to return data. Better to loop here?
                // Or return an error?
                // Let's return a specific error so the loop can continue
                return Err(anyhow::anyhow!("DDoS protection rejected packet"));
            }
        }
        
        // Try to parse header to check for ConnectionId
        // Note: This is a partial parse just to get ConnectionId and Frame Type
        // In a full implementation, we'd want a more efficient way or do this in Connection
        if len > 2 {
            let header_len = u16::from_be_bytes([buf[0], buf[1]]) as usize;
            if len >= 2 + header_len {
                if let Ok(header) = serde_cbor::from_slice::<Header>(&buf[2..2+header_len]) {
                    if let Some(conn_id) = header.connection_id {
                        self.handle_connection_packet(conn_id, addr, &header, &buf[2+header_len..len]).await?;
                    }
                }
            }
        }
        
        Ok((len, addr))
    }

    async fn handle_connection_packet(&mut self, conn_id: ConnectionId, addr: SocketAddr, header: &Header, payload: &[u8]) -> Result<()> {
        let mut connections = self.connections.write().await;
        let mut addr_map = self.addr_map.write().await;
        
        if let Some(state) = connections.get_mut(&conn_id) {
            // Check if address changed
            if state.peer_addr != addr {
                // Address changed!
                
                // Check if this is a PathResponse
                if header.msg_type == FRAME_TYPE_PATH_RESPONSE {
                    if let Ok(response) = serde_cbor::from_slice::<PathResponse>(payload) {
                        if let Some((token, _ts)) = state.pending_challenge {
                            if response.token == token {
                                // Validation successful! Update address
                                tracing::info!(
                                    old_peer = %state.peer_addr,
                                    new_peer = %addr,
                                    connection_id = %conn_id,
                                    "Connection migrated to new address"
                                );
                                
                                // Remove old mapping
                                addr_map.remove(&state.peer_addr);
                                
                                // Update state
                                state.peer_addr = addr;
                                state.pending_challenge = None;
                                
                                // Add new mapping
                                addr_map.insert(addr, conn_id);
                            }
                        }
                    }
                } else {
                    // Not a response, trigger validation
                    // Only send challenge if not already pending or timed out (e.g. 1s)
                    let should_challenge = state.pending_challenge
                        .map(|(_, ts)| ts.elapsed().as_secs() >= 1)
                        .unwrap_or(true);
                        
                    if should_challenge {
                        tracing::info!(
                            peer = %addr,
                            connection_id = %conn_id,
                            "New address detected, sending path challenge"
                        );
                        
                        let mut token = [0u8; 8];
                        getrandom::getrandom(&mut token).expect("Failed to generate random token");
                        state.pending_challenge = Some((token, std::time::Instant::now()));
                        
                        // Send Challenge
                        let challenge = PathChallenge { token };
                        let payload = serde_cbor::to_vec(&challenge)?;
                        
                        let header = Header::new(
                            0, 
                            FRAME_TYPE_PATH_CHALLENGE,
                            0,
                            0,
                            0,
                            0,
                            jsp_core::types::delivery::DeliveryMode::Reliable,
                            None,
                            Some(payload.len() as u32)
                        );
                        
                        let header_bytes = serde_cbor::to_vec(&header)?;
                        let header_len = header_bytes.len() as u16;
                        
                        let mut packet = BytesMut::with_capacity(2 + header_bytes.len() + payload.len());
                        packet.extend_from_slice(&header_len.to_be_bytes());
                        packet.extend_from_slice(&header_bytes);
                        packet.extend_from_slice(&payload);
                        
                        self.transport.send_to(&packet, addr).await?;
                    }
                }
            }
            
            state.last_activity = std::time::Instant::now();
        }
        
        Ok(())
    }

    pub async fn recv_packet(&mut self) -> Result<(Header, Vec<u8>, SocketAddr)> {
        let mut buf = BytesMut::with_capacity(2048);
        buf.resize(2048, 0);
        let (len, addr) = self.transport.recv_from(&mut buf).await?;
        buf.truncate(len);
        
        if len < 2 {
            return Err(anyhow::anyhow!("Packet too short"));
        }
        
        let header_len = u16::from_be_bytes([buf[0], buf[1]]) as usize;
        if len < 2 + header_len {
            return Err(anyhow::anyhow!("Incomplete packet"));
        }
        
        let header_data = &buf[2..2+header_len];
        let payload = buf[2+header_len..].to_vec();
        
        let mut connections = self.connections.write().await;
        let mut addr_map = self.addr_map.write().await;
        
        // Try to find session by address
        let header = if let Some(conn_id) = addr_map.get(&addr).copied() {
            if let Some(state) = connections.get_mut(&conn_id) {
                state.last_activity = std::time::Instant::now();
                
                if let Some(decompressor) = &mut state.header_decompressor {
                    match decompressor.decompress(header_data) {
                        Ok(h) => h,
                        Err(_) => serde_cbor::from_slice(header_data)?
                    }
                } else {
                    serde_cbor::from_slice(header_data)?
                }
            } else {
                serde_cbor::from_slice(header_data)?
            }
        } else {
            // Unknown address, try standard CBOR
            let header: Header = serde_cbor::from_slice(header_data)?;
            
            // Check for migration
            if let Some(conn_id) = header.connection_id {
                if let Some(state) = connections.get_mut(&conn_id) {
                    if state.peer_addr != addr {
                        // Address changed!
                        
                        // Check if this is a PathResponse
                        if header.msg_type == FRAME_TYPE_PATH_RESPONSE {
                            if let Ok(response) = serde_cbor::from_slice::<PathResponse>(&payload) {
                                if let Some((token, _ts)) = state.pending_challenge {
                                    if response.token == token {
                                        tracing::info!(
                                            old_peer = %state.peer_addr,
                                            new_peer = %addr,
                                            connection_id = %conn_id,
                                            "Connection migrated to new address"
                                        );
                                        
                                        addr_map.remove(&state.peer_addr);
                                        state.peer_addr = addr;
                                        state.pending_challenge = None;
                                        addr_map.insert(addr, conn_id);
                                    }
                                }
                            }
                        } else {
                            // Not a response, trigger validation
                            let should_challenge = state.pending_challenge
                                .map(|(_, ts)| ts.elapsed().as_secs() >= 1)
                                .unwrap_or(true);
                                
                            if should_challenge {
                                tracing::info!(
                                    peer = %addr,
                                    connection_id = %conn_id,
                                    "New address detected, sending path challenge"
                                );
                                
                                let mut token = [0u8; 8];
                                getrandom::getrandom(&mut token).expect("Failed to generate random token");
                                state.pending_challenge = Some((token, std::time::Instant::now()));
                                
                                let challenge = PathChallenge { token };
                                let challenge_payload = serde_cbor::to_vec(&challenge)?;
                                
                                let challenge_header = Header::new(
                                    0, 
                                    FRAME_TYPE_PATH_CHALLENGE,
                                    0,
                                    0,
                                    0,
                                    0,
                                    jsp_core::types::delivery::DeliveryMode::Reliable,
                                    None,
                                    Some(challenge_payload.len() as u32)
                                );
                                
                                let header_bytes = serde_cbor::to_vec(&challenge_header)?;
                                let header_len = header_bytes.len() as u16;
                                
                                let mut packet = BytesMut::with_capacity(2 + header_bytes.len() + challenge_payload.len());
                                packet.extend_from_slice(&header_len.to_be_bytes());
                                packet.extend_from_slice(&header_bytes);
                                packet.extend_from_slice(&challenge_payload);
                                
                                self.transport.send_to(&packet, addr).await?;
                            }
                        }
                    }
                }
            }
            header
        };
        
        Ok((header, payload, addr))
    }

    /// Gracefully shutdown the server
    pub async fn shutdown(&mut self) -> Result<()> {
        tracing::info!("Server shutting down");
        
        // Stop cleanup task
        if let Some(task) = self.cleanup_task.take() {
            task.abort();
        }
        
        // Clear all sessions
        let mut connections = self.connections.write().await;
        let mut addr_map = self.addr_map.write().await;
        let count = connections.len();
        connections.clear();
        addr_map.clear();
        
        tracing::info!(sessions_closed = count, "Server shutdown complete");
        
        Ok(())
    }

    /// Get current session count
    pub async fn session_count(&self) -> usize {
        self.connections.read().await.len()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if let Some(task) = self.cleanup_task.take() {
            task.abort();
        }
    }
}
