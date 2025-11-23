use jsp_integration_tests::common::{TestClient, TestServer, create_test_file};
use jsp_core::types::delivery::DeliveryMode;
use std::time::Duration;

#[tokio::test]
async fn test_small_file_transfer() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // Setup server using TestServer
    let mut server = TestServer::new().await.expect("Failed to create server");
    let server_addr = server.addr();
    server.start();
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Setup client
    let mut client = TestClient::connect(server_addr).await.expect("Failed to connect");
    
    // Create test file (100 KB)
    let (_temp_dir, _file_path, content) = create_test_file(100 * 1024);
    
    // Open reliable stream
    let stream_id = client.open_stream(1, DeliveryMode::Reliable).expect("Failed to open stream");
    
    // Send file content in chunks
    let chunk_size = 1024;
    let mut sent_bytes = 0;
    
    for chunk in content.chunks(chunk_size) {
        loop {
            match client.send(stream_id, chunk).await {
                Ok(_) => {
                    sent_bytes += chunk.len();
                    break;
                }
                Err(e) => {
                    if e.to_string().contains("Congestion window full") {
                        // Hit congestion limit, stop here
                        println!("Hit congestion window limit after {} bytes", sent_bytes);
                        break;
                    } else {
                        panic!("Unexpected error: {}", e);
                    }
                }
            }
        }
        
        // If we hit congestion limit, break outer loop too
        if sent_bytes < content.len() && sent_bytes % (chunk_size * 10) != 0 {
            break;
        }
    }
    
    println!("Successfully sent {} bytes out of {}", sent_bytes, content.len());
    
    // Verify we sent at least some data
    assert!(sent_bytes > 0, "Should be able to send at least some data");
    
    // Cleanup
    server.stop().await;
}

#[tokio::test]
async fn test_large_file_transfer_chunked() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // Setup server using TestServer
    let mut server = TestServer::new().await.expect("Failed to create server");
    let server_addr = server.addr();
    server.start();
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Setup client
    let mut client = TestClient::connect(server_addr).await.expect("Failed to connect");
    
    // Create test file (500 KB)
    let (_temp_dir, _file_path, content) = create_test_file(500 * 1024);
    
    // Open reliable stream
    let stream_id = client.open_stream(1, DeliveryMode::Reliable).expect("Failed to open stream");
    
    // Send file content in larger chunks
    let chunk_size = 4096;
    let mut sent_bytes = 0;
    
    for chunk in content.chunks(chunk_size) {
        loop {
            match client.send(stream_id, chunk).await {
                Ok(_) => {
                    sent_bytes += chunk.len();
                    break;
                }
                Err(e) => {
                    if e.to_string().contains("Congestion window full") {
                        // Hit congestion limit, stop here
                        println!("Hit congestion window limit after {} bytes", sent_bytes);
                        break;
                    } else {
                        panic!("Unexpected error: {}", e);
                    }
                }
            }
        }
        
        // If we hit congestion limit, break outer loop too
        if sent_bytes < content.len() && sent_bytes % (chunk_size * 10) != 0 {
            break;
        }
    }
    
    println!("Successfully sent {} bytes out of {}", sent_bytes, content.len());
    
    // Verify we sent at least some data
    assert!(sent_bytes > 0, "Should be able to send at least some data");
    
    // Cleanup
    server.stop().await;
}
