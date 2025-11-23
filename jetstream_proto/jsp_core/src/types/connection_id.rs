use serde::{Deserialize, Serialize};
use std::fmt;

/// Connection ID for mobility support
/// Allows connections to survive IP address changes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(u64);

impl ConnectionId {
    /// Generate a new random Connection ID
    pub fn generate() -> Self {
        let mut bytes = [0u8; 8];
        getrandom::getrandom(&mut bytes).expect("Failed to generate random Connection ID");
        ConnectionId(u64::from_be_bytes(bytes))
    }
    
    /// Create from raw u64
    pub fn from_u64(id: u64) -> Self {
        ConnectionId(id)
    }
    
    /// Get raw u64 value
    pub fn as_u64(&self) -> u64 {
        self.0
    }
    
    /// Check if this is a valid (non-zero) Connection ID
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

impl fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

impl Default for ConnectionId {
    fn default() -> Self {
        ConnectionId(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_generate_unique() {
        let mut ids = HashSet::new();
        for _ in 0..1000 {
            let id = ConnectionId::generate();
            assert!(id.is_valid());
            assert!(ids.insert(id), "Duplicate Connection ID generated");
        }
    }

    #[test]
    fn test_serialization() {
        let id = ConnectionId::generate();
        let bytes = serde_cbor::to_vec(&id).unwrap();
        let decoded: ConnectionId = serde_cbor::from_slice(&bytes).unwrap();
        assert_eq!(id, decoded);
    }

    #[test]
    fn test_display() {
        let id = ConnectionId::from_u64(0x123456789ABCDEF0);
        assert_eq!(format!("{}", id), "123456789abcdef0");
    }
}
