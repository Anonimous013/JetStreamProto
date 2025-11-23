use jsp_transport::connection::Connection;
use jsp_transport::config::ConnectionConfig;
use jsp_core::types::control::CloseReason;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_end_to_end_communication() -> Result<(), Box<dyn std::error::Error>> {
    // Start server
    let config = ConnectionConfig {
        heartbeat_interval: Duration::from_secs(5),
        ..Default::default()
    };
    
    let mut server = Connection::listen_with_config("127.0.0.1:0", config.clone()).await?;
    let server_addr = server.local_addr()?;
    
    // Spawn server task
    let server_handle = tokio::spawn(async move {
        // Receive and echo
        loop {
            if let Ok(packets) = server.recv().await {
                for (stream_id, data) in packets {
                    server.send_on_stream(stream_id, &data).await?;
                }
            }
        }
        #[allow(unreachable_code)]
        Ok::<(), Box<dyn std::error::Error>>(())
    });
    
    // Connect client
    let mut client = Connection::connect_with_config(&server_addr.to_string(), config).await?;
    
    // Test multiple message types
    let test_messages = vec![
        (1, b"Hello, World!".to_vec()),
        (2, b"Test message 2".to_vec()),
        (3, vec![0u8; 1024]), // 1KB binary
        (4, vec![0xFF; 10240]), // 10KB binary
    ];
    
    for (stream_id, data) in test_messages {
        // Send
        client.send_on_stream(stream_id, &data).await?;
        
        // Receive echo
        let received = timeout(Duration::from_secs(5), client.recv()).await??;
        
        // Verify
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].0, stream_id);
        assert_eq!(received[0].1.to_vec(), data);
    }
    
    // Close
    client.close(CloseReason::Normal, Some("Test complete".to_string())).await?;
    server_handle.abort();
    
    Ok(())
}

#[tokio::test]
async fn test_multiple_clients() -> Result<(), Box<dyn std::error::Error>> {
    // Start server
    let config = ConnectionConfig {
        heartbeat_interval: Duration::from_secs(10),
        ..Default::default()
    };
    
    let mut server = Connection::listen_with_config("127.0.0.1:0", config.clone()).await?;
    let server_addr = server.local_addr()?;
    
    // Spawn server echo task
    tokio::spawn(async move {
        loop {
            if let Ok(packets) = server.recv().await {
                for (stream_id, data) in packets {
                    let _ = server.send_on_stream(stream_id, &data).await;
                }
            }
        }
    });
    
    // Create multiple clients
    let client_count = 10;
    let mut handles = Vec::new();
    
    for i in 0..client_count {
        let addr = server_addr.to_string();
        let cfg = config.clone();
        
        let handle = tokio::spawn(async move {
            let mut client = Connection::connect_with_config(&addr, cfg).await?;
            
            // Send unique message
            let msg = format!("Client {} message", i);
            client.send_on_stream(1, msg.as_bytes()).await?;
            
            // Receive echo
            let received = timeout(Duration::from_secs(5), client.recv()).await??;
            assert_eq!(received[0].1.to_vec(), msg.as_bytes());
            
            Ok::<(), Box<dyn std::error::Error>>(())
        });
        
        handles.push(handle);
    }
    
    // Wait for all clients
    for handle in handles {
        handle.await??;
    }
    
    Ok(())
}

#[tokio::test]
async fn test_large_message_transfer() -> Result<(), Box<dyn std::error::Error>> {
    // Start server
    let config = ConnectionConfig {
        heartbeat_interval: Duration::from_secs(10),
        ..Default::default()
    };
    
    let mut server = Connection::listen_with_config("127.0.0.1:0", config.clone()).await?;
    let server_addr = server.local_addr()?;
    
    // Spawn server
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
    
    // Test large message (1MB)
    let large_data = vec![0xAB; 1024 * 1024];
    client.send_on_stream(1, &large_data).await?;
    
    // Receive echo
    let received = timeout(Duration::from_secs(30), client.recv()).await??;
    assert_eq!(received[0].1.len(), large_data.len());
    
    Ok(())
}

#[tokio::test]
async fn test_connection_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    // Start server
    let config = ConnectionConfig::default();
    
    let mut server = Connection::listen_with_config("127.0.0.1:0", config.clone()).await?;
    let server_addr = server.local_addr()?;
    
    tokio::spawn(async move {
        let _ = server.recv().await;
    });
    
    // Connect
    let mut client = Connection::connect_with_config(&server_addr.to_string(), config).await?;
    
    // Send message
    client.send_on_stream(1, b"test").await?;
    
    // Graceful close
    client.close(CloseReason::Normal, Some("Test".to_string())).await?;
    
    // Verify connection is closed
    let result = client.send_on_stream(1, b"should fail").await;
    assert!(result.is_err());
    
    Ok(())
}
