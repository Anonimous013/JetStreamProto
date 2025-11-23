use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use anyhow::Result;
use tracing::{info, error, warn};

/// Messages exchanged over the signaling channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalingMessage {
    /// Register with the server using a unique Peer ID
    Register { peer_id: String },
    /// Successfully registered
    Registered,
    /// Send an offer to a target peer
    Offer { target: String, sdp: String },
    /// Send an answer to a target peer
    Answer { target: String, sdp: String },
    /// Send an ICE candidate to a target peer
    Candidate { target: String, candidate: String },
    /// Error message
    Error { message: String },
}

type Tx = mpsc::UnboundedSender<SignalingMessage>;

/// A simple TCP-based signaling server
pub struct SignalingServer {
    addr: String,
    peers: Arc<Mutex<HashMap<String, Tx>>>,
}

impl SignalingServer {
    pub fn new(addr: impl Into<String>) -> Self {
        Self {
            addr: addr.into(),
            peers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn run(&self) -> Result<()> {
        let listener = TcpListener::bind(&self.addr).await?;
        info!("Signaling server listening on {}", self.addr);

        loop {
            let (stream, addr) = listener.accept().await?;
            let peers = self.peers.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, addr, peers).await {
                    error!("Connection error from {}: {}", addr, e);
                }
            });
        }
    }
}

async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    peers: Arc<Mutex<HashMap<String, Tx>>>,
) -> Result<()> {
    info!("New connection from {}", addr);

    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut peer_id: Option<String> = None;

    // Split stream into owned halves
    let (mut reader, mut writer) = stream.into_split();
    
    // Task to write messages to the socket
    let write_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let json = serde_json::to_string(&msg).unwrap();
            let len = (json.len() as u32).to_be_bytes();
            if writer.write_all(&len).await.is_err() {
                break;
            }
            if writer.write_all(json.as_bytes()).await.is_err() {
                break;
            }
        }
    });

    let mut buf = [0u8; 4];
    loop {
        // Read length
        if reader.read_exact(&mut buf).await.is_err() {
            break;
        }
        let len = u32::from_be_bytes(buf) as usize;
        
        // Read payload
        let mut payload = vec![0u8; len];
        if reader.read_exact(&mut payload).await.is_err() {
            break;
        }

        let msg: SignalingMessage = match serde_json::from_slice(&payload) {
            Ok(m) => m,
            Err(e) => {
                warn!("Invalid message from {}: {}", addr, e);
                continue;
            }
        };

        match msg {
            SignalingMessage::Register { peer_id: pid } => {
                info!("Peer registered: {}", pid);
                let mut map = peers.lock().await;
                map.insert(pid.clone(), tx.clone());
                peer_id = Some(pid);
                let _ = tx.send(SignalingMessage::Registered);
            }
            SignalingMessage::Offer { target, sdp } => {
                relay_message(&peers, &target, SignalingMessage::Offer { target: peer_id.clone().unwrap(), sdp }).await;
            }
            SignalingMessage::Answer { target, sdp } => {
                relay_message(&peers, &target, SignalingMessage::Answer { target: peer_id.clone().unwrap(), sdp }).await;
            }
            SignalingMessage::Candidate { target, candidate } => {
                relay_message(&peers, &target, SignalingMessage::Candidate { target: peer_id.clone().unwrap(), candidate }).await;
            }
            _ => {}
        }
    }

    if let Some(pid) = peer_id {
        info!("Peer disconnected: {}", pid);
        peers.lock().await.remove(&pid);
    }
    write_task.abort();

    Ok(())
}

async fn relay_message(peers: &Arc<Mutex<HashMap<String, Tx>>>, target: &str, msg: SignalingMessage) {
    let map = peers.lock().await;
    if let Some(tx) = map.get(target) {
        let _ = tx.send(msg);
    } else {
        warn!("Target peer not found: {}", target);
    }
}

/// Client for the signaling server
pub struct SignalingClient {
    stream: TcpStream,
    #[allow(dead_code)]
    peer_id: String,
}

impl SignalingClient {
    pub async fn connect(addr: &str, peer_id: String) -> Result<Self> {
        let mut stream = TcpStream::connect(addr).await?;
        
        // Send Register message
        let msg = SignalingMessage::Register { peer_id: peer_id.clone() };
        Self::send_msg(&mut stream, &msg).await?;

        Ok(Self { stream, peer_id })
    }

    async fn send_msg(stream: &mut TcpStream, msg: &SignalingMessage) -> Result<()> {
        let json = serde_json::to_string(msg)?;
        let len = (json.len() as u32).to_be_bytes();
        stream.write_all(&len).await?;
        stream.write_all(json.as_bytes()).await?;
        Ok(())
    }

    pub async fn send(&mut self, msg: SignalingMessage) -> Result<()> {
        Self::send_msg(&mut self.stream, &msg).await
    }

    pub async fn recv(&mut self) -> Result<SignalingMessage> {
        let mut buf = [0u8; 4];
        self.stream.read_exact(&mut buf).await?;
        let len = u32::from_be_bytes(buf) as usize;
        
        let mut payload = vec![0u8; len];
        self.stream.read_exact(&mut payload).await?;
        
        let msg = serde_json::from_slice(&payload)?;
        Ok(msg)
    }
}
