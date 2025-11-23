use tokio::net::UdpSocket;
use std::net::SocketAddr;
use anyhow::Result;
use crate::tcp_transport::TcpTransport;
use crate::quic_transport::QuicTransport;

/// Transport abstraction supporting UDP, TCP, and QUIC
pub enum Transport {
    /// UDP transport (default, low latency)
    Udp(UdpSocket),
    /// TCP transport (fallback when UDP is blocked)
    Tcp(TcpTransport),
    /// QUIC transport (modern, encrypted, multiplexed)
    Quic(QuicTransport),
}

impl Transport {
    /// Create UDP transport
    pub async fn udp(socket: UdpSocket) -> Self {
        Transport::Udp(socket)
    }
    
    /// Create TCP transport
    pub fn tcp(tcp: TcpTransport) -> Self {
        Transport::Tcp(tcp)
    }
    
    /// Create QUIC transport
    pub fn quic(quic: QuicTransport) -> Self {
        Transport::Quic(quic)
    }
    
    /// Send data to address
    pub async fn send_to(&mut self, data: &[u8], addr: SocketAddr) -> Result<usize> {
        match self {
            Transport::Udp(socket) => {
                socket.send_to(data, addr).await.map_err(Into::into)
            }
            Transport::Tcp(tcp) => {
                // TCP is connection-oriented, addr parameter ignored
                tcp.send(data).await
            }
            Transport::Quic(quic) => {
                // QUIC is connection-oriented
                quic.send(data).await
            }
        }
    }
    
    /// Receive data from transport
    pub async fn recv_from(&mut self, buf: &mut [u8]) -> Result<(usize, SocketAddr)> {
        match self {
            Transport::Udp(socket) => {
                socket.recv_from(buf).await.map_err(Into::into)
            }
            Transport::Tcp(tcp) => {
                let len = tcp.recv(buf).await?;
                Ok((len, tcp.peer_addr()))
            }
            Transport::Quic(quic) => {
                let len = quic.recv(buf).await?;
                Ok((len, quic.peer_addr()))
            }
        }
    }
    
    /// Get local address
    pub fn local_addr(&self) -> Result<SocketAddr> {
        match self {
            Transport::Udp(socket) => socket.local_addr().map_err(Into::into),
            Transport::Tcp(tcp) => Ok(tcp.local_addr()),
            Transport::Quic(quic) => Ok(quic.local_addr()),
        }
    }
    
    /// Get peer address (only for TCP/QUIC)
    pub fn peer_addr(&self) -> Option<SocketAddr> {
        match self {
            Transport::Udp(_) => None,
            Transport::Tcp(tcp) => Some(tcp.peer_addr()),
            Transport::Quic(quic) => Some(quic.peer_addr()),
        }
    }
    
    /// Check if this is a TCP transport
    pub fn is_tcp(&self) -> bool {
        matches!(self, Transport::Tcp(_))
    }
    
    /// Check if this is a UDP transport
    pub fn is_udp(&self) -> bool {
        matches!(self, Transport::Udp(_))
    }
    
    /// Check if this is a QUIC transport
    pub fn is_quic(&self) -> bool {
        matches!(self, Transport::Quic(_))
    }
    
    /// Get transport type as string
    pub fn transport_type(&self) -> &'static str {
        match self {
            Transport::Udp(_) => "UDP",
            Transport::Tcp(_) => "TCP",
            Transport::Quic(_) => "QUIC",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tcp_transport::TcpServer;
    
    #[tokio::test]
    async fn test_transport_udp() {
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let local_addr = socket.local_addr().unwrap();
        
        let transport = Transport::udp(socket).await;
        
        assert!(transport.is_udp());
        assert!(!transport.is_tcp());
        assert_eq!(transport.transport_type(), "UDP");
        assert_eq!(transport.local_addr().unwrap(), local_addr);
    }
    
    #[tokio::test]
    async fn test_transport_tcp() {
        let server = TcpServer::bind("127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();
        let server_addr = server.local_addr();
        
        let tcp = TcpTransport::connect(server_addr).await.unwrap();
        let transport = Transport::tcp(tcp);
        
        assert!(transport.is_tcp());
        assert!(!transport.is_udp());
        assert_eq!(transport.transport_type(), "TCP");
        assert!(transport.peer_addr().is_some());
    }
}
