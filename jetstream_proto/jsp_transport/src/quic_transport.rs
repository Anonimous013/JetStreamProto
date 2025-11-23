use quinn::{Endpoint, Connection as QuinnConnection, ServerConfig, ClientConfig};
use std::net::SocketAddr;
use std::sync::Arc;
use anyhow::Result;

/// QUIC transport wrapper
pub struct QuicTransport {
    connection: QuinnConnection,
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
}

impl QuicTransport {
    /// Connect to QUIC server
    pub async fn connect(addr: SocketAddr) -> Result<Self> {
        let client_config = crate::quic_cert::client_config();
        
        let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
        let quic_config = quinn::crypto::rustls::QuicClientConfig::try_from(client_config)?;
        endpoint.set_default_client_config(ClientConfig::new(Arc::new(quic_config)));
        
        // Connect to server
        // Note: "localhost" is used for SNI, but we skip verification in dev
        let connection = endpoint.connect(addr, "localhost")?.await?;
        let local_addr = endpoint.local_addr()?;
        
        tracing::info!(
            local = %local_addr,
            peer = %addr,
            "QUIC connection established"
        );
        
        Ok(Self {
            connection,
            local_addr,
            peer_addr: addr,
        })
    }
    
    /// Send data over QUIC (opens uni-directional stream)
    pub async fn send(&mut self, data: &[u8]) -> Result<usize> {
        let mut send_stream = self.connection.open_uni().await?;
        send_stream.write_all(data).await?;
        send_stream.finish()?;
        
        tracing::trace!(
            bytes = data.len(),
            peer = %self.peer_addr,
            "QUIC send"
        );
        
        Ok(data.len())
    }
    
    /// Receive data from QUIC (accepts uni-directional stream)
    pub async fn recv(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut recv_stream = self.connection.accept_uni().await?;
        let len = recv_stream.read(buf).await?.unwrap_or(0);
        
        tracing::trace!(
            bytes = len,
            peer = %self.peer_addr,
            "QUIC recv"
        );
        
        Ok(len)
    }
    
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
    
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }
    
    /// Close QUIC connection gracefully
    pub fn close(&mut self) {
        self.connection.close(0u32.into(), b"closing");
        tracing::debug!(peer = %self.peer_addr, "QUIC connection closed");
    }
}

/// QUIC server
pub struct QuicServer {
    endpoint: Endpoint,
}

impl QuicServer {
    /// Bind QUIC server
    pub async fn bind(addr: SocketAddr) -> Result<Self> {
        let server_config = crate::quic_cert::server_config()?;
        let config = ServerConfig::with_crypto(Arc::new(
            quinn::crypto::rustls::QuicServerConfig::try_from(server_config)?
        ));
        
        let endpoint = Endpoint::server(config, addr)?;
        
        tracing::info!(
            addr = %endpoint.local_addr()?,
            "QUIC server listening"
        );
        
        Ok(Self { endpoint })
    }
    
    /// Accept incoming QUIC connection
    pub async fn accept(&self) -> Result<QuicTransport> {
        let connecting = self.endpoint.accept().await
            .ok_or_else(|| anyhow::anyhow!("Server closed"))?;
        
        let connection = connecting.await?;
        let peer_addr = connection.remote_address();
        let local_addr = self.endpoint.local_addr()?;
        
        tracing::info!(
            peer = %peer_addr,
            "QUIC connection accepted"
        );
        
        Ok(QuicTransport {
            connection,
            local_addr,
            peer_addr,
        })
    }
    
    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.endpoint.local_addr().map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn init() {
        INIT.call_once(|| {
            let _ = rustls::crypto::ring::default_provider().install_default();
        });
    }

    #[tokio::test]
    async fn test_quic_send_recv() {
        init();
        // Start server
        let server = QuicServer::bind("127.0.0.1:0".parse().unwrap()).await.unwrap();
        let server_addr = server.local_addr().unwrap();
        
        // Start client
        let client_task = tokio::spawn(async move {
            let mut client = QuicTransport::connect(server_addr).await.unwrap();
            client.send(b"hello quic").await.unwrap();
            // Wait a bit to ensure server receives data before closing
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            client.close();
        });
        
        // Accept connection
        let mut server_conn = server.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        let len = server_conn.recv(&mut buf).await.unwrap();
        
        assert_eq!(&buf[..len], b"hello quic");
        
        client_task.await.unwrap();
    }
}
