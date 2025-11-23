use jsp_transport::signaling::SignalingServer;
use anyhow::Result;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    let addr = "0.0.0.0:8080";
    let server = SignalingServer::new(addr);
    
    info!("Starting signaling server on {}", addr);
    server.run().await?;
    
    Ok(())
}
