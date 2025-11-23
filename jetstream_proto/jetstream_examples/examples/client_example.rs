use jsp_transport::connection::Connection;
use jsp_transport::config::ConnectionConfig;
use jsp_core::types::control::CloseReason;
use anyhow::Result;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .with_target(false)
        .init();

    tracing::info!("🚀 JetStreamProto Advanced Client Example");

    // Configure connection with custom settings
    let config = ConnectionConfig::builder()
        .session_timeout(Duration::from_secs(60))
        .heartbeat_interval(Duration::from_secs(3))
        .max_streams(50)
        .rate_limit_messages(200)
        .build();

    // Connect to server
    let mut conn = Connection::connect_with_config("127.0.0.1:8080", config).await?;
    tracing::info!("✅ Connected to server");

    // Perform handshake
    conn.handshake().await?;
    tracing::info!("🤝 Handshake completed");

    // Demonstrate stream multiplexing with different delivery modes
    tracing::info!("📡 Opening multiple streams with different delivery modes...");
    
    // Stream 1: Reliable (guaranteed delivery)
    let stream1 = conn.session.open_reliable_stream(1)?;
    tracing::info!("Stream {} opened: Reliable mode", stream1);
    
    // Stream 2: Partially reliable with 100ms TTL
    let stream2 = conn.session.open_partially_reliable_stream(2, 100)?;
    tracing::info!("Stream {} opened: PartiallyReliable mode (TTL=100ms)", stream2);
    
    // Stream 3: Best effort (no retransmit)
    let stream3 = conn.session.open_best_effort_stream(0)?;
    tracing::info!("Stream {} opened: BestEffort mode", stream3);

    // Send data on different streams
    for i in 0..5 {
        let message = format!("Message {} on stream {}", i, stream1);
        conn.send_on_stream(stream1, message.as_bytes()).await?;
        tracing::debug!("Sent: {}", message);
        
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // Demonstrate session ticket generation for 0-RTT
    tracing::info!("🎫 Generating session ticket for 0-RTT resumption...");
    if let Ok(ticket) = conn.session.generate_session_ticket() {
        tracing::info!("Session ticket generated: {:?}", &ticket.ticket_id[..8]);
        // In a real application, save this ticket for next connection
    }

    // Close streams gracefully
    conn.session.close_stream(stream1)?;
    conn.session.close_stream(stream2)?;
    conn.session.close_stream(stream3)?;
    
    tracing::info!("Streams closed");

    // Wait a bit to demonstrate heartbeat
    tracing::info!("⏳ Waiting to demonstrate heartbeat mechanism...");
    tokio::time::sleep(Duration::from_secs(10)).await;

    // Graceful shutdown
    tracing::info!("👋 Closing connection gracefully...");
    conn.close(CloseReason::Normal, None).await?;

    tracing::info!("✨ Client example completed successfully");
    
    Ok(())
}
