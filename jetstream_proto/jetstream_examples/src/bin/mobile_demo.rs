use jsp_transport::connection::Connection;
use jsp_transport::config::ConnectionConfig;
use jsp_transport::network_status::NetworkType;
use jsp_transport::heartbeat::AppState;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("📱 JetStreamProto Mobile Optimizations Demo");
    println!("{}", "=".repeat(50));
    
    let server_addr = "127.0.0.1:8080";
    
    // Mobile-optimized configuration
    let config = ConnectionConfig {
        heartbeat_interval: Duration::from_secs(10),
        ..Default::default()
    };
    
    println!("\n🔌 Connecting to {}...", server_addr);
    let mut conn = Connection::connect_with_config(server_addr, config).await?;
    println!("✅ Connected!\n");
    
    // Demo 1: Network Type Awareness
    println!("📡 Demo 1: Network Type Awareness");
    println!("{}", "-".repeat(50));
    
    println!("Setting network type: WiFi");
    conn.set_network_type(NetworkType::Wifi).await;
    conn.send_on_stream(1, b"Message on WiFi").await?;
    sleep(Duration::from_secs(2)).await;
    
    println!("Setting network type: Cellular");
    conn.set_network_type(NetworkType::Cellular).await;
    conn.send_on_stream(1, b"Message on Cellular (adaptive compression)").await?;
    sleep(Duration::from_secs(2)).await;
    
    println!("✅ Network type demo complete\n");
    
    // Demo 2: Battery-Aware Heartbeats
    println!("🔋 Demo 2: Battery-Aware Heartbeats");
    println!("{}", "-".repeat(50));
    
    println!("App state: Foreground (heartbeat every 10s)");
    conn.set_app_state(AppState::Foreground).await;
    conn.send_on_stream(1, b"Foreground message").await?;
    sleep(Duration::from_secs(3)).await;
    
    println!("App state: Background (heartbeat every 30s to save battery)");
    conn.set_app_state(AppState::Background).await;
    conn.send_on_stream(1, b"Background message").await?;
    sleep(Duration::from_secs(3)).await;
    
    println!("✅ Battery-aware demo complete\n");
    
    // Demo 3: Adaptive Compression
    println!("🗜️  Demo 3: Adaptive Compression");
    println!("{}", "-".repeat(50));
    
    // Large text payload (compressible)
    let large_text = "Lorem ipsum dolor sit amet, ".repeat(100);
    
    println!("Sending large text ({} bytes)", large_text.len());
    println!("Compression will be adaptive based on network conditions");
    conn.send_on_stream(1, large_text.as_bytes()).await?;
    sleep(Duration::from_secs(2)).await;
    
    println!("✅ Adaptive compression demo complete\n");
    
    // Demo 4: Metrics Monitoring
    println!("📊 Demo 4: Metrics Monitoring");
    println!("{}", "-".repeat(50));
    
    println!("- Adaptive compression: Optimizes based on network");
    println!("- Metrics monitoring: Real-time performance tracking");
    println!("{}", "=".repeat(50));
    
    Ok(())
}
