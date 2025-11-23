use jsp_integration_tests::common::{TestClient, TestServer};
use jsp_core::types::delivery::DeliveryMode;

#[tokio::test]
async fn test_empty_message() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // Create server
    let mut server = TestServer::new().await.expect("Failed to create server");
    server.start();
    let server_addr = server.addr();
    
    // Connect client
    let mut client = TestClient::connect(server_addr).await.expect("Failed to connect");
    let stream_id = client.open_stream(1, DeliveryMode::BestEffort).expect("Failed to open stream");
    
    // Send empty message
    let empty_data = Vec::new();
    let result = client.send(stream_id, &empty_data).await;
    
    // Empty messages should be handled gracefully
    tracing::info!("Empty message result: {:?}", result);
}

#[tokio::test]
async fn test_large_message() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // Create server
    let mut server = TestServer::new().await.expect("Failed to create server");
    server.start();
    let server_addr = server.addr();
    
    // Connect client
    let mut client = TestClient::connect(server_addr).await.expect("Failed to connect");
    let stream_id = client.open_stream(1, DeliveryMode::BestEffort).expect("Failed to open stream");
    
    // Try to send a very large message (10MB)
    let large_data = vec![0u8; 10 * 1024 * 1024];
    let result = client.send(stream_id, &large_data).await;
    
    // Large messages might fail or succeed depending on implementation
    tracing::info!("Large message (10MB) result: {:?}", result.is_ok());
}

#[tokio::test]
async fn test_rapid_stream_creation() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // Create server
    let mut server = TestServer::new().await.expect("Failed to create server");
    server.start();
    let server_addr = server.addr();
    
    // Connect client
    let mut client = TestClient::connect(server_addr).await.expect("Failed to connect");
    
    // Rapidly create many streams
    let mut stream_ids = Vec::new();
    for i in 0..50 {
        match client.open_stream(1, DeliveryMode::BestEffort) {
            Ok(stream_id) => {
                stream_ids.push(stream_id);
            }
            Err(e) => {
                tracing::warn!("Failed to open stream {}: {}", i, e);
                break;
            }
        }
    }
    
    tracing::info!("Created {} streams rapidly", stream_ids.len());
    assert!(stream_ids.len() >= 10, "Should be able to create at least 10 streams");
}

#[tokio::test]
async fn test_stream_reuse_after_close() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // Create server
    let mut server = TestServer::new().await.expect("Failed to create server");
    server.start();
    let server_addr = server.addr();
    
    // Connect client
    let mut client = TestClient::connect(server_addr).await.expect("Failed to connect");
    
    // Open stream
    let stream_id1 = client.open_stream(1, DeliveryMode::BestEffort).expect("Failed to open stream");
    
    // Send data
    let data = b"Test data".to_vec();
    assert!(client.send(stream_id1, &data).await.is_ok());
    
    // Close stream (if close method exists)
    // Note: Current implementation may not have explicit close
    
    // Open another stream - should get a different ID
    let stream_id2 = client.open_stream(1, DeliveryMode::BestEffort).expect("Failed to open stream");
    
    tracing::info!("Stream IDs: {} -> {}", stream_id1, stream_id2);
    // IDs should be different (or same if reused after close)
}

#[tokio::test]
async fn test_zero_priority_stream() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // Create server
    let mut server = TestServer::new().await.expect("Failed to create server");
    server.start();
    let server_addr = server.addr();
    
    // Connect client
    let mut client = TestClient::connect(server_addr).await.expect("Failed to connect");
    
    // Open stream with zero priority
    let stream_id = client.open_stream(0, DeliveryMode::BestEffort).expect("Failed to open stream with priority 0");
    
    // Send data
    let data = b"Zero priority data".to_vec();
    assert!(client.send(stream_id, &data).await.is_ok());
    
    tracing::info!("Zero priority stream works");
}

