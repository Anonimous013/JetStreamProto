use anyhow::Result;
use jsp_transport::{
    connection::Connection,
    server::Server,
    config::ConnectionConfig,
};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║     JetStreamProto - Connection Mobility Demo           ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    info!("Starting Connection Mobility Demo");

    // 1. Start Server
    let server_addr = "127.0.0.1:8080";
    let mut server = Server::bind(server_addr).await?;
    
    let server_handle = tokio::spawn(async move {
        info!("🖥️  Server listening on {}", server_addr);
        println!("Server: Ready to accept connections\n");
        
        loop {
            match server.accept().await {
                Ok((addr, _session)) => {
                    info!("📡 Server received packet from {}", addr);
                }
                Err(e) => {
                    warn!("Server error: {}", e);
                }
            }
            sleep(Duration::from_millis(10)).await;
        }
    });

    // Give server time to start
    sleep(Duration::from_millis(200)).await;

    // 2. Start Client
    let client_bind_addr = "127.0.0.1:9001";
    let mut config = ConnectionConfig::default();
    config.bind_addr = Some(client_bind_addr.to_string());
    
    info!("📱 Client binding to {}", client_bind_addr);
    println!("Client: Binding to {}", client_bind_addr);
    
    // 3. Connect
    let mut client = Connection::connect_with_config(server_addr, config).await?;
    
    // Perform handshake
    client.handshake().await?;
    info!("✅ Client connected to {}", server_addr);
    println!("Client: Connected to server\n");

    // Open stream before sending data
    client.session.streams_mut().open_stream(1, Default::default())
        .map_err(|e| anyhow::anyhow!(e))?;
    info!("📊 Stream 1 opened");

    // 4. Send Data (Pre-migration)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Phase 1: Pre-Migration Communication");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    
    info!("📤 Sending Message 1 from {}", client_bind_addr);
    println!("Client: Sending 'Hello from 9001' via {}", client_bind_addr);
    client.send_on_stream(1, b"Hello from 9001").await?;
    sleep(Duration::from_millis(500)).await;
    
    info!("📤 Sending Message 2 from {}", client_bind_addr);
    println!("Client: Sending 'Data packet 2' via {}", client_bind_addr);
    client.send_on_stream(1, b"Data packet 2").await?;
    sleep(Duration::from_secs(1)).await;

    // Display connection stats
    println!("\n📊 Connection Statistics (Pre-Migration):");
    println!("   Local Address:  {}", client.local_addr()?);
    println!("   Remote Address: {}", client.peer_addr);
    println!("   Pool Hit Rate:  {:.1}%", client.pool_hit_rate());
    
    let metrics = client.pool_metrics();
    println!("   Pool Metrics:");
    println!("     - Acquisitions: {}", metrics.acquisitions);
    println!("     - Releases:     {}", metrics.releases);
    println!("     - Hits:         {}", metrics.hits);
    println!("     - Misses:       {}", metrics.misses);

    // 5. Migrate
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Phase 2: Connection Migration");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    
    let new_bind_addr = "127.0.0.1:9002";
    info!("🔄 Migrating client from {} to {}", client_bind_addr, new_bind_addr);
    println!("Client: Initiating migration to {}", new_bind_addr);
    println!("Client: Sending PathChallenge to server...");
    
    client.migrate(new_bind_addr).await?;
    
    println!("Client: Migration complete!");
    println!("Client: New local address: {}\n", new_bind_addr);
    sleep(Duration::from_millis(500)).await;

    // 6. Send Data (Post-migration)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Phase 3: Post-Migration Communication");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    
    info!("📤 Sending Message 3 from {}", new_bind_addr);
    println!("Client: Sending 'Hello from 9002' via {}", new_bind_addr);
    client.send_on_stream(1, b"Hello from 9002").await?;
    sleep(Duration::from_millis(500)).await;
    
    info!("📤 Sending Message 4 from {}", new_bind_addr);
    println!("Client: Sending 'Post-migration data' via {}", new_bind_addr);
    client.send_on_stream(1, b"Post-migration data").await?;
    sleep(Duration::from_millis(500)).await;
    
    info!("📤 Sending Message 5 from {}", new_bind_addr);
    println!("Client: Sending 'Connection still alive!' via {}", new_bind_addr);
    client.send_on_stream(1, b"Connection still alive!").await?;
    
    // Wait for path validation to complete
    sleep(Duration::from_secs(1)).await;

    // Display final stats
    println!("\n📊 Connection Statistics (Post-Migration):");
    println!("   Local Address:  {}", client.local_addr()?);
    println!("   Remote Address: {}", client.peer_addr);
    println!("   Pool Hit Rate:  {:.1}%", client.pool_hit_rate());
    
    let final_metrics = client.pool_metrics();
    println!("   Pool Metrics:");
    println!("     - Acquisitions: {}", final_metrics.acquisitions);
    println!("     - Releases:     {}", final_metrics.releases);
    println!("     - Hits:         {}", final_metrics.hits);
    println!("     - Misses:       {}", final_metrics.misses);
    
    // Summary
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Demo Summary");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("✅ Successfully demonstrated connection mobility:");
    println!("   • Client migrated from {} to {}", client_bind_addr, new_bind_addr);
    println!("   • Sent 5 messages total (2 pre-migration, 3 post-migration)");
    println!("   • Connection remained active throughout migration");
    println!("   • Path validation completed successfully");
    println!("\n🎉 Demo completed successfully!\n");
    
    info!("Demo completed");
    
    // Cleanup
    server_handle.abort();
    
    Ok(())
}
