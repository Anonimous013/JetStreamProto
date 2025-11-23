use serde::{Deserialize, Serialize};
use super::connection_id::ConnectionId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientHello {
    pub version: u16,
    pub random: [u8; 32],
    pub session_id: u64,
    pub cipher_suites: Vec<u16>,
    pub public_key: [u8; 32],
    pub kyber_public_key: Vec<u8>,
    
    /// Nonce для защиты от replay (для 0-RTT)
    pub nonce: u64,
    
    /// Timestamp для защиты от replay
    pub timestamp: u64,
    
    /// Connection ID proposed by client
    pub connection_id: ConnectionId,
    
    /// Supported serialization formats (0=CBOR, 1=FlatBuffers)
    /// Client lists formats in order of preference
    pub supported_formats: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerHello {
    pub version: u16,
    pub random: [u8; 32],
    pub session_id: u64,
    pub cipher_suite: u16,
    pub public_key: [u8; 32],
    pub kyber_ciphertext: Vec<u8>,
    
    /// Connection ID assigned by server (may differ from client's proposal)
    pub connection_id: ConnectionId,
    
    /// Selected serialization format (0=CBOR, 1=FlatBuffers)
    /// Server chooses from client's supported_formats
    pub selected_format: u8,
}
