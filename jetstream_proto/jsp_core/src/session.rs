use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use crate::types::connection_id::ConnectionId;
use crate::codec::SerializationFormat;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    New,
    HelloSent,
    Authenticated,
    Established,
}

use crate::crypto::{CryptoContext, CipherSuite};
use crate::types::handshake::{ClientHello, ServerHello};
use crate::types::control::{SessionConfig, SessionTicket};
use crate::stream::StreamManager;
use crate::replay_protection::ReplayProtection;
use anyhow::Result;

const TLS_AES_256_GCM_SHA384: u16 = 0x1302;
const TLS_CHACHA20_POLY1305_SHA256: u16 = 0x1303;

// #[derive(Debug)] // CryptoContext doesn't implement Debug
pub struct Session {
    pub state: SessionState,
    pub session_id: u64,
    pub crypto: CryptoContext,
    client_random: [u8; 32],
    server_random: [u8; 32],
    
    // Timeout and activity tracking
    created_at: Instant,
    last_activity: Instant,
    config: SessionConfig,
    
    // Stream multiplexing
    streams: StreamManager,
    
    // 0-RTT resumption
    pub session_ticket: Option<SessionTicket>,
    
    // Replay protection
    replay_protection: Option<ReplayProtection>,
    
    // Serialization format negotiated during handshake
    serialization_format: SerializationFormat,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    pub fn new() -> Self {
        Self::with_config(SessionConfig::default())
    }

    pub fn with_config(config: SessionConfig) -> Self {
        let now = Instant::now();
        
        // Create replay protection if enabled
        let replay_protection = if config.enable_replay_protection {
            Some(ReplayProtection::new(
                config.replay_window_size,
                Duration::from_secs(config.max_clock_skew_secs),
            ))
        } else {
            None
        };
        
        Self {
            state: SessionState::New,
            session_id: 0,
            crypto: CryptoContext::new(),
            client_random: [0u8; 32],
            server_random: [0u8; 32],
            created_at: now,
            last_activity: now,
            config,
            streams: StreamManager::new(config.max_streams),
            session_ticket: None,
            replay_protection,
            serialization_format: SerializationFormat::default(), // Default to CBOR
        }
    }

    /// Check if session has expired due to inactivity
    pub fn is_expired(&self) -> bool {
        let idle_duration = self.last_activity.elapsed();
        idle_duration > Duration::from_secs(self.config.timeout_secs)
    }

    /// Update last activity timestamp
    pub fn update_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Get time since last activity
    pub fn idle_duration(&self) -> Duration {
        self.last_activity.elapsed()
    }

    /// Get session age
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Open a new stream for multiplexing with specified delivery mode
    pub fn open_stream(&mut self, priority: u8, delivery_mode: crate::types::delivery::DeliveryMode) -> Result<u32> {
        self.update_activity();
        self.streams.open_stream(priority, delivery_mode)
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// Open a reliable stream (guaranteed delivery)
    pub fn open_reliable_stream(&mut self, priority: u8) -> Result<u32> {
        self.open_stream(priority, crate::types::delivery::DeliveryMode::Reliable)
    }

    /// Open a partially reliable stream with TTL
    pub fn open_partially_reliable_stream(&mut self, priority: u8, ttl_ms: u32) -> Result<u32> {
        self.open_stream(priority, crate::types::delivery::DeliveryMode::PartiallyReliable { ttl_ms })
    }

    /// Open a best-effort stream (no retransmit)
    pub fn open_best_effort_stream(&mut self, priority: u8) -> Result<u32> {
        self.open_stream(priority, crate::types::delivery::DeliveryMode::BestEffort)
    }

    /// Close a stream
    pub fn close_stream(&mut self, stream_id: u32) -> Result<()> {
        self.update_activity();
        self.streams.close_stream(stream_id)
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// Get stream manager
    pub fn streams(&self) -> &StreamManager {
        &self.streams
    }

    /// Get mutable stream manager
    pub fn streams_mut(&mut self) -> &mut StreamManager {
        &mut self.streams
    }

    /// Generate session ticket for 0-RTT resumption
    pub fn generate_session_ticket(&self) -> Result<SessionTicket> {
        use rand_core::RngCore;
        
        let mut ticket_id = [0u8; 32];
        rand_core::OsRng.fill_bytes(&mut ticket_id);
        
        // Export session state (simplified - in production, encrypt this properly)
        let state = self.crypto.export_session_state()?;
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        Ok(SessionTicket {
            ticket_id,
            encrypted_state: state,
            created_at: now,
            lifetime: 3600, // 1 hour
        })
    }

    /// Validate and import session ticket for 0-RTT
    pub fn import_session_ticket(&mut self, ticket: &SessionTicket) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        // Check ticket expiration
        if now > ticket.created_at + ticket.lifetime as u64 {
            return Err(anyhow::anyhow!("Session ticket expired"));
        }
        
        // Import session state
        self.crypto.import_session_state(&ticket.encrypted_state)?;
        self.session_ticket = Some(ticket.clone());
        
        Ok(())
    }

    pub fn generate_client_hello(&mut self) -> Result<Vec<u8>, anyhow::Error> {
        use rand_core::RngCore;
        
        self.state = SessionState::HelloSent;
        self.update_activity();
        
        // Generate cryptographically secure random
        let mut rng = rand_core::OsRng;
        rng.fill_bytes(&mut self.client_random);
        
        // Generate nonce for replay protection
        let nonce = rng.next_u64();
        
        // Get current timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        let hello = ClientHello {
            version: 1,
            random: self.client_random,
            session_id: 0,
            cipher_suites: vec![TLS_CHACHA20_POLY1305_SHA256, TLS_AES_256_GCM_SHA384],
            public_key: *self.crypto.x25519_public_key(),
            kyber_public_key: self.crypto.kyber_public_key().to_vec(),
            nonce,
            timestamp,
            connection_id: ConnectionId::generate(),
            // Advertise supported serialization formats (prefer FlatBuffers, fallback to CBOR)
            supported_formats: vec![
                SerializationFormat::FlatBuffers.to_byte(),
                SerializationFormat::Cbor.to_byte(),
            ],
        };
        Ok(serde_cbor::to_vec(&hello)?)
    }

