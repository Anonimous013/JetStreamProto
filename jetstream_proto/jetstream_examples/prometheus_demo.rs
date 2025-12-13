//! Prometheus Metrics Example
//! 
//! Demonstrates how to use Prometheus metrics with JetStreamProto.

use jsp_transport::prometheus::{global_registry, MetricsExporter};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,jsp_transport=debug")
        .init();

    println!("🚀 JetStreamProto Prometheus Metrics Demo");
    println!("==========================================\n");

    // Get the global metrics registry
    let registry = global_registry();
    
    // Simulate some metrics
    println!("📊 Recording sample metrics...");
    
    // Record connections
    registry.record_connection();
    registry.record_connection();
    registry.record_connection();
    
    // Record handshakes
    registry.record_handshake(0.015); // 15ms
    registry.record_handshake(0.012); // 12ms
    registry.record_handshake(0.018); // 18ms
    
    // Record data transfer
    registry.record_bytes_sent(1024 * 1024); // 1MB
    registry.record_bytes_received(512 * 1024); // 512KB
    registry.record_packet_sent();
    registry.record_packet_sent();
    registry.record_packet_received();
    
    // Record some errors
    registry.record_error();
    registry.record_timeout();
    
    // Close one connection
    registry.record_connection_close(30.5); // 30.5 seconds
    
    println!("✅ Metrics recorded\n");
    
    // Export metrics to console
    println!("📋 Current Metrics:");
    println!("{}", "=".repeat(80));
    let metrics = jsp_transport::prometheus::export_metrics()?;
    println!("{}", metrics);
    println!("{}", "=".repeat(80));
    
    // Start HTTP metrics server
    let addr: SocketAddr = "127.0.0.1:9090".parse()?;
    println!("\n🌐 Starting metrics HTTP server on http://{}", addr);
    println!("   Metrics endpoint: http://{}/metrics", addr);
    println!("   Health endpoint:  http://{}/health", addr);
    println!("\n💡 You can now scrape metrics with Prometheus!");
    println!("   Add this to your prometheus.yml:");
    println!("   ```yaml");
    println!("   scrape_configs:");
    println!("     - job_name: 'jetstream_proto'");
    println!("       static_configs:");
    println!("         - targets: ['{}']", addr);
    println!("   ```");
    println!("\n⏳ Server running... Press Ctrl+C to stop");
    
    let exporter = MetricsExporter::new(addr);
    exporter.start().await?;

    Ok(())
}
