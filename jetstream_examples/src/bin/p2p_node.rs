use jsp_transport::connection::Connection;
use jsp_transport::config::ConnectionConfig;
use anyhow::Result;
use tracing::info;
use std::env;
use tokio::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Usage: p2p_node <peer_id> <target_peer_id> [port]");
        return Ok(());
    }
    
    let peer_id = args[1].clone();
    let target_peer_id = args[2].clone();
    let port = args.get(3).map(|s| s.as_str()).unwrap_or("0");
    
    let config = ConnectionConfig::builder()
        .stun_servers(vec!["stun.l.google.com:19302".to_string()])
        .build();
        
    let mut connection = Connection::bind_with_config(&format!("0.0.0.0:{}", port), config).await?;
    
    info!("Bound to {}", connection.local_addr()?);
    
    info!("Connecting to signaling server...");
    connection.connect_p2p(peer_id, target_peer_id, "127.0.0.1:8080".to_string()).await?;
    
    info!("P2P setup initiated. Waiting for candidates...");
    
    // Keep alive
    tokio::time::sleep(Duration::from_secs(60)).await;
    
    Ok(())
}
