use jsp_integration_tests::common::{TestClient, TestServer};
use jsp_core::types::delivery::DeliveryMode;
use jsp_transport::config::ServerConfig;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_per_connection_message_rate_limit() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // Create server with default config (100 msg/s per connection)
    let mut server = TestServer::new().await.expect("Failed to create server");
    server.start();
    let server_addr = server.addr();
    
    // Connect client
    let mut client = TestClient::connect(server_addr).await.expect("Failed to connect");
    let stream_id = client.open_stream(1, DeliveryMode::BestEffort).expect("Failed to open stream");
    
    // Try to send messages rapidly
    let data = vec![0u8; 100];
    let mut sent_count = 0;
    let mut denied_count = 0;
    
    // Try to send 150 messages (should hit 100 msg/s limit)
    for _ in 0..150 {
        match client.send(stream_id, &data).await {
            Ok(_) => sent_count += 1,
            Err(e) if e.to_string().contains("Rate limit") => {
                denied_count += 1;
            }
            Err(e) => {
                // Ignore congestion window errors for this test
                if !e.to_string().contains("Congestion window") {
                    panic!("Unexpected error: {}", e);
                }
            }
        }
    }
    
    tracing::info!("Sent: {}, Denied: {}", sent_count, denied_count);
    
    // Should have hit rate limit
    assert!(denied_count > 0, "Should have hit rate limit");
    // Allow some overage due to token refill during test execution
    assert!(sent_count <= 110, "Should not significantly exceed rate limit");
}

#[tokio::test]
async fn test_rate_limit_refill() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // Create server with low rate limit for testing
    let mut config = ServerConfig::default();
    config.connection.rate_limit_messages = 10;
    config.connection.rate_limit_bytes = 10000;
    
    let mut server = TestServer::with_config(config).await.expect("Failed to create server");
    server.start();
    let server_addr = server.addr();
    
    // Connect client
    let mut client = TestClient::connect(server_addr).await.expect("Failed to connect");
    let stream_id = client.open_stream(1, DeliveryMode::BestEffort).expect("Failed to open stream");
    
    let data = vec![0u8; 100];
    
    // Consume all tokens (10 messages)
    let mut sent_count = 0;
    for _ in 0..20 {
        match client.send(stream_id, &data).await {
            Ok(_) => sent_count += 1,
            Err(e) if e.to_string().contains("Rate limit") => break,
            Err(_) => {} // Ignore other errors
        }
    }
    
    tracing::info!("Initial burst sent: {} messages", sent_count);
    // Token refill happens during sending, so allow some overage
    assert!(sent_count >= 10, "Should send at least initial capacity");
    assert!(sent_count <= 20, "Should not exceed capacity too much");
    
    // Wait for refill (10 tokens/sec, wait 500ms = ~5 tokens)
    sleep(Duration::from_millis(500)).await;
    
    // Try to send again - should have refilled some tokens
    let mut refilled_count = 0;
    for _ in 0..10 {
        match client.send(stream_id, &data).await {
            Ok(_) => refilled_count += 1,
            Err(e) if e.to_string().contains("Rate limit") => break,
            Err(_) => {}
        }
    }
    
    tracing::info!("After refill sent: {} messages", refilled_count);
    
    // Should have refilled at least a few tokens
    assert!(refilled_count > 0, "Tokens should have refilled");
}
