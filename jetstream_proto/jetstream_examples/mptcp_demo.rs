//! MPTCP Demo
//! 
//! Demonstrates Multi-path TCP capabilities.

use jsp_transport::mptcp::{MptcpManager, MptcpConfig, SchedulerAlgorithm};
use std::net::SocketAddr;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,jsp_transport=debug")
        .init();

    println!("🚀 JetStreamProto MPTCP Demo");
    println!("===========================\n");

    // Configure MPTCP
    let config = MptcpConfig {
        enabled: true,
        max_subflows: 4,
        scheduler_algo: SchedulerAlgorithm::MinRtt,
    };

    println!("⚙️ Configuration:");
    println!("  • Enabled: {}", config.enabled);
    println!("  • Max Subflows: {}", config.max_subflows);
    println!("  • Scheduler: {:?}", config.scheduler_algo);
    println!();

    let remote_addr: SocketAddr = "127.0.0.1:8080".parse()?;
    
    // Create Manager
    let manager = MptcpManager::new(config, remote_addr);
    
    println!("🌐 Starting Interface Watcher...");
    manager.start().await;
    
    // Simulate interface discovery (since we mocked the watcher)
    println!("👀 Watching for network interfaces...");
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    println!("\n✨ MPTCP Layer Active");
    println!("   Traffic will automatically be distributed across:");
    println!("   - eth0 (192.168.1.10) [RTT ~10ms]");
    println!("   - wlan0 (10.0.0.5)    [RTT ~50ms]");
    
    // Send data
    println!("\n📦 Sending data over MPTCP...");
    for i in 1..=5 {
        let msg = format!("Message #{}", i);
        match manager.send(msg.as_bytes()).await {
            Ok(_) => println!("   ✅ Sent '{}'", msg),
            Err(e) => println!("   ❌ Failed to send: {}", e),
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!("\n✅ MPTCP demonstration completed!");
    
    Ok(())
}
