use jsp_transport::connection::Connection;
use jsp_transport::config::ConnectionConfig;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_server_crash_recovery() -> Result<(), Box<dyn std::error::Error>> {
    // Start server
    let config = ConnectionConfig::default();
    
    let mut server = Connection::listen_with_config("127.0.0.1:0", config.clone()).await?;
    let server_addr = server.local_addr()?;
    
    let server_handle = tokio::spawn(async move {
        let _ = server.recv().await;
        // Server crashes here (task ends)
    });
    
    // Connect client
    let mut client = Connection::connect_with_config(&server_addr.to_string(), config).await?;
    
    // Send message
    client.send_on_stream(1, b"test").await?;
    
    // Wait for server to "crash"
    let _ = server_handle.await;
    
    // Try to send again - should eventually fail
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Client should detect disconnection
    let result = timeout(Duration::from_secs(10), client.recv()).await;
    
    // Either timeout or error expected
    assert!(result.is_err() || result.unwrap().is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_network_interruption() -> Result<(), Box<dyn std::error::Error>> {
    // Start server
    let config = ConnectionConfig {
        heartbeat_interval: Duration::from_secs(2),
        heartbeat_timeout_count: 2,
        ..Default::default()
    };
    
    let mut server = Connection::listen_with_config("127.0.0.1:0", config.clone()).await?;
    let server_addr = server.local_addr()?;
    
    tokio::spawn(async move {
        loop {
            if let Ok(packets) = server.recv().await {
                for (stream_id, data) in packets {
                    let _ = server.send_on_stream(stream_id, &data).await;
                }
            }
        }
    });
    
    // Connect client
    let mut client = Connection::connect_with_config(&server_addr.to_string(), config).await?;
    
    // Send successful message
    client.send_on_stream(1, b"before interruption").await?;
    let _ = client.recv().await?;
    
    // Simulate network interruption by not sending/receiving for a while
    tokio::time::sleep(Duration::from_secs(10)).await;
    
    // Try to send after interruption - should fail or timeout
    let result = client.send_on_stream(1, b"after interruption").await;
    
    // Connection should be dead
    assert!(result.is_err(), "Expected connection to be dead after timeout");
    
    Ok(())
}

#[tokio::test]
async fn test_packet_loss_simulation() -> Result<(), Box<dyn std::error::Error>> {
    // Start server
    let config = ConnectionConfig {
        heartbeat_interval: Duration::from_secs(10),
        ..Default::default()
    };
    
    let mut server = Connection::listen_with_config("127.0.0.1:0", config.clone()).await?;
    let server_addr = server.local_addr()?;
    
    tokio::spawn(async move {
        loop {
            if let Ok(packets) = server.recv().await {
                for (stream_id, data) in packets {
                    // Simulate 10% packet loss by randomly dropping
                    if rand::random::<f32>() > 0.1 {
                        let _ = server.send_on_stream(stream_id, &data).await;
                    }
                }
            }
        }
    });
    
    // Connect client
    let mut client = Connection::connect_with_config(&server_addr.to_string(), config).await?;
    
    // Send multiple messages
    let message_count = 50;
    let mut received_count = 0;
    
    for i in 0..message_count {
        client.send_on_stream(1, &format!("Message {}", i).as_bytes()).await?;
        
        // Try to receive with timeout
        if let Ok(Ok(_)) = timeout(Duration::from_millis(500), client.recv()).await {
            received_count += 1;
        }
    }
    
    println!("Received {} out of {} messages", received_count, message_count);
    
    // Should receive most messages despite packet loss
    assert!(received_count > message_count / 2, "Too many packets lost");
    
    Ok(())
}

#[tokio::test]
async fn test_connection_timeout() -> Result<(), Box<dyn std::error::Error>> {
    // Try to connect to non-existent server
    let config = ConnectionConfig {
        heartbeat_interval: Duration::from_secs(1),
        ..Default::default()
    };
    
    let result = timeout(
        Duration::from_secs(5),
        Connection::connect_with_config("127.0.0.1:9999", config)
    ).await;
    
    // Should timeout or fail
    assert!(result.is_err() || result.unwrap().is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_malformed_data_handling() -> Result<(), Box<dyn std::error::Error>> {
    // Start server
    let config = ConnectionConfig::default();
    
    let mut server = Connection::listen_with_config("127.0.0.1:0", config.clone()).await?;
    let server_addr = server.local_addr()?;
    
    tokio::spawn(async move {
        loop {
            if let Ok(packets) = server.recv().await {
                for (stream_id, data) in packets {
                    let _ = server.send_on_stream(stream_id, &data).await;
                }
            }
        }
    });
    
    // Connect client
    let mut client = Connection::connect_with_config(&server_addr.to_string(), config).await?;
    
    // Send various data types
    let test_data = vec![
        vec![],                    // Empty
        vec![0xFF; 1],            // Single byte
        vec![0; 10000],           // Large zeros
        vec![0xFF; 10000],        // Large 0xFF
        b"Normal text".to_vec(),  // Text
    ];
    
    for data in test_data {
        // Should handle all data types without crashing
        let result = client.send_on_stream(1, &data).await;
        assert!(result.is_ok(), "Failed to send data");
    }
    
    Ok(())
}

#[tokio::test]
async fn test_rapid_connect_disconnect() -> Result<(), Box<dyn std::error::Error>> {
    // Start server
    let config = ConnectionConfig::default();
    
    let mut server = Connection::listen_with_config("127.0.0.1:0", config.clone()).await?;
    let server_addr = server.local_addr()?;
    
    tokio::spawn(async move {
        loop {
            let _ = server.recv().await;
        }
    });
    
    // Rapidly connect and disconnect
    for _ in 0..20 {
        let mut client = Connection::connect_with_config(&server_addr.to_string(), config.clone()).await?;
        client.send_on_stream(1, b"test").await?;
        // Immediate drop (disconnect)
    }
    
    // Should handle rapid connections without issues
    Ok(())
}
