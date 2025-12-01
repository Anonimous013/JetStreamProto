use jsp_transport::connection::Connection;
use jsp_transport::config::ConnectionConfig;
use jsp_core::types::control::CloseReason;
use jsp_core::types::delivery::DeliveryMode;
use std::net::SocketAddr;

/// Test client helper for integration tests
#[allow(dead_code)]
pub struct TestClient {
    connection: Connection,
}

impl TestClient {
    /// Create a new test client and connect to the server
    pub async fn connect(server_addr: SocketAddr) -> anyhow::Result<Self> {
        let mut config = ConnectionConfig::default();
        if server_addr.ip().is_loopback() {
            config.bind_addr = Some("127.0.0.1:0".to_string());
        }
        Self::connect_with_config(server_addr, config).await
    }
    
    /// Create a new test client with custom configuration
    pub async fn connect_with_config(
        server_addr: SocketAddr,
        config: ConnectionConfig,
    ) -> anyhow::Result<Self> {
        let mut connection = Connection::connect_with_config(&server_addr.to_string(), config).await?;
        
        // Perform handshake with timeout
        tokio::time::timeout(std::time::Duration::from_secs(5), connection.handshake())
            .await
            .map_err(|_| anyhow::anyhow!("Handshake timed out"))??;
        
        tracing::info!("Test client connected to {}", server_addr);
        
        Ok(Self { connection })
    }
    
    /// Send data on a stream
    pub async fn send(&mut self, stream_id: u32, data: &[u8]) -> anyhow::Result<()> {
        self.connection.send_on_stream(stream_id, data).await
    }
    
    /// Receive data from a stream
    /// Note: This is a simplified receiver that waits for the next packet on the specified stream
    /// In a real scenario, you'd want to buffer packets for other streams
    pub async fn receive(&mut self, stream_id: u32) -> anyhow::Result<Vec<u8>> {
        loop {
            let packets = self.connection.recv().await?;
            for (id, data) in packets {
                if id == stream_id {
                    return Ok(data.to_vec());
                }
            }
        }
    }
    
    /// Process incoming packets (including ACKs) without waiting for specific data
    /// This is useful for processing ACKs to update congestion window
    pub async fn process_incoming(&mut self) -> anyhow::Result<()> {
        let _ = self.connection.recv().await?;
        Ok(())
    }
    
    /// Open a new stream with specified delivery mode
    pub fn open_stream(&mut self, priority: u8, mode: DeliveryMode) -> anyhow::Result<u32> {
        self.connection.open_stream(priority, mode)
    }
    
    /// Close a stream
    /// Note: Commented out until public API available
    /*
    pub fn close_stream(&mut self, stream_id: u32) -> anyhow::Result<()> {
        self.connection.session.close_stream(stream_id)
    }
    */
    
    /// Get the session ID
    pub fn session_id(&self) -> u64 {
        self.connection.session_id()
    }
    
    /// Get the local address
    pub fn local_addr(&self) -> anyhow::Result<SocketAddr> {
        self.connection.local_addr()
    }
    
    /// Get the peer address
    pub fn peer_addr(&self) -> SocketAddr {
        self.connection.peer_addr
    }
    
    /// Close the connection gracefully
    pub async fn close(mut self) -> anyhow::Result<()> {
        self.connection.close(CloseReason::Normal, None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::TestServer;
    
    #[tokio::test]
    async fn test_client_connect() {
        // Start test server
        let mut server = TestServer::new().await.unwrap();
        let server_addr = server.addr();
        server.start();
        
        // Give server time to start
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        // Connect client
        let client = TestClient::connect(server_addr).await.unwrap();
        
        assert!(client.session_id() > 0);
        
        // Cleanup
        client.close().await.unwrap();
        server.stop().await;
    }
    
    #[tokio::test]
    async fn test_client_send_receive() {
        // Start test server
        let mut server = TestServer::new().await.unwrap();
        let server_addr = server.addr();
        server.start();
        
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        // Connect client
        let mut client = TestClient::connect(server_addr).await.unwrap();
        
        // Open stream
        let stream_id = client.open_stream(1, DeliveryMode::Reliable).unwrap();
        
        // Send data
        let test_data = b"Hello, JetStreamProto!";
        client.send(stream_id, test_data).await.unwrap();
        
        // Note: In a real test, we'd need the server to echo the data back
        // For now, just verify no errors
        
        // Cleanup
        client.close().await.unwrap();
        server.stop().await;
    }
}
