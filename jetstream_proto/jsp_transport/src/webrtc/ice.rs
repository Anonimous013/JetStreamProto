//! ICE (Interactive Connectivity Establishment)

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// ICE candidate type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum IceCandidateType {
    /// Host candidate (local address)
    Host,
    /// Server reflexive (STUN-discovered address)
    Srflx,
    /// Peer reflexive
    Prflx,
    /// Relay (TURN)
    Relay,
}

/// ICE transport policy
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum IceTransportPolicy {
    All,
    Relay,
}

/// ICE candidate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceCandidate {
    /// Candidate type
    pub candidate_type: IceCandidateType,
    
    /// Foundation
    pub foundation: String,
    
    /// Component ID (1 = RTP, 2 = RTCP)
    pub component: u16,
    
    /// Transport protocol
    pub protocol: String,
    
    /// Priority
    pub priority: u32,
    
    /// IP address
    pub address: String,
    
    /// Port
    pub port: u16,
    
    /// Related address (for srflx/relay)
    pub related_address: Option<String>,
    
    /// Related port
    pub related_port: Option<u16>,
}

impl IceCandidate {
    /// Create a new host candidate
    pub fn host(addr: SocketAddr, foundation: String, component: u16) -> Self {
        Self {
            candidate_type: IceCandidateType::Host,
            foundation,
            component,
            protocol: "udp".to_string(),
            priority: Self::calculate_priority(IceCandidateType::Host, component),
            address: addr.ip().to_string(),
            port: addr.port(),
            related_address: None,
            related_port: None,
        }
    }
    
    /// Create a server reflexive candidate
    pub fn srflx(
        public_addr: SocketAddr,
        local_addr: SocketAddr,
        foundation: String,
        component: u16,
    ) -> Self {
        Self {
            candidate_type: IceCandidateType::Srflx,
            foundation,
            component,
            protocol: "udp".to_string(),
            priority: Self::calculate_priority(IceCandidateType::Srflx, component),
            address: public_addr.ip().to_string(),
            port: public_addr.port(),
            related_address: Some(local_addr.ip().to_string()),
            related_port: Some(local_addr.port()),
        }
    }
    
    /// Create a relay candidate
    pub fn relay(
        relay_addr: SocketAddr,
        local_addr: SocketAddr,
        foundation: String,
        component: u16,
    ) -> Self {
        Self {
            candidate_type: IceCandidateType::Relay,
            foundation,
            component,
            protocol: "udp".to_string(),
            priority: Self::calculate_priority(IceCandidateType::Relay, component),
            address: relay_addr.ip().to_string(),
            port: relay_addr.port(),
            related_address: Some(local_addr.ip().to_string()),
            related_port: Some(local_addr.port()),
        }
    }
    
    /// Calculate candidate priority (RFC 5245)
    fn calculate_priority(candidate_type: IceCandidateType, component: u16) -> u32 {
        let type_preference = match candidate_type {
            IceCandidateType::Host => 126,
            IceCandidateType::Prflx => 110,
            IceCandidateType::Srflx => 100,
            IceCandidateType::Relay => 0,
        };
        
        // Priority = (2^24)*(type preference) + (2^8)*(local preference) + (256 - component ID)
        (1 << 24) * type_preference + (1 << 8) * 65535 + (256 - component as u32)
    }
    
    /// Convert to SDP candidate string
    pub fn to_sdp(&self) -> String {
        let mut sdp = format!(
            "candidate:{} {} {} {} {} {} typ {}",
            self.foundation,
            self.component,
            self.protocol,
            self.priority,
            self.address,
            self.port,
            match self.candidate_type {
                IceCandidateType::Host => "host",
                IceCandidateType::Srflx => "srflx",
                IceCandidateType::Prflx => "prflx",
                IceCandidateType::Relay => "relay",
            }
        );
        
        if let (Some(addr), Some(port)) = (&self.related_address, self.related_port) {
            sdp.push_str(&format!(" raddr {} rport {}", addr, port));
        }
        
        sdp
    }
}

/// ICE connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IceConnectionState {
    New,
    Checking,
    Connected,
    Completed,
    Failed,
    Disconnected,
    Closed,
}

/// ICE gatherer for collecting candidates
pub struct IceGatherer {
    candidates: Vec<IceCandidate>,
    state: IceConnectionState,
}

impl IceGatherer {
    pub fn new() -> Self {
        Self {
            candidates: Vec::new(),
            state: IceConnectionState::New,
        }
    }
    
    /// Gather host candidates
    pub async fn gather_host_candidates(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Get local network interfaces
        let interfaces = get_local_addresses()?;
        
        for (idx, addr) in interfaces.iter().enumerate() {
            let candidate = IceCandidate::host(
                *addr,
                format!("host{}", idx),
                1, // Component 1 (RTP/data)
            );
            self.candidates.push(candidate);
        }
        
        Ok(())
    }
    
    /// Get all gathered candidates
    pub fn candidates(&self) -> &[IceCandidate] {
        &self.candidates
    }
    
    /// Get connection state
    pub fn state(&self) -> IceConnectionState {
        self.state
    }
}

impl Default for IceGatherer {
    fn default() -> Self {
        Self::new()
    }
}

/// Get local network addresses
fn get_local_addresses() -> Result<Vec<SocketAddr>, Box<dyn std::error::Error>> {
    use std::net::{IpAddr, Ipv4Addr};
    
    // Simplified: return localhost for now
    // In production, enumerate actual network interfaces
    Ok(vec![
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_host_candidate() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 5000);
        let candidate = IceCandidate::host(addr, "host1".to_string(), 1);
        
        assert_eq!(candidate.candidate_type, IceCandidateType::Host);
        assert_eq!(candidate.address, "192.168.1.100");
        assert_eq!(candidate.port, 5000);
        assert!(candidate.priority > 0);
    }
    
    #[test]
    fn test_candidate_priority() {
        let host_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 5000);
        let host = IceCandidate::host(host_addr, "h1".to_string(), 1);
        
        let public_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)), 5000);
        let srflx = IceCandidate::srflx(public_addr, host_addr, "s1".to_string(), 1);
        
        // Host candidates should have higher priority than srflx
        assert!(host.priority > srflx.priority);
    }
    
    #[test]
    fn test_sdp_format() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), 5000);
        let candidate = IceCandidate::host(addr, "host1".to_string(), 1);
        
        let sdp = candidate.to_sdp();
        assert!(sdp.contains("candidate:"));
        assert!(sdp.contains("typ host"));
        assert!(sdp.contains("192.168.1.1"));
    }
}