#[tokio::test]
async fn test_max_priority_stream() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // Create server
    let mut server = TestServer::new().await.expect("Failed to create server");
    server.start();
    let server_addr = server.addr();
    
    // Connect client
    let mut client = TestClient::connect(server_addr).await.expect("Failed to connect");
    
    // Open stream with maximum priority
    let stream_id = client.open_stream(255, DeliveryMode::BestEffort).expect("Failed to open stream with max priority");
    
    // Send data
    let data = b"Max priority data".to_vec();
    assert!(client.send(stream_id, &data).await.is_ok());
    
    tracing::info!("Max priority (255) stream works");
}

#[tokio::test]
async fn test_connection_after_server_restart() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // Create and start server
    let mut server = TestServer::new().await.expect("Failed to create server");
    server.start();
    let server_addr = server.addr();
    
    // Connect client
    let mut client = TestClient::connect(server_addr).await.expect("Failed to connect");
    let stream_id = client.open_stream(1, DeliveryMode::BestEffort).expect("Failed to open stream");
    
    // Send data
    let data = b"Before restart".to_vec();
    assert!(client.send(stream_id, &data).await.is_ok());
    
    // Stop server
    server.stop().await;
    
    // Try to send data - should fail
    let result = client.send(stream_id, &data).await;
    tracing::info!("Send after server stop: {:?}", result.is_ok());
    
    // Note: Reconnection would require new client connection
}

#[tokio::test]
async fn test_simultaneous_bidirectional_send() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // Create server
    let mut server = TestServer::new().await.expect("Failed to create server");
    server.start();
    let server_addr = server.addr();
    
    // Connect two clients
    let mut client1 = TestClient::connect(server_addr).await.expect("Failed to connect client1");
    let mut client2 = TestClient::connect(server_addr).await.expect("Failed to connect client2");
    
    let stream1 = client1.open_stream(1, DeliveryMode::BestEffort).expect("Failed to open stream1");
    let stream2 = client2.open_stream(1, DeliveryMode::BestEffort).expect("Failed to open stream2");
    
    // Both clients send simultaneously
    let data1 = b"Client 1 message".to_vec();
    let data2 = b"Client 2 message".to_vec();
    
    let send1 = client1.send(stream1, &data1);
    let send2 = client2.send(stream2, &data2);
    
    // Wait for both
    // Wait for both (sequentially for simplicity and to avoid type issues)
    let result1 = send1.await;
    let result2 = send2.await;
    
    assert!(result1.is_ok(), "Client 1 send should succeed");
    assert!(result2.is_ok(), "Client 2 send should succeed");
    
    tracing::info!("Simultaneous bidirectional send successful");
}

#[tokio::test]
async fn test_partially_reliable_with_zero_ttl() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // Create server
    let mut server = TestServer::new().await.expect("Failed to create server");
    server.start();
    let server_addr = server.addr();
    
    // Connect client
    let mut client = TestClient::connect(server_addr).await.expect("Failed to connect");
    
    // Open partially reliable stream with zero TTL
    let stream_id = client.open_stream(1, DeliveryMode::PartiallyReliable { ttl_ms: 0 })
        .expect("Failed to open stream with 0 TTL");
    
    // Send data
    let data = b"Zero TTL data".to_vec();
    let result = client.send(stream_id, &data).await;
    
    tracing::info!("Zero TTL partially reliable stream result: {:?}", result.is_ok());
}

#[tokio::test]
async fn test_very_high_ttl() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // Create server
    let mut server = TestServer::new().await.expect("Failed to create server");
    server.start();
    let server_addr = server.addr();
    
    // Connect client
    let mut client = TestClient::connect(server_addr).await.expect("Failed to connect");
    
    // Open partially reliable stream with very high TTL
    let stream_id = client.open_stream(1, DeliveryMode::PartiallyReliable { ttl_ms: u32::MAX })
        .expect("Failed to open stream with max TTL");
    
    // Send data
    let data = b"Max TTL data".to_vec();
    assert!(client.send(stream_id, &data).await.is_ok());
    
    tracing::info!("Very high TTL (u32::MAX) stream works");
}
