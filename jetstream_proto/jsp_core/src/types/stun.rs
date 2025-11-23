use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// STUN message types for NAT traversal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StunMessageType {
    /// Request to discover public address
    BindingRequest,
    /// Response with discovered address
    BindingResponse,
    /// Error response
    BindingError,
}

/// STUN attribute types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StunAttribute {
    /// The public address as observed by the STUN server
    MappedAddress(SocketAddr),
    /// The STUN server's address
    SourceAddress(SocketAddr),
    /// Alternative server address for testing NAT type
    ChangedAddress(SocketAddr),
    /// Error code and message
    ErrorCode(u16, String),
}

/// STUN message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StunMessage {
    /// Message type
    pub msg_type: StunMessageType,
    /// Transaction ID to match requests and responses
    pub transaction_id: [u8; 16],
    /// Message attributes
    pub attributes: Vec<StunAttribute>,
}

impl StunMessage {
    /// Create a new binding request
    pub fn binding_request() -> Self {
        let mut transaction_id = [0u8; 16];
        getrandom::getrandom(&mut transaction_id).expect("Failed to generate random transaction ID");
        
        Self {
            msg_type: StunMessageType::BindingRequest,
            transaction_id,
            attributes: Vec::new(),
        }
    }
    
    /// Create a binding response
    pub fn binding_response(transaction_id: [u8; 16], mapped_addr: SocketAddr) -> Self {
        Self {
            msg_type: StunMessageType::BindingResponse,
            transaction_id,
            attributes: vec![StunAttribute::MappedAddress(mapped_addr)],
        }
    }
    
    /// Create an error response
    pub fn binding_error(transaction_id: [u8; 16], code: u16, message: String) -> Self {
        Self {
            msg_type: StunMessageType::BindingError,
            transaction_id,
            attributes: vec![StunAttribute::ErrorCode(code, message)],
        }
    }
    
    /// Get the mapped address from response
    pub fn get_mapped_address(&self) -> Option<SocketAddr> {
        for attr in &self.attributes {
            if let StunAttribute::MappedAddress(addr) = attr {
                return Some(*addr);
            }
        }
        None
    }
    
    /// Get error code and message
    pub fn get_error(&self) -> Option<(u16, &str)> {
        for attr in &self.attributes {
            if let StunAttribute::ErrorCode(code, msg) = attr {
                return Some((*code, msg.as_str()));
            }
        }
        None
    }
    
    /// Serialize to bytes using CBOR
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_cbor::to_vec(self).expect("Failed to serialize StunMessage")
    }
    
    /// Deserialize from bytes using CBOR
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_cbor::Error> {
        serde_cbor::from_slice(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_binding_request() {
        let req = StunMessage::binding_request();
        assert_eq!(req.msg_type, StunMessageType::BindingRequest);
        assert_eq!(req.transaction_id.len(), 16);
    }
    
    #[test]
    fn test_binding_response() {
        let tid = [1u8; 16];
        let addr = "1.2.3.4:5678".parse().unwrap();
        let resp = StunMessage::binding_response(tid, addr);
        
        assert_eq!(resp.msg_type, StunMessageType::BindingResponse);
        assert_eq!(resp.transaction_id, tid);
        assert_eq!(resp.get_mapped_address(), Some(addr));
    }
    
    #[test]
    fn test_binding_error() {
        let tid = [2u8; 16];
        let err = StunMessage::binding_error(tid, 400, "Bad Request".to_string());
        
        assert_eq!(err.msg_type, StunMessageType::BindingError);
        assert_eq!(err.get_error(), Some((400, "Bad Request")));
    }
}
