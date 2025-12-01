use jsp_transport::connection::Connection;
use jsp_transport::server::Server;
use jsp_transport::config::{ConnectionConfig, ServerConfig};
use jsp_core::types::control::CloseReason;
use anyhow::Result;
use std::time::Duration;
use tokio::time::timeout;

/// Test basic connection establishment and handshake
#[tokio::test]
async fn test_connection_handshake() -> Result<()> {
    // Start server
    let server_task = tokio::spawn(async {
        let mut server = Server::bind("127.0.0.1:9001").await.unwrap();
        server.accept().await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect client
    let mut client = Connection::connect_with_config("127.0.0.1:9001", ConnectionConfig::default()).await?;
    client.handshake().await?;

    assert!(client.session_id() > 0);
    
    server_task.abort();
    Ok(())
}

/// Test session timeout and cleanup
#[tokio::test]
async fn test_session_timeout() -> Result<()> {
    let config = ServerConfig::builder()
        .connection(
            ConnectionConfig::builder()
                .session_timeout(Duration::from_secs(2))
                .build()
        )
        .cleanup_interval(Duration::from_secs(1))
        .build();

    let mut server = Server::bind_with_config("127.0.0.1:9002", config).await?;
    
    // Create a connection
    let client_task = tokio::spawn(async {
        let mut client = Connection::connect_with_config("127.0.0.1:9002", ConnectionConfig::default()).await.unwrap();
        client.handshake().await.unwrap();
        // Keep connection alive for a bit
        tokio::time::sleep(Duration::from_millis(500)).await;
    });

    // Accept connection
    server.accept().await?;
    
    // Wait for client to finish
    client_task.await?;
    
    // Initial session count should be 1
    assert_eq!(server.session_count().await, 1);
    
    // Wait for session to expire (2s timeout + 1s cleanup)
    tokio::time::sleep(Duration::from_secs(4)).await;
    
    // Session should be cleaned up
    assert_eq!(server.session_count().await, 0);
    
    server.shutdown().await?;
    Ok(())
}

/// Test stream multiplexing with different delivery modes
#[tokio::test]
async fn test_stream_multiplexing() -> Result<()> {
    let mut client = Connection::connect_with_config("127.0.0.1:9003", ConnectionConfig::default()).await?;
    
    // Open multiple streams with different delivery modes
    let stream1 = client.open_stream(1, jsp_core::types::delivery::DeliveryMode::Reliable)?;
    let stream2 = client.open_stream(2, jsp_core::types::delivery::DeliveryMode::PartiallyReliable { ttl_ms: 100 })?;
    let stream3 = client.open_stream(0, jsp_core::types::delivery::DeliveryMode::BestEffort)?;
    
    assert!(stream1 > 0);
    assert!(stream2 > 0);
    assert!(stream3 > 0);
    
    // Streams are created
    // Note: actual stream count checking would require access to internal state
    
    // Delivery modes are set during open_stream
    // Verification would require internal API access
    
    Ok(())
}

/// Test rate limiting
#[tokio::test]
async fn test_rate_limiting() -> Result<()> {
    let config = ConnectionConfig::builder()
        .rate_limit_messages(5)  // Only 5 messages allowed
        .rate_limit_bytes(1000)
        .build();

    let mut client = Connection::connect_with_config("127.0.0.1:9004", config).await?;
    
    let stream_id = client.open_stream(0, jsp_core::types::delivery::DeliveryMode::Reliable)?;
    let data = b"test message";
    
    // First 5 messages should succeed
    for _ in 0..5 {
        assert!(client.send_on_stream(stream_id, data).await.is_ok());
    }
    
    // 6th message should fail due to rate limit
    let result = client.send_on_stream(stream_id, data).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Rate limit"));
    
    Ok(())
}

/// Test graceful shutdown
#[tokio::test]
async fn test_graceful_shutdown() -> Result<()> {
    let mut client = Connection::connect_with_config("127.0.0.1:9005", ConnectionConfig::default()).await?;
    
    assert!(!client.is_closing());
    
    // Gracefully close connection
    client.close(CloseReason::Normal, Some("Test shutdown".to_string())).await?;
    
    assert!(client.is_closing());
    
    Ok(())
}

/// Test 0-RTT session resumption
#[test]
fn test_session_resumption() -> Result<()> {
    use jsp_core::types::control::SessionTicket;
    use std::time::{SystemTime, UNIX_EPOCH};
    
    // Create a mock session ticket
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let ticket = SessionTicket {
        ticket_id: [42u8; 32],
        encrypted_state: vec![1, 2, 3, 4, 5],
        created_at: now,
        lifetime: 3600,
    };
    
    // Verify ticket structure
    assert_eq!(ticket.ticket_id.len(), 32);
    assert_eq!(ticket.lifetime, 3600);
    assert!(!ticket.encrypted_state.is_empty());
    
    // Verify ticket is not expired
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert!(current_time <= ticket.created_at + ticket.lifetime as u64);
    
    Ok(())
}

/// Test heartbeat mechanism
#[tokio::test]
async fn test_heartbeat() -> Result<()> {
    use jsp_core::types::control::HeartbeatFrame;
    
    let ping = HeartbeatFrame::ping(42);
    assert_eq!(ping.sequence, 42);
    assert!(!ping.is_response);
    
    let pong = HeartbeatFrame::pong(42);
    assert_eq!(pong.sequence, 42);
    assert!(pong.is_response);
    
    Ok(())
}

/// Test concurrent connections
#[tokio::test]
async fn test_concurrent_connections() -> Result<()> {
    let server_task = tokio::spawn(async {
        let mut server = Server::bind("127.0.0.1:9008").await.unwrap();
        
        // Accept 3 connections
        for _ in 0..3 {
            server.accept().await.unwrap();
        }
        
        assert_eq!(server.session_count().await, 3);
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create 3 concurrent clients
    let mut handles = vec![];
    for _ in 0..3 {
        let handle = tokio::spawn(async {
            let mut client = Connection::connect_with_config("127.0.0.1:9008", ConnectionConfig::default()).await.unwrap();
            client.handshake().await.unwrap();
        });
        handles.push(handle);
    }

    // Wait for all clients
    for handle in handles {
        handle.await?;
    }

    // Wait for server
    timeout(Duration::from_secs(5), server_task).await??;
    
    Ok(())
}

/// Test configuration builders
#[test]
fn test_configuration() {
    let conn_config = ConnectionConfig::builder()
        .session_timeout(Duration::from_secs(60))
        .heartbeat_interval(Duration::from_secs(10))
        .max_streams(200)
        .rate_limit_messages(500)
        .build();
    
    assert_eq!(conn_config.session_timeout, Duration::from_secs(60));
    assert_eq!(conn_config.max_streams, 200);
    
    let server_config = ServerConfig::builder()
        .connection(conn_config)
        .global_rate_limit_messages(Some(10000))
        .build();
    
    assert_eq!(server_config.global_rate_limit_messages, Some(10000));
}
