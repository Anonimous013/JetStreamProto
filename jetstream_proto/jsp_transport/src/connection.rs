use crate::udp::UdpTransport;
use jsp_core::session::Session;
use jsp_core::types::control::{HeartbeatFrame, CloseFrame, CloseReason, AckFrame};
use jsp_core::types::header::{Header, FRAME_TYPE_DATA, FRAME_TYPE_ACK, FRAME_TYPE_STUN, FRAME_TYPE_PATH_CHALLENGE, FRAME_TYPE_PATH_RESPONSE};
use jsp_core::types::stun::{StunMessage, StunMessageType, StunAttribute};
use jsp_core::types::path_validation::{PathChallenge, PathResponse};
use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use bytes::{Bytes, BytesMut};
use tracing::{info, warn};

use crate::reliability::ReliabilityLayer;
use crate::heartbeat::HeartbeatManager;
use crate::rate_limit::RateLimiter;
use crate::memory_pool::PacketPool;
use crate::ice::IceAgent;
use crate::config::ConnectionConfig;
use crate::priority_queue::PriorityQueue;
use jsp_core::qos::QosPriority;

pub struct Connection {
    pub(crate) transport: UdpTransport,
    session: Session,
    reliability: ReliabilityLayer,
    pub peer_addr: SocketAddr,
    pub public_addr: Option<SocketAddr>,
    stun_server_addrs: Vec<SocketAddr>,
    migration_start: Option<std::time::Instant>,
    
    // Heartbeat management
    heartbeat: Arc<HeartbeatManager>,
    heartbeat_task: Option<tokio::task::JoinHandle<()>>,
    
    // Rate limiting
    rate_limiter: RateLimiter,
    
    // Memory pool
    packet_pool: PacketPool,
    
    // Graceful shutdown
    closing: Arc<AtomicBool>,
    
    // Configuration
    config: ConnectionConfig,
    
    // Role
    pub is_server: bool,
    
    // Coalescing
    coalescing_buffer: Arc<Mutex<BytesMut>>,
    last_coalesce_flush: Arc<Mutex<std::time::Instant>>,
    
    // ICE
    ice_agent: Option<IceAgent>,
    flush_task: Option<tokio::task::JoinHandle<()>>,

    // Metrics
    metrics: Arc<crate::metrics::Metrics>,

    // Sender Task
    sender_task: Option<tokio::task::JoinHandle<()>>,
    sender_notify: Arc<tokio::sync::Notify>,
    priority_queue: Arc<Mutex<PriorityQueue<Vec<u8>>>>,

    // Circuit Breaker
    circuit_breaker: Arc<crate::circuit_breaker::CircuitBreaker>,

    // Header Compression
    header_compressor: Option<jsp_core::compression::header_compression::HeaderCompressor>,
    header_decompressor: Option<jsp_core::compression::header_compression::HeaderCompressor>,

    // DDoS Protection
    _ddos_protection: Option<crate::ddos_protection::DdosProtection>,

    // Mobile Optimizations
    pub adaptive_compression: Arc<Mutex<crate::compression::adaptive::AdaptiveCompression>>,
    pub network_status: Arc<crate::network_status::NetworkStatus>,
}

impl Connection {
    pub async fn connect_with_config(addr: &str, config: ConnectionConfig) -> Result<Self> {
        let bind_addr = config.bind_addr.as_deref().unwrap_or("0.0.0.0:0");
        let transport = UdpTransport::bind(bind_addr).await?;
        let peer_addr: SocketAddr = addr.parse()?;
        Self::new_from_transport(transport, peer_addr, config, false).await
    }

    pub async fn bind_with_config(bind_addr: &str, config: ConnectionConfig) -> Result<Self> {
        let transport = UdpTransport::bind(bind_addr).await?;
        let peer_addr: SocketAddr = "0.0.0.0:0".parse()?;
        Self::new_from_transport(transport, peer_addr, config, true).await
    }

