//! WebRTC Signaling
//! 
//! Handles SDP offer/answer exchange for WebRTC connection establishment.

use serde::{Deserialize, Serialize};

/// SDP type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SdpType {
    Offer,
    Answer,
}

/// Session Description Protocol message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDescription {
    pub sdp_type: SdpType,
    pub sdp: String,
}

impl SessionDescription {
    /// Create an offer
    pub fn offer(sdp: String) -> Self {
        Self {
            sdp_type: SdpType::Offer,
            sdp,
        }
    }
    
    /// Create an answer
    pub fn answer(sdp: String) -> Self {
        Self {
            sdp_type: SdpType::Answer,
            sdp,
        }
    }
}

/// Signaling message for WebRTC
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SignalingMessage {
    /// SDP offer or answer
    #[serde(rename = "sdp")]
    Sdp(SessionDescription),
    
    /// ICE candidate
    #[serde(rename = "ice")]
    IceCandidate {
        candidate: String,
        sdp_mid: Option<String>,
        sdp_mline_index: Option<u16>,
    },
}

/// Signaling channel for exchanging WebRTC messages
pub struct SignalingChannel {
    // In a real implementation, this would use WebSocket or other transport
    // For now, this is a placeholder
}

impl SignalingChannel {
    pub fn new() -> Self {
        Self {}
    }
    
    /// Send a signaling message
    pub async fn send(&self, _message: SignalingMessage) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: Implement actual signaling transport (WebSocket, HTTP, etc.)
        Ok(())
    }
    
    /// Receive a signaling message
    pub async fn recv(&self) -> Result<SignalingMessage, Box<dyn std::error::Error>> {
        // TODO: Implement actual signaling transport
        Err("Not implemented".into())
    }
}

impl Default for SignalingChannel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_description() {
        let sdp = "v=0\no=- 123 456 IN IP4 127.0.0.1\n".to_string();
        let offer = SessionDescription::offer(sdp.clone());
        
        assert_eq!(offer.sdp_type, SdpType::Offer);
        assert_eq!(offer.sdp, sdp);
    }
    
    #[test]
    fn test_signaling_message_serialization() {
        let sdp = SessionDescription::offer("test".to_string());
        let message = SignalingMessage::Sdp(sdp);
        
        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("\"type\":\"sdp\""));
    }
}
