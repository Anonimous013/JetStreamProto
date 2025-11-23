use jsp_transport::server::Server;
use jsp_transport::config::ServerConfig;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Test server helper for integration tests
pub struct TestServer {
    server: Arc<Mutex<Server>>,
    addr: SocketAddr,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl TestServer {
    /// Create a new test server on a random available port
    pub async fn new() -> anyhow::Result<Self> {
        Self::with_config(ServerConfig::default()).await
    }
    
    /// Create a new test server with custom configuration
    pub async fn with_config(config: ServerConfig) -> anyhow::Result<Self> {
        // Bind to localhost with random port (use :0 for OS to assign)
        let server = Server::bind_with_config("127.0.0.1:0", config).await?;
        let actual_addr = server.local_addr()?;
        
        tracing::info!("Test server created on {}", actual_addr);
        
        Ok(Self {
            server: Arc::new(Mutex::new(server)),
            addr: actual_addr,
            handle: None,
        })
    }
    
    /// Start the server in the background
    pub fn start(&mut self) {
        let server: Arc<Mutex<Server>> = Arc::clone(&self.server);
        
        let handle = tokio::spawn(async move {
            loop {
                let mut srv = server.lock().await;
                
                match srv.accept().await {
                    Ok((addr, _session)) => {
                        tracing::debug!("Test server accepted connection from {}", addr);
                        // Session is automatically managed by the server
                    }
                    Err(e) => {
                        tracing::error!("Test server accept error: {}", e);
                        break;
                    }
                }
            }
        });
        
        self.handle = Some(handle);
    }
    
    /// Get the server's listening address
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }
    
    /// Stop the server
    pub async fn stop(mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
            let _ = handle.await;
        }
        
        let mut server = self.server.lock().await;
        let _ = server.shutdown().await;
    }
    
    /// Get the number of active sessions
    pub async fn session_count(&self) -> usize {
        let server = self.server.lock().await;
        server.session_count().await
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_server_creation() {
        let server = TestServer::new().await.unwrap();
        assert!(server.addr().port() > 0);
    }
    
    #[tokio::test]
    async fn test_server_start_stop() {
        let mut server = TestServer::new().await.unwrap();
        server.start();
        
        // Give it a moment to start
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        server.stop().await;
    }
}
