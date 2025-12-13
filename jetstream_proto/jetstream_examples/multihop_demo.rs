//! Multi-Hop Tunnel Demo
//! 
//! Demonstrates basic usage of the multi-hop tunnel manager.

use jsp_transport::multihop::{MultiHopConfig, HopConfig};
use jsp_transport::multihop::config::{WireGuardConfig, ShadowsocksConfig, XRayConfig};
use jsp_transport::multihop::MultiHopEngine;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,jsp_transport=debug")
        .init();

    println!("🚀 JetStreamProto Multi-Hop Tunnel Demo");
    println!("========================================\n");

    // Create a 4-hop configuration
    let config = MultiHopConfig {
        enabled: true,
        hop_timeout_secs: 10,
        auto_failover: true,
        health_check_interval_secs: 30,
        chain: vec![
            // Hop 1: WireGuard entry
            HopConfig::WireGuard(WireGuardConfig {
                endpoint: "127.0.0.1:51820".to_string(),
                private_key: "YourPrivateKeyHere==".to_string(),
                peer_public_key: "PeerPublicKeyHere==".to_string(),
                allowed_ips: vec!["0.0.0.0/0".to_string()],
                persistent_keepalive: 25,
                listen_port: 0,
            }),
            
            // Hop 2: Shadowsocks with obfuscation
            HopConfig::Shadowsocks(ShadowsocksConfig {
                endpoint: "127.0.0.1:8388".to_string(),
                password: "YourStrongPassword".to_string(),
                method: "aes-256-gcm".to_string(),
                obfs: "tls".to_string(),
                udp_relay: false,
                local_port: 0,
            }),
            
            // Hop 3: XRay VLESS/TLS
            HopConfig::XRay(XRayConfig {
                endpoint: "127.0.0.1:443".to_string(),
                server_name: "example.com".to_string(),
                uuid: "your-uuid-here".to_string(),
                tls: true,
                websocket: false,
                ws_path: "/".to_string(),
                local_port: 0,
            }),
            
            // Hop 4: WireGuard exit
            HopConfig::WireGuardExit(WireGuardConfig {
                endpoint: "127.0.0.1:51821".to_string(),
                private_key: "ExitPrivateKeyHere==".to_string(),
                peer_public_key: "ExitPeerPublicKeyHere==".to_string(),
                allowed_ips: vec!["0.0.0.0/0".to_string()],
                persistent_keepalive: 25,
                listen_port: 0,
            }),
        ],
    };

    println!("📋 Configuration:");
    println!("  - Hops: {}", config.chain.len());
    println!("  - Auto-failover: {}", config.auto_failover);
    println!("  - Health check interval: {}s\n", config.health_check_interval_secs);

    // Create and start the multi-hop engine
    println!("🔧 Creating multi-hop engine...");
    let mut engine = MultiHopEngine::new(config);

    println!("▶️  Starting multi-hop chain...");
    match engine.start().await {
        Ok(_) => {
            println!("✅ Multi-hop chain started successfully!\n");
            
            // Get router for metrics
            if let Some(router) = engine.router() {
                println!("📊 Chain Statistics:");
                println!("  - Total hops: {}", router.hop_count());
                println!("  - Total latency: {:.2}ms", router.total_latency_ms().await);
                println!("  - Throughput: {} bytes/sec\n", router.total_throughput().await);
            }
            
            // Send test data
            println!("📤 Sending test data through multi-hop chain...");
            let test_data = b"Hello from JetStreamProto Multi-Hop!";
            match engine.send(test_data).await {
                Ok(_) => println!("✅ Data sent successfully through all hops!"),
                Err(e) => println!("❌ Failed to send data: {}", e),
            }
            
            // Keep running for a bit
            println!("\n⏳ Running for 10 seconds...");
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            
            // Stop the engine
            println!("\n🛑 Stopping multi-hop engine...");
            engine.stop().await?;
            println!("✅ Multi-hop engine stopped cleanly");
        }
        Err(e) => {
            println!("❌ Failed to start multi-hop chain: {}", e);
            println!("\n💡 Note: This demo requires:");
            println!("  1. WireGuard servers at the configured endpoints");
            println!("  2. Shadowsocks server running");
            println!("  3. XRay binary installed and in PATH");
            println!("  4. Valid keys and credentials configured");
        }
    }

    Ok(())
}