    async fn new_from_transport(transport: UdpTransport, peer_addr: SocketAddr, config: ConnectionConfig, is_server: bool) -> Result<Self> {
        let heartbeat_config = crate::heartbeat::HeartbeatConfig {
            foreground_interval: config.heartbeat_interval,
            background_interval: Duration::from_secs(30), // Default background interval
            timeout_count: config.heartbeat_timeout_count,
        };
        let heartbeat = Arc::new(HeartbeatManager::new(heartbeat_config));
        
        let rate_limiter = RateLimiter::new(
            config.rate_limit_messages,
            config.rate_limit_bytes,
        );
        
        let packet_pool = PacketPool::new(
            config.pool_capacity,
            config.pool_max_packet_size,
        );
        
        let stun_server_addrs = config.stun_servers.iter()
            .filter_map(|s| s.parse().ok())
            .collect();

        // Initialize ICE agent
        let peer_id = transport.local_addr()?.to_string();
        let mut agent = IceAgent::new(peer_id);

        let mut connection = Self {
            transport,
            session: Session::new(),
            reliability: ReliabilityLayer::new(),
            peer_addr,
            public_addr: None,
            stun_server_addrs,
            migration_start: None,
            heartbeat,
            heartbeat_task: None,
            rate_limiter,
            packet_pool,
            closing: Arc::new(AtomicBool::new(false)),
            config: config.clone(),
            is_server,
            coalescing_buffer: Arc::new(Mutex::new(BytesMut::with_capacity(1500))),
            last_coalesce_flush: Arc::new(Mutex::new(std::time::Instant::now())),
            ice_agent: None, // Set later
            flush_task: None,
            metrics: Arc::new(crate::metrics::Metrics::new()),
            sender_task: None,
            sender_notify: Arc::new(tokio::sync::Notify::new()),
            priority_queue: Arc::new(Mutex::new(PriorityQueue::new())),
            circuit_breaker: Arc::new(crate::circuit_breaker::CircuitBreaker::new(Default::default())),
            header_compressor: None,
            header_decompressor: None,
            _ddos_protection: None,
            adaptive_compression: Arc::new(Mutex::new(crate::compression::adaptive::AdaptiveCompression::default_config())),
            network_status: Arc::new(crate::network_status::NetworkStatus::new()),
        };

        if !is_server {
            // Gather candidates (STUN)
            agent.gather_candidates(&mut connection).await?;
            
            // Wait for remote candidates (with timeout)
            let wait_timeout = Duration::from_secs(5);
            let start = std::time::Instant::now();
            
            while start.elapsed() < wait_timeout {
                if let Some(sig) = &mut agent.signaling {
                    match tokio::time::timeout(Duration::from_millis(500), sig.recv()).await {
                        Ok(Ok(msg)) => {
                            agent.process_signaling_message(msg).await?;
                            
                            // If we have at least one remote candidate, try connectivity checks
                            if agent.remote_candidates_count() > 0 {
                                info!("Received remote candidates, performing connectivity checks...");
                                if let Some(selected) = agent.perform_connectivity_checks(&mut connection).await? {
                                    info!("P2P connection established via {}", selected);
                                    // Update peer_addr to use the selected candidate
                                    connection.peer_addr = selected;
                                    break;
                                }
                            }
                        }
                        Ok(Err(e)) => {
                            warn!("Error receiving from signaling: {}", e);
                        }
                        Err(_) => continue, // timeout, keep waiting
                    }
                }
            }
        }
        
        connection.ice_agent = Some(agent);
        Ok(connection)
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.transport.local_addr()
    }

