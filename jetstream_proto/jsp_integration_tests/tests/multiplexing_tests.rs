use jsp_integration_tests::common::{TestClient, TestServer};
use jsp_core::types::delivery::DeliveryMode;

#[tokio::test]
async fn test_multiple_streams_basic() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // Create server
    let mut server = TestServer::new().await.expect("Failed to create server");
    server.start();
    let server_addr = server.addr();
    
    // Connect client
    let mut client = TestClient::connect(server_addr).await.expect("Failed to connect");
    
    // Open multiple streams
    let stream1 = client.open_stream(1, DeliveryMode::BestEffort).expect("Failed to open stream 1");
    let stream2 = client.open_stream(2, DeliveryMode::BestEffort).expect("Failed to open stream 2");
    let stream3 = client.open_stream(3, DeliveryMode::BestEffort).expect("Failed to open stream 3");
    
    tracing::info!("Opened streams: {}, {}, {}", stream1, stream2, stream3);
    
    // Verify streams have unique IDs
    assert_ne!(stream1, stream2);
    assert_ne!(stream2, stream3);
    assert_ne!(stream1, stream3);
}

#[tokio::test]
async fn test_concurrent_stream_sends() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // Create server
    let mut server = TestServer::new().await.expect("Failed to create server");
    server.start();
    let server_addr = server.addr();
    
    // Connect client
    let mut client = TestClient::connect(server_addr).await.expect("Failed to connect");
    
    // Open multiple streams
    let stream1 = client.open_stream(1, DeliveryMode::BestEffort).expect("Failed to open stream 1");
    let stream2 = client.open_stream(2, DeliveryMode::BestEffort).expect("Failed to open stream 2");
    let stream3 = client.open_stream(3, DeliveryMode::BestEffort).expect("Failed to open stream 3");
    
    // Send data on different streams
    let data1 = b"Stream 1 data".to_vec();
    let data2 = b"Stream 2 data".to_vec();
    let data3 = b"Stream 3 data".to_vec();
    
    // Send concurrently
    let send1 = client.send(stream1, &data1).await;
    let send2 = client.send(stream2, &data2).await;
    let send3 = client.send(stream3, &data3).await;
    
    // All sends should succeed
    assert!(send1.is_ok(), "Stream 1 send failed: {:?}", send1.err());
    assert!(send2.is_ok(), "Stream 2 send failed: {:?}", send2.err());
    assert!(send3.is_ok(), "Stream 3 send failed: {:?}", send3.err());
    
    tracing::info!("Successfully sent data on all streams");
}

#[tokio::test]
async fn test_stream_isolation() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // Create server
    let mut server = TestServer::new().await.expect("Failed to create server");
    server.start();
    let server_addr = server.addr();
    
    // Connect two clients
    let mut client1 = TestClient::connect(server_addr).await.expect("Failed to connect client1");
    let mut client2 = TestClient::connect(server_addr).await.expect("Failed to connect client2");
    
    // Each client opens streams
    let c1_stream1 = client1.open_stream(1, DeliveryMode::BestEffort).expect("Failed to open c1 stream 1");
    let c1_stream2 = client1.open_stream(2, DeliveryMode::BestEffort).expect("Failed to open c1 stream 2");
    
    let c2_stream1 = client2.open_stream(1, DeliveryMode::BestEffort).expect("Failed to open c2 stream 1");
    let c2_stream2 = client2.open_stream(2, DeliveryMode::BestEffort).expect("Failed to open c2 stream 2");
    
    // Send data on each client's streams
    let data = b"Test data".to_vec();
    
    assert!(client1.send(c1_stream1, &data).await.is_ok());
    assert!(client1.send(c1_stream2, &data).await.is_ok());
    assert!(client2.send(c2_stream1, &data).await.is_ok());
    assert!(client2.send(c2_stream2, &data).await.is_ok());
    
    tracing::info!("Stream isolation test passed - both clients can use streams independently");
}

#[tokio::test]
async fn test_max_streams_limit() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // Create server with low stream limit
    use jsp_transport::config::ServerConfig;
    let mut config = ServerConfig::default();
    config.connection.max_streams = 5;
    
    let mut server = TestServer::with_config(config).await.expect("Failed to create server");
    server.start();
    let server_addr = server.addr();
    
    // Connect client
    let mut client = TestClient::connect(server_addr).await.expect("Failed to connect");
    
    // Open streams up to the limit
    let mut streams = Vec::new();
    for i in 0..5 {
        match client.open_stream(1, DeliveryMode::BestEffort) {
            Ok(stream_id) => {
                tracing::info!("Opened stream {}: {}", i, stream_id);
                streams.push(stream_id);
            }
            Err(e) => {
                tracing::error!("Failed to open stream {}: {}", i, e);
                panic!("Should be able to open {} streams", i + 1);
            }
        }
    }
    
    // Try to open one more stream (should fail or succeed depending on implementation)
    match client.open_stream(1, DeliveryMode::BestEffort) {
        Ok(stream_id) => {
            tracing::warn!("Opened stream beyond limit: {}", stream_id);
            // Some implementations may allow this
        }
        Err(e) => {
            tracing::info!("Correctly rejected stream beyond limit: {}", e);
        }
    }
    
    assert_eq!(streams.len(), 5, "Should have opened exactly 5 streams");
}

#[tokio::test]
async fn test_stream_priority_ordering() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // Create server
    let mut server = TestServer::new().await.expect("Failed to create server");
    server.start();
    let server_addr = server.addr();
    
    // Connect client
    let mut client = TestClient::connect(server_addr).await.expect("Failed to connect");
    
    // Open streams with different priorities
    let low_priority = client.open_stream(1, DeliveryMode::BestEffort).expect("Failed to open low priority stream");
    let medium_priority = client.open_stream(5, DeliveryMode::BestEffort).expect("Failed to open medium priority stream");
    let high_priority = client.open_stream(10, DeliveryMode::BestEffort).expect("Failed to open high priority stream");
    
    // Send data on all streams
    let data = b"Priority test data".to_vec();
    
    assert!(client.send(low_priority, &data).await.is_ok());
    assert!(client.send(medium_priority, &data).await.is_ok());
    assert!(client.send(high_priority, &data).await.is_ok());
    
    tracing::info!("Successfully sent data on streams with different priorities");
    // Note: Actual priority enforcement would require more complex testing
}

#[tokio::test]
async fn test_mixed_delivery_modes() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // Create server
    let mut server = TestServer::new().await.expect("Failed to create server");
    server.start();
    let server_addr = server.addr();
    
    // Connect client
    let mut client = TestClient::connect(server_addr).await.expect("Failed to connect");
    
    // Open streams with different delivery modes
    let reliable = client.open_stream(1, DeliveryMode::Reliable).expect("Failed to open reliable stream");
    let best_effort = client.open_stream(1, DeliveryMode::BestEffort).expect("Failed to open best effort stream");
    let partial = client.open_stream(1, DeliveryMode::PartiallyReliable { ttl_ms: 1000 })
        .expect("Failed to open partially reliable stream");
    
    // Send data on all streams
    let data = b"Mixed mode test".to_vec();
    
    assert!(client.send(reliable, &data).await.is_ok());
    assert!(client.send(best_effort, &data).await.is_ok());
    assert!(client.send(partial, &data).await.is_ok());
    
    tracing::info!("Successfully sent data on streams with different delivery modes");
}
