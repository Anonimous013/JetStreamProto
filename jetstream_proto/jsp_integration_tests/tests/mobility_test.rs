use anyhow::Result;
use jsp_transport::{
    connection::Connection,
    server::Server,
    config::ConnectionConfig,
};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_connection_mobility() -> Result<()> {
    // Initialize tracing (ignore if already initialized)
    let _ = tracing_subscriber::fmt::try_init();
    
    // 1. Start Server
    let server_addr = "127.0.0.1:8085";
    let mut server = Server::bind(server_addr).await?;
    
    let server_handle = tokio::spawn(async move {
        // Server loop - accept connections and process packets
        loop {
            match server.accept().await {
                Ok((addr, _session)) => {
                    println!("Server received packet from {}", addr);
                }
                Err(_e) => {
                    // Ignore errors for this test
                }
            }
        }
    });

    // Give server time to start
    sleep(Duration::from_millis(100)).await;

    // 2. Start Client
    let client_bind_addr = "127.0.0.1:9010";
    let mut config = ConnectionConfig::default();
    config.bind_addr = Some(client_bind_addr.to_string());
    
    // 3. Connect
    let mut client = Connection::connect_with_config(server_addr, config).await?;
    client.handshake().await?;
    println!("Client connected from {}", client_bind_addr);
    
    // Open stream before sending data
    client.session.streams_mut().open_stream(1, Default::default())
        .map_err(|e| anyhow::anyhow!(e))?;
    
    // 4. Send Data (Pre-migration)
    client.send_on_stream(1, b"Data before migration").await?;
    sleep(Duration::from_millis(200)).await;
    
    // 5. Migrate to new address
    let new_bind_addr = "127.0.0.1:9011";
    println!("Migrating client to {}", new_bind_addr);
    client.migrate(new_bind_addr).await?;
    
    // 6. Send Data (Post-migration)
    // This should trigger path validation on the server side
    client.send_on_stream(1, b"Data after migration").await?;
    
    // 7. Wait for path validation to complete
    sleep(Duration::from_secs(1)).await;
    
    // 8. Send more data to verify connection still works
    client.send_on_stream(1, b"Final data").await?;
    sleep(Duration::from_millis(200)).await;
    
    println!("Mobility test completed successfully");
    
    // Cleanup
    server_handle.abort();
    
    Ok(())
}
