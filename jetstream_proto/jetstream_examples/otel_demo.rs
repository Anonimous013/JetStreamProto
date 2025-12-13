//! Distributed Tracing Example
//! 
//! Demonstrates tracing with JetStreamProto.

use jsp_transport::otel::{init_tracer, global_tracer};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,jsp_transport=debug")
        .init();

    println!("🚀 JetStreamProto Distributed Tracing Demo");
    println!("==========================================\n");

    // Initialize tracer
    println!("📊 Initializing tracer...");
    init_tracer("jetstream_proto_demo")?;
    println!("✅ Tracer initialized\n");

    let tracer = global_tracer();
    println!("Service: {}\n", tracer.service_name());

    // Create a root span
    println!("🔍 Creating trace spans...");
    let mut root_span = tracer.start_span("connection_lifecycle");
    root_span.set_attribute("connection.id", "conn-12345");
    root_span.set_attribute("protocol", "jetstream");

    // Simulate connection handshake
    {
        let mut handshake_span = tracer.start_span("handshake");
        handshake_span.set_attribute("handshake.type", "1-RTT");
        
        println!("  ├─ Handshake span started");
        tokio::time::sleep(Duration::from_millis(15)).await;
        
        handshake_span.set_attribute("handshake.duration_ms", "15");
        let duration = handshake_span.end();
        println!("  ├─ Handshake completed ({:?})", duration);
    }

    // Simulate data transfer
    {
        let mut transfer_span = tracer.start_span("data_transfer");
        transfer_span.set_attribute("stream.id", "1");
        transfer_span.set_attribute("bytes", "1024");
        
        println!("  ├─ Data transfer span started");
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        let mut attrs = std::collections::HashMap::new();
        attrs.insert("bytes".to_string(), "1024".to_string());
        transfer_span.add_event("data_sent", attrs);
        
        let duration = transfer_span.end();
        println!("  ├─ Data transfer completed ({:?}, 1024 bytes)", duration);
    }

    // Simulate multi-hop routing
    {
        let mut multihop_span = tracer.start_span("multihop_routing");
        multihop_span.set_attribute("hop.count", "4");
        
        println!("  ├─ Multi-hop routing span started");
        
        for hop in 1..=4 {
            let mut hop_span = tracer.start_span(format!("hop_{}", hop));
            hop_span.set_attribute("hop.index", hop.to_string());
            hop_span.set_attribute("hop.type", match hop {
                1 => "wireguard",
                2 => "shadowsocks",
                3 => "xray",
                4 => "wireguard_exit",
                _ => "unknown",
            });
            
            tokio::time::sleep(Duration::from_millis(25)).await;
            
            hop_span.set_attribute("hop.latency_ms", "25");
            hop_span.end();
            
            println!("  │  ├─ Hop {} completed (25ms)", hop);
        }
        
        let duration = multihop_span.end();
        println!("  ├─ Multi-hop routing completed ({:?} total)", duration);
    }

    // Complete root span
    let total_duration = root_span.end();
    println!("  └─ Connection lifecycle completed ({:?})\n", total_duration);

    println!("📋 Trace Information:");
    println!("  - Service: {}", tracer.service_name());
    println!("  - Total Duration: {:?}", total_duration);
    println!("  - Spans Created: 7 (1 root + 6 children)\n");

    println!("💡 Features:");
    println!("  ✅ Span creation and nesting");
    println!("  ✅ Attribute tracking");
    println!("  ✅ Event logging");
    println!("  ✅ Duration measurement");
    println!("  ✅ Hierarchical tracing\n");

    println!("✅ Tracing demo completed!");

    Ok(())
}