    /// Migrate connection to a new local address
    pub async fn migrate(&mut self, new_bind_addr: &str) -> Result<()> {
        let new_transport = UdpTransport::bind(new_bind_addr).await?;
        self.transport = new_transport;
        tracing::info!("Connection migrated to local address: {}", new_bind_addr);
        self.migration_start = Some(std::time::Instant::now());
        
        // Send a probe packet (PathChallenge) to peer to update their view of our address
        // Or just send next data packet. 
        // For now, let's send a PathChallenge to be proactive
        let mut token = [0u8; 8];
        getrandom::getrandom(&mut token).expect("Failed to generate random token");
        let challenge = PathChallenge { token };
        let payload = serde_cbor::to_vec(&challenge)?;
        
        let mut header = Header::new(
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
        header.connection_id = Some(jsp_core::types::connection_id::ConnectionId::from_u64(self.session.session_id));
        
        // Do NOT compress migration packet so server can identify connection from new address
        let header_bytes = serde_cbor::to_vec(&header)?;
        let header_len = header_bytes.len() as u16;
        
        let mut packet = self.packet_pool.acquire();
        packet.reserve(2 + header_bytes.len() + payload.len());
        packet.extend_from_slice(&header_len.to_be_bytes());
        packet.extend_from_slice(&header_bytes);
        packet.extend_from_slice(&payload);
        
        self.transport.send_to(&packet, self.peer_addr).await?;
        self.packet_pool.release(packet);
        
        Ok(())
    }

    pub async fn discover_public_address(&mut self) -> Result<Option<SocketAddr>> {
        if self.stun_server_addrs.is_empty() {
            return Ok(None);
        }

        let servers = self.stun_server_addrs.clone();

        for server_addr in servers {
            let req = StunMessage::binding_request();
            let payload = req.to_bytes();
            
            // We need to wrap this in a STUN frame
            #[allow(clippy::too_many_arguments)]
            let header = Header::new(
                0,
                FRAME_TYPE_STUN,
                0,
                0,
                0,
                0,
                Default::default(),
                None,
                Some(payload.len() as u32)
            );
            
            let header_bytes = serde_cbor::to_vec(&header)?;
            let header_len = header_bytes.len() as u16;
            
            let mut packet = BytesMut::with_capacity(2 + header_bytes.len() + payload.len());
            packet.extend_from_slice(&header_len.to_be_bytes());
            packet.extend_from_slice(&header_bytes);
            packet.extend_from_slice(&payload);
            
            self.transport.send_to(&packet, server_addr).await?;
            
            // Wait for response with timeout
            let timeout = self.config.stun_timeout;
            let start = std::time::Instant::now();
            
            while start.elapsed() < timeout {
                match tokio::time::timeout(Duration::from_millis(100), self.recv()).await {
                    Ok(Ok(_)) => {
                        if let Some(addr) = self.public_addr {
                            return Ok(Some(addr));
                        }
                    }
                    Ok(Err(e)) => tracing::warn!("Error receiving during STUN: {}", e),
                    Err(_) => continue, // timeout
                }
            }
        }
        
        Ok(None)
    }

    pub async fn listen(bind_addr: &str) -> Result<Self> {
        Self::listen_with_config(bind_addr, ConnectionConfig::default()).await
    }

    pub async fn listen_with_config(bind_addr: &str, config: ConnectionConfig) -> Result<Self> {
        let mut connection = Self::bind_with_config(bind_addr, config).await?;
        tracing::info!(bind_addr, "Listening for incoming connection");
        
        connection.handshake().await?;
        
        Ok(connection)
    }

    pub async fn handshake(&mut self) -> Result<()> {
        if self.is_server {
            // Server side handshake
            tracing::info!("Waiting for incoming handshake...");
            
            // Wait for ClientHello
            let mut buf = [0u8; 2048];
            let (len, peer_addr) = self.transport.recv_from(&mut buf).await?;
            
            // Update peer address
            self.peer_addr = peer_addr;
            
            let client_hello = self.session.process_client_hello(&buf[..len])?;
            
            // Select cipher suite
            let cipher_suite = client_hello.cipher_suites
                .first()
                .copied()
                .unwrap_or(0x1301);
                
            // Generate ServerHello
            // For simple Connection, we use session_id 1 or random
            let session_id = 1;
            let (server_hello, kyber_shared) = self.session.generate_server_hello(
                session_id,
                cipher_suite,
                &client_hello.kyber_public_key,
                &client_hello.supported_formats  // Pass client's supported formats
            )?;
            
            // Derive keys
            self.session.derive_keys_from_client_hello(&client_hello.public_key, Some(&kyber_shared));
            
            // Send ServerHello
            self.transport.send_to(&server_hello, peer_addr).await?;
            
            tracing::info!(
                peer = %peer_addr,
                session_id,
                "Handshake accepted"
            );
        } else {
            // Client side handshake
            let hello = self.session.generate_client_hello()?;
            self.transport.send_to(&hello, self.peer_addr).await?;
            
            tracing::info!(peer = %self.peer_addr, "Handshake initiated");
            
            // Simple wait for response (blocking for now, should be loop)
            let mut buf = [0u8; 2048];
            let (len, _src) = self.transport.recv_from(&mut buf).await?;
            
            self.session.process_server_hello(&buf[..len])?;
            
            tracing::info!(
                peer = %self.peer_addr,
                session_id = self.session.session_id,
                "Handshake completed"
            );
        }
        
        // Start heartbeat after successful handshake
        self.start_heartbeat();
        
        // Start flush task if coalescing is enabled
        self.start_flush_task();
        
        // Start sender task for QoS
        self.start_sender_task();
        
        Ok(())
    }

    /// Update application state (Foreground/Background)
    pub async fn set_app_state(&self, state: crate::heartbeat::AppState) {
        self.heartbeat.set_app_state(state).await;
    }

    /// Update network type
    pub async fn set_network_type(&self, net_type: crate::network_status::NetworkType) {
        self.network_status.set_network_type(net_type).await;
    }
    /// Get reference to heartbeat manager (for testing/monitoring)
    pub fn heartbeat(&self) -> &HeartbeatManager {
        &self.heartbeat
    }

    /// Start heartbeat task
    fn start_heartbeat(&mut self) {
        let heartbeat = Arc::clone(&self.heartbeat);
        let transport = self.transport.clone();
        let peer_addr = self.peer_addr;
        let closing = Arc::clone(&self.closing);
        let interval_duration = Duration::from_secs(self.config.heartbeat_interval.as_secs());
        
        if interval_duration.as_secs() == 0 {
            return;
        }

        let task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval_duration);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            
            loop {
                interval.tick().await;
                
                if closing.load(Ordering::Relaxed) {
                    break;
                }
                
                if heartbeat.is_timed_out().await {
                    tracing::warn!(peer = %peer_addr, "Connection timed out");
                    break;
                }
                
                if heartbeat.should_send().await {
                    let seq = heartbeat.next_sequence().await;
                    let ping = HeartbeatFrame::ping(seq);
                    
                    if let Ok(data) = serde_cbor::to_vec(&ping) {
                        if transport.send_to(&data, peer_addr).await.is_ok() {
                            heartbeat.mark_sent().await;
                            tracing::debug!(peer = %peer_addr, seq, "Heartbeat sent");
                        }
                    }
                }
            }
        });
        
        self.heartbeat_task = Some(task);
    }

    /// Start background flush task for coalescing
    fn start_flush_task(&mut self) {
        if self.config.coalescing_window_ms == 0 {
            return; // Coalescing disabled
        }
        
        let buffer = Arc::clone(&self.coalescing_buffer);
        let last_flush = Arc::clone(&self.last_coalesce_flush);
        let transport = self.transport.clone();
        let peer_addr = self.peer_addr;
        let window_ms = self.config.coalescing_window_ms;
        let closing = Arc::clone(&self.closing);
        
        let task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(window_ms / 2));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            
            loop {
                interval.tick().await;
                
                if closing.load(Ordering::Relaxed) {
                    break;
                }
                
                // Check if flush is needed
                let should_flush = {
                    let last = last_flush.lock().unwrap();
                    let buf = buffer.lock().unwrap();
                    !buf.is_empty() && last.elapsed().as_millis() as u64 >= window_ms
                };
                
                if should_flush {
                    // Flush the buffer
                    let data = {
                        let mut buf = buffer.lock().unwrap();
                        if buf.is_empty() {
                            continue;
                        }
                        let data = buf.clone();
                        buf.clear();
                        data
                    };
                    
                    if let Err(e) = transport.send_to(&data, peer_addr).await {
                        tracing::warn!("Background flush failed: {}", e);
                    } else {
                        *last_flush.lock().unwrap() = std::time::Instant::now();
                        tracing::trace!("Background flush sent {} bytes", data.len());
                    }
                }
            }
        });
        
        self.flush_task = Some(task);
    }

    /// Start background sender task for QoS
    fn start_sender_task(&mut self) {
        let priority_queue: Arc<Mutex<PriorityQueue<Vec<u8>>>> = Arc::clone(&self.priority_queue);
        let metrics = Arc::clone(&self.metrics);
        let sender_notify = Arc::clone(&self.sender_notify);
        let transport = self.transport.clone();
        let peer_addr = self.peer_addr;
        let closing = Arc::clone(&self.closing);
        let circuit_breaker = Arc::clone(&self.circuit_breaker);
        
        // For coalescing integration
        let coalescing_buffer = Arc::clone(&self.coalescing_buffer);
        let last_flush = Arc::clone(&self.last_coalesce_flush);
        let config = self.config.clone();
        
        let task = tokio::spawn(async move {
            loop {
                // Wait for notification
                sender_notify.notified().await;
                
                if closing.load(Ordering::Relaxed) {
                    break;
                }
                
                // Drain queue
                loop {
                    let packet = {
                        let mut queue = priority_queue.lock().unwrap();
                        queue.dequeue()
                    };
                    
                    match packet {
                        Some(data) => {
                            // Check if coalescing is enabled
                            if config.coalescing_window_ms > 0 {
                                let should_flush = {
                                    let mut buf = coalescing_buffer.lock().unwrap();
                                    let last = last_flush.lock().unwrap();
                                    let now = std::time::Instant::now();
                                    
                                    // If adding this packet would exceed max size, or time window passed
                                    if buf.len() + data.len() > config.pool_max_packet_size {
                                        true
                                    } else if !buf.is_empty() && now.duration_since(*last).as_millis() as u64 >= config.coalescing_window_ms {
                                        true
                                    } else {
                                        // Append to buffer
                                        buf.extend_from_slice(&data);
                                        false
                                    }
                                };
                                
                                if should_flush {
                                    // Flush existing buffer first
                                    let flush_data = {
                                        let mut buf = coalescing_buffer.lock().unwrap();
                                        if buf.is_empty() {
                                            None
                                        } else {
                                            let d = buf.clone();
                                            buf.clear();
                                            Some(d)
                                        }
                                    };
                                    
                                    if let Some(d) = flush_data {
                                        let len = d.len();
                                        match transport.send_to(&d, peer_addr).await {
                                            Ok(_) => {
                                                circuit_breaker.record_success();
                                                metrics.record_packet_sent(len);
                                                *last_flush.lock().unwrap() = std::time::Instant::now();
                                            },
                                            Err(e) => {
                                                circuit_breaker.record_failure();
                                                metrics.record_error();
                                                tracing::warn!("Coalesced flush failed: {}", e);
                                            }
                                        }
                                    }
                                    
                                    // Now handle current packet
                                    // If it fits in empty buffer, add it. Else send directly.
                                    if data.len() <= config.pool_max_packet_size {
                                        let mut buf = coalescing_buffer.lock().unwrap();
                                        buf.extend_from_slice(&data);
                                    } else {
                                        // Too big for coalescing, send directly
                                        match transport.send_to(&data, peer_addr).await {
                                            Ok(_) => circuit_breaker.record_success(),
                                            Err(e) => {
                                                circuit_breaker.record_failure();
                                                tracing::warn!("Direct send failed: {}", e);
                                            }
                                        }
                                    }
                                }
                            } else {
                                // No coalescing, send directly
                                match transport.send_to(&data, peer_addr).await {
                                    Ok(_) => circuit_breaker.record_success(),
                                    Err(e) => {
                                        circuit_breaker.record_failure();
                                        tracing::warn!("Sender task failed to send: {}", e);
                                    }
                                }
                            }
                        }
                        None => break, // Queue empty
                    }
                }
            }
        });
        
        self.sender_task = Some(task);
    }

    /// Send data on a specific stream
    pub async fn send_on_stream(&mut self, stream_id: u32, data: &[u8]) -> Result<()> {
        if self.closing.load(Ordering::Relaxed) {
            return Err(anyhow::anyhow!("Connection is closing"));
        }
        
        // Check rate limit
        if !self.rate_limiter.check_and_consume(data.len()) {
            tracing::warn!(
                peer = %self.peer_addr,
                stream_id,
                "Rate limit exceeded"
            );
            return Err(anyhow::anyhow!("Rate limit exceeded"));
        }
        
        // Check congestion window
        if !self.reliability.can_send() {
             tracing::warn!(
                peer = %self.peer_addr,
                stream_id,
                "Congestion window full"
            );
            return Err(anyhow::anyhow!("Congestion window full"));
        }
        
        // Check circuit breaker
        if !self.circuit_breaker.allow_request() {
             tracing::warn!(
                peer = %self.peer_addr,
                stream_id,
                "Circuit breaker open, request rejected"
            );
            return Err(anyhow::anyhow!("Circuit breaker open"));
        }
        
        // Get stream to determine delivery mode and priority
        let (delivery_mode, priority) = {
            let stream = self.session.streams()
                .get_stream(stream_id)
                .ok_or_else(|| anyhow::anyhow!("Stream not found"))?;
            (stream.delivery_mode, stream.priority)
        };
        
        // Update session activity
        self.session.update_activity();
        
        // Get next sequence number
        let seq = self.reliability.next_sequence();
        
        // Track packet if needed (Reliable or PartiallyReliable)
        if delivery_mode.requires_retransmit() {
            self.reliability.track_sent_packet(seq, Bytes::copy_from_slice(data), delivery_mode);
        }
        
        // Check for piggybacked ACK
        let piggyback = if self.reliability.has_pending_acks() {
            let (ack, ranges) = self.reliability.get_ack_info();
            if ranges.is_empty() {
                 self.reliability.on_ack_sent();
                 Some(ack)
            } else {
                None
            }
        } else {
            None
        };

        // Create Header
        let mut header = Header::new(
            stream_id,
            FRAME_TYPE_DATA,
            0, // flags
            seq,
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis() as u64,
            0, // nonce (TODO: Encryption)
            delivery_mode,
            piggyback,
            Some(data.len() as u32),
        );
        
        // Determine if we should compress
        let use_compression = if let Some(start) = self.migration_start {
            if start.elapsed() < Duration::from_secs(5) {
                false
            } else {
                self.migration_start = None;
                self.header_compressor.is_some()
            }
        } else {
            self.header_compressor.is_some()
        };

        if !use_compression {
             header.connection_id = Some(jsp_core::types::connection_id::ConnectionId::from_u64(self.session.session_id));
        }
        
        // Update adaptive compression metrics
        // In a real implementation, we would get RTT from reliability layer and packet loss from metrics
        // For now, we just simulate or use placeholder values if not available
        {
            let rtt = self.metrics.get_avg_rtt();
            // Simple packet loss estimation: (retransmits / total_sent)
            // This is a rough approximation
            let loss = 0.0; // TODO: Get from reliability layer
            
            let mut adaptive = self.adaptive_compression.lock().unwrap();
            adaptive.update_metrics(rtt, loss);
        }
        
        // Serialize Header
        let header_bytes = if use_compression {
            if let Some(compressor) = &mut self.header_compressor {
                // TODO: Pass compression level to compressor if supported
                // For now, adaptive compression just tracks the level
                compressor.compress(&header)
            } else {
                serde_cbor::to_vec(&header)?
            }
        } else {
            serde_cbor::to_vec(&header)?
        };
        let header_len = header_bytes.len() as u16;
        
        // Construct packet: [Header Len (2)] [Header] [Data]
        let packet_len = 2 + header_bytes.len() + data.len();
        
        let mut packet = Vec::with_capacity(packet_len);
        packet.extend_from_slice(&header_len.to_be_bytes());
        packet.extend_from_slice(&header_bytes);
        packet.extend_from_slice(data);
        
        // Determine priority
        let priority = QosPriority::from_value(priority).unwrap_or_default();
        
        // Enqueue
        {
            let mut queue = self.priority_queue.lock().unwrap();
            queue.enqueue(packet, priority);
        }
        
        // Notify sender
        self.sender_notify.notify_one();
        
        tracing::trace!(
            peer = %self.peer_addr,
            stream_id,
            seq,
            ?delivery_mode,
            bytes = data.len(),
            "Data sent on stream"
        );
        
        Ok(())
    }
    
    async fn send_ack(&mut self) -> Result<()> {
        let (ack, sack_ranges) = self.reliability.get_ack_info();
        
        let ack_frame = AckFrame {
            cumulative_ack: ack,
            sack_ranges,
        };
        
        let payload = serde_cbor::to_vec(&ack_frame)?;
        
        // Create Header
        let header = Header::new(
            0, // Stream ID 0 for control
            FRAME_TYPE_ACK,
            0,
            0, // Sequence 0 for ACKs? Or track control sequence?
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis() as u64,
            0,
            jsp_core::types::delivery::DeliveryMode::BestEffort, // ACKs are best effort
            None, // No piggyback on ACK frame
            Some(payload.len() as u32),
        );
        
        let header_bytes = if let Some(compressor) = &mut self.header_compressor {
            compressor.compress(&header)
        } else {
            serde_cbor::to_vec(&header)?
        };
        let header_len = header_bytes.len() as u16;
        
        let mut packet = self.packet_pool.acquire();
        packet.reserve(2 + header_bytes.len() + payload.len());
        packet.extend_from_slice(&header_len.to_be_bytes());
        packet.extend_from_slice(&header_bytes);
        packet.extend_from_slice(&payload);
        
        self.transport.send_to(&packet, self.peer_addr).await?;
        
        self.packet_pool.release(packet);
        
        // Reset batching state
        self.reliability.on_ack_sent();
        
        Ok(())
    }

    /// Receive packets from the connection
    /// Returns a list of (stream_id, data) tuples that are ready (in-order)
    pub async fn recv(&mut self) -> Result<Vec<(u32, Bytes)>> {
        let mut buf = BytesMut::with_capacity(2048);
        buf.resize(2048, 0);
        let (len, src) = self.transport.recv_from(&mut buf).await?;
        buf.truncate(len);
        
        self.metrics.record_packet_received(len);
        
        if src != self.peer_addr {
            // Check if it's a STUN response from one of our servers
            let is_stun = self.stun_server_addrs.contains(&src);
            if !is_stun {
                // Ignore packets from other peers (for now)
                return Ok(Vec::new());
            }
        }
        
        let data = buf.freeze();
        
        // Parse Header Length
        if data.len() < 2 {
            return Ok(Vec::new());
        }
        
        let header_len = u16::from_be_bytes([data[0], data[1]]) as usize;
        if data.len() < 2 + header_len {
            return Ok(Vec::new());
        }
        
        let mut result = Vec::new();
        let mut current_data = data;
        
        while !current_data.is_empty() {
            if current_data.len() < 2 {
                break; // Malformed or empty
            }
            
            let header_len = u16::from_be_bytes([current_data[0], current_data[1]]) as usize;
            if current_data.len() < 2 + header_len {
                break; // Incomplete header
            }
            
            let header_bytes = &current_data[2..2+header_len];
            
            // Try to decompress or deserialize
            let header_result = if let Some(decompressor) = &mut self.header_decompressor {
                // Check first byte to see if it's compressed (flags < 0x80) or CBOR (map/array >= 0x80)
                if !header_bytes.is_empty() && header_bytes[0] < 0x80 {
                    decompressor.decompress(header_bytes).map_err(|e| anyhow::anyhow!("Decompression failed: {}", e))
                } else {
                    serde_cbor::from_slice(header_bytes).map_err(|e| anyhow::anyhow!("Deserialization failed: {}", e))
                }
            } else {
                serde_cbor::from_slice(header_bytes).map_err(|e| anyhow::anyhow!("Deserialization failed: {}", e))
            };

            let header: Header = match header_result {
                Ok(h) => h,
                Err(e) => {
                    tracing::warn!("Failed to parse header: {}", e);
                    break;
                }
            };
            
            // Determine payload length
            let payload_len = if let Some(len) = header.payload_len {
                len as usize
            } else {
                current_data.len() - (2 + header_len)
            };
            
            if current_data.len() < 2 + header_len + payload_len {
                tracing::warn!("Incomplete payload");
                break;
            }
            
            let payload = current_data.slice(2+header_len..2+header_len+payload_len);
            
            // Advance buffer for next packet
            let next_start = 2 + header_len + payload_len;
            let remaining = current_data.slice(next_start..);
            current_data = remaining;
            
            // Process packet
            
            // Process piggybacked ACK if present
            if let Some(ack) = header.piggybacked_ack {
                 self.reliability.on_ack(ack, &[]);
            }
            
            // Update activity
            self.session.update_activity();
            
            // Handle Control Frames
            if header.is_control_frame() {
                if header.msg_type == FRAME_TYPE_ACK {
                    if let Ok(ack_frame) = serde_cbor::from_slice::<AckFrame>(&payload) {
                        self.reliability.on_ack(ack_frame.cumulative_ack, &ack_frame.sack_ranges);
                    }
                } else if header.msg_type == FRAME_TYPE_STUN {
                     if let Ok(msg) = StunMessage::from_bytes(&payload) {
                         if let StunMessageType::BindingResponse = msg.msg_type {
                             for attr in msg.attributes {
                                 if let StunAttribute::MappedAddress(addr) = attr {
                                     self.public_addr = Some(addr);
                                     tracing::info!("Discovered public address: {}", addr);
                                 }
                             }
                         }
                     }
                } else if header.msg_type == FRAME_TYPE_PATH_CHALLENGE {
                    if let Ok(challenge) = serde_cbor::from_slice::<PathChallenge>(&payload) {
                        tracing::debug!("Received PathChallenge, sending response");
                        
                        // Send PathResponse
                        let response = PathResponse { token: challenge.token };
                        let resp_payload = serde_cbor::to_vec(&response)?;
                        
                        let resp_header = Header::new(
                            0,
                            FRAME_TYPE_PATH_RESPONSE,
                            0,
                            0,
                            0,
                            0,
                            jsp_core::types::delivery::DeliveryMode::Reliable,
                            None,
                            Some(resp_payload.len() as u32)
                        );
                        
                        let header_bytes = if let Some(compressor) = &mut self.header_compressor {
                            compressor.compress(&resp_header)
                        } else {
                            serde_cbor::to_vec(&resp_header)?
                        };
                        let header_len = header_bytes.len() as u16;
                        
                        let mut packet = self.packet_pool.acquire();
                        packet.reserve(2 + header_bytes.len() + resp_payload.len());
                        packet.extend_from_slice(&header_len.to_be_bytes());
                        packet.extend_from_slice(&header_bytes);
                        packet.extend_from_slice(&resp_payload);
                        
                        self.transport.send_to(&packet, src).await?;
                        self.packet_pool.release(packet);
                    }
                }
                continue;
            }
            
            // Handle Data Frame
            // Track received packet for reliability
            self.reliability.track_received_packet(header.sequence, header.stream_id, payload);
            
            // Check if ACK should be sent
            if self.reliability.should_send_ack(
                self.config.ack_batch_size, 
                Duration::from_millis(self.config.ack_batch_timeout_ms)
            ) {
                self.send_ack().await?;
            }
            
            // Check for in-order packets
            let packets = self.reliability.pop_received_packets();
            
            for (_seq, stream_id, p_data) in packets {
                result.push((stream_id, p_data));
            }
        }
        
        Ok(result)
    }

    /// Manually flush pending ACKs
    pub async fn flush_acks(&mut self) -> Result<()> {
        if self.reliability.has_pending_acks() {
            self.send_ack().await?;
        }
        Ok(())
    }

    /// Cleanup expired packets from reliability layer
    pub fn cleanup_expired_packets(&mut self) {
        self.reliability.cleanup_expired();
    }

    /// Flush coalesced packets
    pub async fn flush_coalesced(&mut self) -> Result<()> {
        let data = {
            let mut buf = self.coalescing_buffer.lock().unwrap();
            if buf.is_empty() {
                return Ok(());
            }
            let data = buf.clone();
            buf.clear();
            data
        };
        
        self.transport.send_to(&data, self.peer_addr).await?;
        *self.last_coalesce_flush.lock().unwrap() = std::time::Instant::now();
        
        Ok(())
    }

    /// Gracefully close the connection
    pub async fn close(&mut self, reason: CloseReason, message: Option<String>) -> Result<()> {
        self.closing.store(true, Ordering::Relaxed);
        
        tracing::info!(
            peer = %self.peer_addr,
            ?reason,
            "Closing connection"
        );
        
        // Send close frame
        let close_frame = if let Some(msg) = message {
            CloseFrame::with_reason(reason, msg)
        } else {
            CloseFrame { reason_code: reason, message: None }
        };
        
        if let Ok(data) = serde_cbor::to_vec(&close_frame) {
            let _ = self.transport.send_to(&data, self.peer_addr).await;
        }
        
        // Stop heartbeat task
        if let Some(task) = self.heartbeat_task.take() {
            task.abort();
        }
        
        tracing::info!(peer = %self.peer_addr, "Connection closed");
        
        Ok(())
    }

    /// Check if connection is closing
    pub fn is_closing(&self) -> bool {
        self.closing.load(Ordering::Relaxed)
    }

    /// Get connection configuration
    pub fn config(&self) -> &ConnectionConfig {
        &self.config
    }

    /// Get packet pool metrics
    pub fn pool_metrics(&self) -> crate::memory_pool::PoolMetrics {
        self.packet_pool.metrics()
    }

    /// Get packet pool hit rate (percentage)
    pub fn pool_hit_rate(&self) -> f64 {
        self.packet_pool.hit_rate()
    }

    /// Get connection metrics
    pub fn metrics(&self) -> crate::metrics::MetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Process received heartbeat
    pub async fn process_heartbeat(&self, frame: &HeartbeatFrame) {
        if frame.is_response {
            // Received pong
            self.heartbeat.mark_received().await;
            tracing::debug!(
                peer = %self.peer_addr,
                seq = frame.sequence,
                "Heartbeat pong received"
            );
        } else {
            // Received ping, send pong
            let pong = HeartbeatFrame::pong(frame.sequence);
            if let Ok(data) = serde_cbor::to_vec(&pong) {
                let _ = self.transport.send_to(&data, self.peer_addr).await;
                tracing::debug!(
                    peer = %self.peer_addr,
                    seq = frame.sequence,
                    "Heartbeat pong sent"
                );
            }
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        // Abort heartbeat task on drop
        if let Some(task) = self.heartbeat_task.take() {
            task.abort();
        }
        
        // Abort flush task on drop
        if let Some(task) = self.flush_task.take() {
            task.abort();
        }
    }
}
