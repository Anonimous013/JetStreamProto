use serde::{Deserialize, Serialize};

/// Path validation challenge message
/// Sent to verify a new path before accepting it
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathChallenge {
    /// Random token for this challenge
    pub token: [u8; 8],
}

/// Path validation response message
/// Echoes the challenge token to prove path ownership
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathResponse {
    /// Token from the challenge
    pub token: [u8; 8],
}

impl PathChallenge {
    /// Create a new challenge with random token
    pub fn new() -> Self {
        let mut token = [0u8; 8];
        getrandom::getrandom(&mut token).expect("Failed to generate random token");
        Self { token }
    }
    
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_cbor::to_vec(self).expect("Failed to serialize PathChallenge")
    }
    
    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_cbor::Error> {
        serde_cbor::from_slice(bytes)
    }
}

impl PathResponse {
    /// Create response for a challenge
    pub fn for_challenge(challenge: &PathChallenge) -> Self {
        Self {
            token: challenge.token,
        }
    }
    
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_cbor::to_vec(self).expect("Failed to serialize PathResponse")
    }
    
    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_cbor::Error> {
        serde_cbor::from_slice(bytes)
    }
    
    /// Verify this response matches a challenge
    pub fn matches(&self, challenge: &PathChallenge) -> bool {
        self.token == challenge.token
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_challenge_response_flow() {
        let challenge = PathChallenge::new();
        let response = PathResponse::for_challenge(&challenge);
        assert!(response.matches(&challenge));
    }

    #[test]
    fn test_different_challenges() {
        let challenge1 = PathChallenge::new();
        let challenge2 = PathChallenge::new();
        let response1 = PathResponse::for_challenge(&challenge1);
        
        assert!(response1.matches(&challenge1));
        assert!(!response1.matches(&challenge2));
    }

    #[test]
    fn test_serialization() {
        let challenge = PathChallenge::new();
        let bytes = challenge.to_bytes();
        let decoded = PathChallenge::from_bytes(&bytes).unwrap();
        assert_eq!(challenge.token, decoded.token);
    }
}
