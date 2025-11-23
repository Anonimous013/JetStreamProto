use jsp_transport::turn_server::TurnServer;
use anyhow::Result;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    let bind_addr = "0.0.0.0:3478";
    let relay_port_range = (10000, 20000);
    
    info!("Starting TURN relay server on {}", bind_addr);
    info!("Relay port range: {}-{}", relay_port_range.0, relay_port_range.1);
    
    let server = TurnServer::new(bind_addr, relay_port_range).await?;
    server.run().await?;
    
    Ok(())
}
