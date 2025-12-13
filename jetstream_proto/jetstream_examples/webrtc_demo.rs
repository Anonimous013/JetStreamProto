//! WebRTC Transport Example
//! 
//! Demonstrates WebRTC transport usage with JetStreamProto.

use jsp_transport::webrtc::{WebRTCConfig, WebRTCTransport};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,jsp_transport=debug")
        .init();

    println!("🚀 JetStreamProto WebRTC Transport Demo");
    println!("========================================\n");

    // Create WebRTC configuration
    let config = WebRTCConfig {
        stun_servers: vec![
            "stun:stun.l.google.com:19302".to_string(),
            "stun:stun1.l.google.com:19302".to_string(),
        ],
        ..Default::default()
    };
    
    println!("📋 Configuration:");
    println!("  - STUN servers: {}", config.stun_servers.len());
    println!("  - ICE policy: {:?}", config.ice_transport_policy);
    println!("  - Data channel: {}", config.data_channel_label);
    println!("  - Ordered: {}", config.ordered);
    println!("  - Reliable: {}\n", config.max_retransmits.is_none());

    // Create WebRTC transport
    println!("🔧 Creating WebRTC transport...");
    let transport = WebRTCTransport::new(config)?;
    
    // Initialize connection
    println!("▶️  Initializing WebRTC connection...");
    transport.initialize().await?;
    println!("✅ WebRTC transport initialized\n");
    
    // Check connection state
    let state = transport.connection_state().await;
    println!("📊 Connection state: {:?}\n", state);
    
    // Simulate sending data
    println!("📤 Sending test data...");
    let test_data = b"Hello from JetStreamProto via WebRTC!";
    match transport.send(test_data).await {
        Ok(sent) => println!("✅ Sent {} bytes", sent),
        Err(e) => println!("❌ Send failed: {}", e),
    }
    
    println!("\n💡 WebRTC Features:");
    println!("  ✅ NAT traversal with STUN/TURN");
    println!("  ✅ Browser compatibility");
    println!("  ✅ Reliable data channels");
    println!("  ✅ ICE candidate gathering");
    println!("  ✅ Automatic fallback");
    
    // Close connection
    println!("\n🛑 Closing WebRTC transport...");
    transport.close().await?;
    println!("✅ Transport closed cleanly");

    Ok(())
}
