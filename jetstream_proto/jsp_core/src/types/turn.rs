use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// TURN protocol messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TurnMessage {
    /// Request allocation of relay address
    Allocate {
        requested_lifetime: u32, // seconds
    },
    
    /// Successful allocation response
    AllocateSuccess {
        relay_addr: SocketAddr,
        lifetime: u32,
        allocation_id: u64,
    },
    
    /// Allocation failed
    AllocateError {
        code: u16,
        reason: String,
    },
    
    /// Send data through relay to peer
    Send {
        allocation_id: u64,
        peer_addr: SocketAddr,
        data: Vec<u8>,
    },
    
    /// Receive data from relay
    Data {
        peer_addr: SocketAddr,
        data: Vec<u8>,
    },
    
    /// Refresh allocation to keep it alive
    Refresh {
        allocation_id: u64,
        lifetime: u32,
    },
    
    /// Refresh successful
    RefreshSuccess {
        lifetime: u32,
    },
    
    /// Create permission for peer to send data
    CreatePermission {
        allocation_id: u64,
        peer_addr: SocketAddr,
    },
    
    /// Permission created
    PermissionSuccess,
    
    /// Generic error
    Error {
        code: u16,
        reason: String,
    },
}

impl TurnMessage {
    /// Serialize to bytes using CBOR
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_cbor::to_vec(self).expect("Failed to serialize TurnMessage")
    }

    /// Deserialize from bytes using CBOR
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_cbor::Error> {
        serde_cbor::from_slice(bytes)
    }
}

/// TURN error codes
pub mod error_codes {
    pub const ALLOCATION_QUOTA_REACHED: u16 = 486;
    pub const INSUFFICIENT_CAPACITY: u16 = 508;
    pub const ALLOCATION_MISMATCH: u16 = 437;
    pub const PERMISSION_DENIED: u16 = 403;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocate_serialization() {
        let msg = TurnMessage::Allocate {
            requested_lifetime: 600,
        };
        let bytes = msg.to_bytes();
        let decoded = TurnMessage::from_bytes(&bytes).unwrap();
        
        match decoded {
            TurnMessage::Allocate { requested_lifetime } => {
                assert_eq!(requested_lifetime, 600);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_allocate_success_serialization() {
        let addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();
        let msg = TurnMessage::AllocateSuccess {
            relay_addr: addr,
            lifetime: 600,
            allocation_id: 12345,
        };
        
        let bytes = msg.to_bytes();
        let decoded = TurnMessage::from_bytes(&bytes).unwrap();
        
        match decoded {
            TurnMessage::AllocateSuccess { relay_addr, lifetime, allocation_id } => {
                assert_eq!(relay_addr, addr);
                assert_eq!(lifetime, 600);
                assert_eq!(allocation_id, 12345);
            }
            _ => panic!("Wrong message type"),
        }
    }
}
