use jsp_transport::connection::Connection;
use jsp_transport::config::ConnectionConfig;
use jsp_transport::signaling::SignalingServer;
use tokio::time::Duration;

#[tokio::test]
async fn test_p2p_candidate_exchange() -> anyhow::Result<()> {
    // 1. Start Signaling Server
    let signaling_addr = "127.0.0.1:9090";
    let server = SignalingServer::new(signaling_addr);
    tokio::spawn(async move {
        server.run().await.unwrap();
    });
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // 2. Start Peer A
    let config = ConnectionConfig::default();
    let mut peer_a = Connection::bind_with_config("0.0.0.0:0", config.clone()).await?;
    let peer_a_id = "peer_a".to_string();
    
    // 3. Start Peer B
    let mut peer_b = Connection::bind_with_config("0.0.0.0:0", config).await?;
    let peer_b_id = "peer_b".to_string();
    
    // 4. Connect Peer A to Signaling
    peer_a.connect_p2p(peer_a_id.clone(), peer_b_id.clone(), signaling_addr.to_string()).await?;
    
    // 5. Connect Peer B to Signaling
    peer_b.connect_p2p(peer_b_id.clone(), peer_a_id.clone(), signaling_addr.to_string()).await?;
    
    // 6. Wait for candidates to be exchanged
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    Ok(())
}
