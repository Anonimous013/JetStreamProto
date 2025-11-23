use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::net::SocketAddr;
use anyhow::Result;

/// TCP transport for fallback when UDP is blocked
pub struct TcpTransport {
    stream: TcpStream,
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
}

impl TcpTransport {
    /// Connect to remote peer via TCP
    pub async fn connect(addr: SocketAddr) -> Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        
        // Enable TCP_NODELAY to reduce latency
        stream.set_nodelay(true)?;
        
        let local_addr = stream.local_addr()?;
        let peer_addr = stream.peer_addr()?;
        
        tracing::info!(
            local = %local_addr,
            peer = %peer_addr,
            "TCP connection established"
        );
        
        Ok(Self {
            stream,
            local_addr,
            peer_addr,
        })
    }
    
    /// Send data over TCP with length prefix
    /// 
    /// Format: [4-byte length (big-endian)][data]
    pub async fn send(&mut self, data: &[u8]) -> Result<usize> {
        // Send length prefix (4 bytes, big-endian)
        let len = (data.len() as u32).to_be_bytes();
        self.stream.write_all(&len).await?;
        
        // Send actual data
        self.stream.write_all(data).await?;
        self.stream.flush().await?;
        
        tracing::trace!(
            bytes = data.len(),
            peer = %self.peer_addr,
            "TCP send"
        );
        
        Ok(data.len())
    }
    
    /// Receive data from TCP (read length prefix first)
    /// 
    /// Returns number of bytes read
    pub async fn recv(&mut self, buf: &mut [u8]) -> Result<usize> {
        // Read length prefix (4 bytes)
        let mut len_buf = [0u8; 4];
        self.stream.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        
        // Validate length
        if len == 0 {
            return Err(anyhow::anyhow!("Received zero-length message"));
        }
        
        if len > buf.len() {
            return Err(anyhow::anyhow!(
                "Buffer too small: need {}, have {}",
                len,
                buf.len()
            ));
        }
        
        // Read actual data
        self.stream.read_exact(&mut buf[..len]).await?;
        
        tracing::trace!(
            bytes = len,
            peer = %self.peer_addr,
            "TCP recv"
        );
        
        Ok(len)
    }
    
    /// Get local address
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
    
    /// Get peer address
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }
    
    /// Shutdown the TCP connection
    pub async fn shutdown(&mut self) -> Result<()> {
        self.stream.shutdown().await?;
        tracing::debug!(peer = %self.peer_addr, "TCP connection closed");
        Ok(())
    }
}

/// TCP server for accepting connections
pub struct TcpServer {
    listener: TcpListener,
    local_addr: SocketAddr,
}

impl TcpServer {
    /// Bind TCP server to address
    pub async fn bind(addr: SocketAddr) -> Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        let local_addr = listener.local_addr()?;
        
        tracing::info!(
            addr = %local_addr,
            "TCP server listening"
        );
        
        Ok(Self {
            listener,
            local_addr,
        })
    }
    
    /// Accept incoming TCP connection
    pub async fn accept(&self) -> Result<TcpTransport> {
        let (stream, peer_addr) = self.listener.accept().await?;
        
        // Enable TCP_NODELAY
        stream.set_nodelay(true)?;
        
        let local_addr = stream.local_addr()?;
        
        tracing::info!(
            peer = %peer_addr,
            local = %local_addr,
            "TCP connection accepted"
        );
        
        Ok(TcpTransport {
            stream,
            local_addr,
            peer_addr,
        })
    }
    
    /// Get local address
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;
    
    #[tokio::test]
    async fn test_tcp_send_recv() {
        // Start server
        let server = TcpServer::bind("127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();
        let server_addr = server.local_addr();
        
        // Spawn server task
        let server_handle = tokio::spawn(async move {
            let mut transport = server.accept().await.unwrap();
            
            // Receive message
            let mut buf = vec![0u8; 1024];
            let len = transport.recv(&mut buf).await.unwrap();
            
            // Echo back
            transport.send(&buf[..len]).await.unwrap();
        });
        
        // Connect client
        let mut client = TcpTransport::connect(server_addr).await.unwrap();
        
        // Send message
        let message = b"Hello, TCP!";
        client.send(message).await.unwrap();
        
        // Receive echo
        let mut buf = vec![0u8; 1024];
        let len = client.recv(&mut buf).await.unwrap();
        
        assert_eq!(&buf[..len], message);
        
        // Cleanup
        client.shutdown().await.unwrap();
        server_handle.await.unwrap();
    }
    
    #[tokio::test]
    async fn test_tcp_large_message() {
        // Start server
        let server = TcpServer::bind("127.0.0.1:0".parse().unwrap())
            .await
            .unwrap();
        let server_addr = server.local_addr();
        
        // Spawn server task
        let server_handle = tokio::spawn(async move {
            let mut transport = server.accept().await.unwrap();
            let mut buf = vec![0u8; 100_000];
            let len = transport.recv(&mut buf).await.unwrap();
            transport.send(&buf[..len]).await.unwrap();
        });
        
        // Connect client
        let mut client = TcpTransport::connect(server_addr).await.unwrap();
        
        // Send large message (64KB)
        let message = vec![0xAB; 65536];
        client.send(&message).await.unwrap();
        
        // Receive echo
        let mut buf = vec![0u8; 100_000];
        let len = client.recv(&mut buf).await.unwrap();
        
        assert_eq!(len, message.len());
        assert_eq!(&buf[..len], &message[..]);
        
        // Cleanup
        client.shutdown().await.unwrap();
        server_handle.await.unwrap();
    }
}