    pub fn process_server_hello(&mut self, data: &[u8]) -> Result<(), anyhow::Error> {
        let hello: ServerHello = serde_cbor::from_slice(data)?;
        
        self.update_activity();
        
        // Decapsulate Kyber ciphertext to get shared secret
        let kyber_shared = self.crypto.decapsulate_kyber(&hello.kyber_ciphertext)?;
        
        // Derive shared secret using HKDF
        self.crypto.derive_shared_secret(
            &hello.public_key,
            Some(&kyber_shared),
            &self.client_random,
            &hello.random
        );
        
        // Set negotiated cipher suite
        let suite = match hello.cipher_suite {
            TLS_CHACHA20_POLY1305_SHA256 => CipherSuite::ChaCha20Poly1305,
            TLS_AES_256_GCM_SHA384 => CipherSuite::Aes256Gcm,
            _ => return Err(anyhow::anyhow!("Unsupported cipher suite: {:x}", hello.cipher_suite)),
        };
        self.crypto.set_cipher_suite(suite);
        
        self.session_id = hello.session_id;
        self.state = SessionState::Established;
        
        // Apply negotiated serialization format
        if let Some(format) = SerializationFormat::from_byte(hello.selected_format) {
            self.serialization_format = format;
        } else {
            // Fallback to CBOR if unknown format
            self.serialization_format = SerializationFormat::Cbor;
        }
        
        Ok(())
    }

    // Server-side methods
    
    pub fn process_client_hello(&mut self, data: &[u8]) -> Result<ClientHello, anyhow::Error> {
        let hello: ClientHello = serde_cbor::from_slice(data)?;
        
        self.update_activity();
        
        // Check replay protection if enabled
        if let Some(ref mut rp) = self.replay_protection {
            rp.check_and_register(hello.nonce, hello.timestamp)
                .map_err(|e| anyhow::anyhow!("Replay attack detected: {}", e))?;
            
            tracing::debug!(
                nonce = hello.nonce,
                timestamp = hello.timestamp,
                window_size = rp.window_size(),
                "Replay protection check passed"
            );
        }
        
        // Store client random for key derivation
        self.client_random = hello.random;
        
        Ok(hello)
    }

    pub fn generate_server_hello(&mut self, session_id: u64, cipher_suite: u16, client_kyber_pk: &[u8], supported_formats: &[u8]) -> Result<(Vec<u8>, Vec<u8>), anyhow::Error> {
        use rand_core::RngCore;
        
        self.update_activity();
        
        // Set negotiated cipher suite
        let suite = match cipher_suite {
            TLS_CHACHA20_POLY1305_SHA256 => CipherSuite::ChaCha20Poly1305,
            TLS_AES_256_GCM_SHA384 => CipherSuite::Aes256Gcm,
            _ => return Err(anyhow::anyhow!("Unsupported cipher suite: {:x}", cipher_suite)),
        };
        self.crypto.set_cipher_suite(suite);
        
        // Generate cryptographically secure random
        let mut rng = rand_core::OsRng;
        rng.fill_bytes(&mut self.server_random);
        
        // Encapsulate Kyber shared secret
        let (kyber_ciphertext, kyber_shared) = self.crypto.encapsulate_kyber(client_kyber_pk)?;
        
        // Select serialization format from client's supported formats
        // Prefer FlatBuffers if client supports it, otherwise fallback to CBOR
        let selected_format = supported_formats.iter()
            .find(|&&f| f == SerializationFormat::FlatBuffers.to_byte())
            .or_else(|| supported_formats.first())
            .copied()
            .unwrap_or(SerializationFormat::Cbor.to_byte());
        
        // Apply selected format to session
        if let Some(format) = SerializationFormat::from_byte(selected_format) {
            self.serialization_format = format;
        }
        
        let hello = ServerHello {
            version: 1,
            random: self.server_random,
            session_id,
            cipher_suite,
            public_key: *self.crypto.x25519_public_key(),
            kyber_ciphertext,
            connection_id: ConnectionId::generate(),
            selected_format,
        };
        
        self.session_id = session_id;
        self.state = SessionState::Established;
        
        Ok((serde_cbor::to_vec(&hello)?, kyber_shared))
    }
    
    /// Get the negotiated serialization format for this session
    pub fn serialization_format(&self) -> SerializationFormat {
        self.serialization_format
    }

    pub fn derive_keys_from_client_hello(&mut self, client_public_key: &[u8; 32], kyber_shared: Option<&[u8]>) {
        self.update_activity();
        
        // Derive shared secret using HKDF
        self.crypto.derive_shared_secret(
            client_public_key,
            kyber_shared,
            &self.client_random,
            &self.server_random
        );
    }
}
