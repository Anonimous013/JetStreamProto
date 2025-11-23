use jsp_transport::stun_server::StunServer;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Create and start STUN server
    let mut stun_server = StunServer::new("0.0.0.0:3478").await?;
    let addr = stun_server.local_addr()?;
    
    println!("STUN server listening on {}", addr);
    
    stun_server.start();
    
    // Keep server running
    println!("Press Ctrl+C to stop");
    tokio::signal::ctrl_c().await?;
    
    println!("Shutting down...");
    stun_server.stop();
    
    Ok(())
}
