// Benchmarks package for JetStreamProto
// This is a library crate that provides benchmarking utilities

pub mod utils {
    use jsp_transport::connection::Connection;
    use jsp_transport::config::ConnectionConfig;
    
    /// Setup a connection pair for benchmarking
    pub async fn setup_connection_pair() -> (Connection, Connection) {
        setup_connection_pair_with_config(ConnectionConfig::default()).await
    }

    /// Setup a connection pair with custom config
    pub async fn setup_connection_pair_with_config(config: ConnectionConfig) -> (Connection, Connection) {
        // Create server
        let mut server = Connection::bind_with_config("127.0.0.1:0", config.clone())
            .await
            .expect("Failed to create server");
        let server_addr = server.local_addr().expect("Failed to get server address");

        // Create client
        let client = Connection::connect_with_config(&server_addr.to_string(), config)
            .await
            .expect("Failed to create client");

        // Perform handshake
        let server_handle = tokio::spawn(async move {
            server.handshake().await.expect("Server handshake failed");
            server
        });

        let mut client = client;
        client.handshake().await.expect("Client handshake failed");
        
        let server = server_handle.await.expect("Server task failed");

        (client, server)
    }
}
