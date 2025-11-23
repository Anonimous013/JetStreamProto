use std::net::SocketAddr;
use std::collections::HashSet;
use anyhow::Result;
use tracing::{info, warn};
use crate::connection::Connection;
use crate::signaling::{SignalingClient, SignalingMessage};
use crate::turn_client::TurnClient;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CandidateType {
    Host,
    ServerReflexive,
    PeerReflexive,
    Relayed,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Candidate {
    pub addr: SocketAddr,
    pub candidate_type: CandidateType,
    pub priority: u32,
}

pub struct IceAgent {
    local_candidates: HashSet<Candidate>,
    remote_candidates: HashSet<Candidate>,
    pub signaling: Option<SignalingClient>,
    peer_id: String,
    target_peer_id: Option<String>,
    selected_candidate: Option<SocketAddr>,
    turn_client: Option<TurnClient>,
}

impl IceAgent {
    pub fn new(peer_id: String) -> Self {
        Self {
            local_candidates: HashSet::new(),
            remote_candidates: HashSet::new(),
            signaling: None,
            peer_id,
            target_peer_id: None,
            selected_candidate: None,
            turn_client: None,
        }
    }

    pub fn set_turn_server(&mut self, turn_server_addr: SocketAddr) {
        self.turn_client = Some(TurnClient::new(turn_server_addr));
    }

    pub async fn connect_signaling(&mut self, url: &str) -> Result<()> {
        let client = SignalingClient::connect(url, self.peer_id.clone()).await?;
        self.signaling = Some(client);
        Ok(())
    }

    pub fn set_target_peer(&mut self, target: String) {
        self.target_peer_id = Some(target);
    }

    pub fn add_local_candidate(&mut self, addr: SocketAddr, c_type: CandidateType) {
        let priority = match c_type {
            CandidateType::Host => 100,
            CandidateType::ServerReflexive => 80,
            CandidateType::PeerReflexive => 60,
            CandidateType::Relayed => 40,
        };
        
        let candidate = Candidate {
            addr,
            candidate_type: c_type,
            priority,
        };
        
        self.local_candidates.insert(candidate);
    }

    pub async fn gather_candidates(&mut self, connection: &mut Connection) -> Result<()> {
        // 1. Host candidates (local interface)
        // For now, we just use the address we are bound to if possible, or 0.0.0.0
        // In a real implementation, we'd iterate interfaces.
        if let Ok(addr) = connection.local_addr() {
             self.add_local_candidate(addr, CandidateType::Host);
        }

        // 2. Server Reflexive (STUN)
        if let Some(public_addr) = connection.discover_public_address().await? {
            self.add_local_candidate(public_addr, CandidateType::ServerReflexive);
        }
        
        // 3. Relayed (TURN)
        if let Some(turn_client) = &mut self.turn_client {
            match turn_client.allocate(&connection.transport).await {
                Ok(relay_addr) => {
                    info!("TURN relay allocated: {}", relay_addr);
                    self.add_local_candidate(relay_addr, CandidateType::Relayed);
                }
                Err(e) => {
                    warn!("TURN allocation failed: {}", e);
                }
            }
        }
        
        // Send candidates via signaling
        if let (Some(sig), Some(target)) = (&mut self.signaling, &self.target_peer_id) {
            for c in &self.local_candidates {
                let json = serde_json::to_string(c)?;
                let msg = SignalingMessage::Candidate {
                    target: target.clone(),
                    candidate: json,
                };
                sig.send(msg).await?;
            }
        }

        Ok(())
    }

    pub async fn process_signaling_message(&mut self, msg: SignalingMessage) -> Result<()> {
        match msg {
            SignalingMessage::Candidate { candidate, .. } => {
                if let Ok(c) = serde_json::from_str::<Candidate>(&candidate) {
                    info!("Received remote candidate: {:?}", c);
                    self.remote_candidates.insert(c);
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    pub async fn perform_connectivity_checks(&mut self, connection: &mut Connection) -> Result<Option<SocketAddr>> {
        use jsp_core::types::stun::StunMessage;
        use jsp_core::types::header::{Header, FRAME_TYPE_STUN};
        use bytes::BytesMut;
        use tokio::time::{timeout, Duration};
        
        if self.remote_candidates.is_empty() {
            warn!("No remote candidates to check");
            return Ok(None);
        }
        
        // Sort candidates by priority
        let mut candidates: Vec<_> = self.remote_candidates.iter().collect();
        candidates.sort_by_key(|c| std::cmp::Reverse(c.priority));
        
        // Try each candidate
        for candidate in candidates {
            info!("Testing connectivity to {:?}", candidate.addr);
            
            // Send STUN Binding Request
            let req = StunMessage::binding_request();
            let payload = req.to_bytes();
            
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
            
            connection.transport.send_to(&packet, candidate.addr).await?;
            
            // Wait for response (short timeout)
            let check_timeout = Duration::from_millis(500);
            let start = std::time::Instant::now();
            
            while start.elapsed() < check_timeout {
                match timeout(Duration::from_millis(100), connection.recv()).await {
                    Ok(Ok(_)) => {
                        // If we got any response, consider this candidate valid
                        info!("Connectivity check succeeded for {:?}", candidate.addr);
                        self.selected_candidate = Some(candidate.addr);
                        return Ok(Some(candidate.addr));
                    }
                    Ok(Err(e)) => {
                        warn!("Error during connectivity check: {}", e);
                    }
                    Err(_) => continue, // timeout, keep trying
                }
            }
            
            info!("Connectivity check failed for {:?}", candidate.addr);
        }
        
        warn!("All connectivity checks failed");
        Ok(None)
    }
    
    pub fn get_selected_candidate(&self) -> Option<SocketAddr> {
        self.selected_candidate
    }
    
    pub fn remote_candidates_count(&self) -> usize {
        self.remote_candidates.len()
    }
    
    pub fn get_best_remote_candidate(&self) -> Option<SocketAddr> {
        // If we have a selected candidate from connectivity checks, use it
        if let Some(addr) = self.selected_candidate {
            return Some(addr);
        }
        
        // Otherwise, fallback to highest priority
        self.remote_candidates.iter()
            .max_by_key(|c| c.priority)
            .map(|c| c.addr)
    }
}


// Need Serialize/Deserialize for Candidate to send over signaling
impl serde::Serialize for Candidate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Candidate", 3)?;
        state.serialize_field("addr", &self.addr)?;
        // We need to serialize enum as string or number for simplicity, or derive Serialize for CandidateType
        // Let's derive it for CandidateType
        state.serialize_field("type", &self.candidate_type)?;
        state.serialize_field("priority", &self.priority)?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for Candidate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct CandidateHelper {
            addr: SocketAddr,
            #[serde(rename = "type")]
            candidate_type: CandidateType,
            priority: u32,
        }
        
        let helper = CandidateHelper::deserialize(deserializer)?;
        Ok(Candidate {
            addr: helper.addr,
            candidate_type: helper.candidate_type,
            priority: helper.priority,
        })
    }
}

impl serde::Serialize for CandidateType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            CandidateType::Host => serializer.serialize_str("host"),
            CandidateType::ServerReflexive => serializer.serialize_str("srflx"),
            CandidateType::PeerReflexive => serializer.serialize_str("prflx"),
            CandidateType::Relayed => serializer.serialize_str("relay"),
        }
    }
}

impl<'de> serde::Deserialize<'de> for CandidateType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "host" => Ok(CandidateType::Host),
            "srflx" => Ok(CandidateType::ServerReflexive),
            "prflx" => Ok(CandidateType::PeerReflexive),
            "relay" => Ok(CandidateType::Relayed),
            _ => Err(serde::de::Error::custom("unknown candidate type")),
        }
    }
}
