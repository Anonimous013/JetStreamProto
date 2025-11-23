use jsp_integration_tests::common::{TestClient, TestServer};
use jsp_core::types::delivery::DeliveryMode;
use std::time::Duration;

#[tokio::test]
async fn test_slow_start_behavior() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // Setup server using TestServer
    let mut server = TestServer::new().await.expect("Failed to create server");
    let server_addr = server.addr();
    server.start();
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Setup client
    let mut client = TestClient::connect(server_addr).await.expect("Failed to connect");
    let stream_id = client.open_stream(1, DeliveryMode::Reliable).expect("Failed to open stream");
    
    // Send burst of packets
    let packet_size = 1000;
    let data = vec![0u8; packet_size];
    
    let mut sent_count = 0;
    for i in 0..20 {
        match client.send(stream_id, &data).await {
            Ok(_) => {
                sent_count += 1;
                tracing::debug!("Sent packet {}", i + 1);
            }
            Err(e) if e.to_string().contains("Congestion window full") => {
                tracing::info!("Hit congestion window limit after {} packets", sent_count);
                break;
            }
            Err(e) => {
                tracing::error!("Unexpected error: {}", e);
                panic!("Unexpected error: {}", e);
            }
        }
    }
    
    println!("Successfully sent {} packets", sent_count);
    tracing::info!("Successfully sent {} packets", sent_count);
    
    // Basic assertion: we should be able to send at least some packets
    assert!(sent_count > 0, "Should be able to send at least one packet");
    
    // Cleanup
    server.stop().await;
}
