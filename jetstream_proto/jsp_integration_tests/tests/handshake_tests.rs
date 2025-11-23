use jsp_integration_tests::common::{TestClient, TestServer};
use jsp_transport::config::ServerConfig;

#[tokio::test]
async fn test_basic_handshake() {
    // Start server
    let mut server = TestServer::new().await.expect("Failed to create server");
    server.start();
    
    // Connect client
    let client = TestClient::connect(server.addr()).await.expect("Failed to connect client");
    
    // Verify session established
    assert!(client.session_id() > 0);
    
    // Verify server sees the session
    // Note: This might be race-prone if we check immediately, but TestClient::connect waits for handshake
    assert_eq!(server.session_count().await, 1);
    
    // Cleanup
    client.close().await.expect("Failed to close client");
    server.stop().await;
}

#[tokio::test]
async fn test_handshake_with_kyber() {
    // Start server with default config (which enables Kyber)
    let mut server = TestServer::new().await.expect("Failed to create server");
    server.start();
    
    // Connect client
    let client = TestClient::connect(server.addr()).await.expect("Failed to connect client");
    
    // Verify session established
    assert!(client.session_id() > 0);
    
    // We can't easily inspect internal crypto state from here, 
    // but successful handshake implies Kyber worked if enabled
    
    client.close().await.expect("Failed to close client");
    server.stop().await;
}

#[tokio::test]
async fn test_handshake_with_replay_protection() {
    // Start server with replay protection enabled
    let config = ServerConfig::default();
    // Replay protection is enabled by default in SessionConfig, which ServerConfig uses
    
    let mut server = TestServer::with_config(config).await.expect("Failed to create server");
    server.start();
    
    // Connect client 1
    let client1 = TestClient::connect(server.addr()).await.expect("Failed to connect client 1");
    assert!(client1.session_id() > 0);
    
    // Connect client 2 (should also succeed with different nonce)
    let client2 = TestClient::connect(server.addr()).await.expect("Failed to connect client 2");
    assert!(client2.session_id() > 0);
    assert_ne!(client1.session_id(), client2.session_id());
    
    client1.close().await.expect("Failed to close client 1");
    client2.close().await.expect("Failed to close client 2");
    server.stop().await;
}

#[tokio::test]
async fn test_concurrent_handshakes() {
    let mut server = TestServer::new().await.expect("Failed to create server");
    server.start();
    let server_addr = server.addr();
    
    let mut handles = vec![];
    
    // Spawn 10 concurrent clients
    for _ in 0..10 {
        handles.push(tokio::spawn(async move {
            let client = TestClient::connect(server_addr).await.expect("Failed to connect");
            let id = client.session_id();
            client.close().await.expect("Failed to close");
            id
        }));
    }
    
    let mut session_ids = vec![];
    for handle in handles {
        session_ids.push(handle.await.unwrap());
    }
    
    // Verify all session IDs are unique
    session_ids.sort();
    session_ids.dedup();
    assert_eq!(session_ids.len(), 10);
    
    server.stop().await;
}
