//! MPTCP Subflow

use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::UdpSocket;
use std::sync::Arc;

/// Represents a single subflow (path)
#[derive(Debug)]
pub struct Subflow {
    pub id: u32,
    pub local_addr: SocketAddr,
    pub remote_addr: SocketAddr,
    socket: Arc<UdpSocket>,
    
    // Metrics
    pub rtt: Duration,
    pub cwnd: u32,
    pub bytes_inflight: u32,
}

impl Subflow {
    pub async fn new(id: u32, local: SocketAddr, remote: SocketAddr) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(local).await?;
        socket.connect(remote).await?;
        
        Ok(Self {
            id,
            local_addr: local,
            remote_addr: remote,
            socket: Arc::new(socket),
            rtt: Duration::from_millis(100), // Initial estimate
            cwnd: 10 * 1400, // Initial CWND
            bytes_inflight: 0,
        })
    }

    pub async fn send(&self, data: &[u8]) -> std::io::Result<usize> {
        self.socket.send(data).await
    }

    pub fn update_rtt(&mut self, rtt: Duration) {
        // EWMA
        self.rtt = rtt; 
    }
}
