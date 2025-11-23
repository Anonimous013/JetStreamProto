use jsp_transport::connection::Connection;
use jsp_transport::config::ConnectionConfigBuilder;
use jsp_transport::stun_server::StunServer;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_stun_discovery() -> anyhow::Result<()> {
    // 1. Start STUN server
    let stun_addr = "127.0.0.1:3478";
    let mut stun_server = StunServer::new(stun_addr).await?;
    stun_server.start();
    
    // Give it a moment to start
    sleep(Duration::from_millis(100)).await;
    
    // 2. Configure client
    let config = ConnectionConfigBuilder::default()
        .stun_servers(vec![stun_addr.to_string()])
        .stun_timeout(Duration::from_secs(1))
        .build();
        
    // 3. Create client connection
    // Bind to port 0 to get a random port
    let mut client = Connection::bind_with_config("127.0.0.1:0", config).await?;
    
    // 4. Discover public address
    let public_addr = client.discover_public_address().await?;
    
    // 5. Verify
    assert!(public_addr.is_some());
    let addr = public_addr.unwrap();
    println!("Discovered address: {}", addr);
    
    assert_eq!(addr.ip().to_string(), "127.0.0.1");
    
    stun_server.stop();
    
    Ok(())
}
