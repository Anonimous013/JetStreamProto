use jsp_transport::connection::Connection;
use jsp_transport::config::ConnectionConfig;
use jsp_core::types::control::CloseReason;
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 JetStreamProto Benchmark Client");
    println!("{}", "=".repeat(50));
    
    let args: Vec<String> = std::env::args().collect();
    let server_addr = args.get(1).map(|s| s.as_str()).unwrap_or("127.0.0.1:8080");
    
    println!("\n🌐 Server: {}", server_addr);
    println!("⏱️  Duration: 30 seconds");
    println!();
    
    // Connect
    let config = ConnectionConfig {
        heartbeat_interval: Duration::from_secs(10),
        ..Default::default()
    };
    
    println!("🔌 Connecting...");
    let conn = Connection::connect_with_config(server_addr, config).await?;
    println!("✅ Connected!\n");
    
    let conn = Arc::new(tokio::sync::Mutex::new(conn));
    
    // Metrics
    let messages_sent = Arc::new(AtomicU64::new(0));
    let bytes_sent = Arc::new(AtomicU64::new(0));
    let messages_received = Arc::new(AtomicU64::new(0));
    let bytes_received = Arc::new(AtomicU64::new(0));
    let latencies = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    
    // Sender task
    let conn_send = conn.clone();
    let messages_sent_clone = messages_sent.clone();
    let bytes_sent_clone = bytes_sent.clone();
    let latencies_clone = latencies.clone();
    
    let sender = tokio::spawn(async move {
        let payload = vec![0u8; 1024]; // 1KB payload
        let mut interval = tokio::time::interval(Duration::from_millis(10));
        
        for _i in 0..3000 {
            interval.tick().await;
            
            let start = Instant::now();
            let mut conn = conn_send.lock().await;
            
            if let Ok(_) = conn.send_on_stream(1, &payload).await {
                messages_sent_clone.fetch_add(1, Ordering::Relaxed);
                bytes_sent_clone.fetch_add(payload.len() as u64, Ordering::Relaxed);
                
                // Record latency (simplified - actual RTT would need echo)
                let elapsed = start.elapsed();
                latencies_clone.lock().await.push(elapsed.as_micros() as u64);
            }
        }
    });
    
    // Receiver task
    let conn_recv = conn.clone();
    let messages_received_clone = messages_received.clone();
    let bytes_received_clone = bytes_received.clone();
    
    let receiver = tokio::spawn(async move {
        loop {
            let mut conn = conn_recv.lock().await;
            match conn.recv().await {
                Ok(packets) => {
                    for (_, data) in packets {
                        messages_received_clone.fetch_add(1, Ordering::Relaxed);
                        bytes_received_clone.fetch_add(data.len() as u64, Ordering::Relaxed);
                    }
                }
                Err(_) => break,
            }
        }
    });
    
    // Progress reporter
    let messages_sent_reporter = messages_sent.clone();
    let bytes_sent_reporter = bytes_sent.clone();
    let start_time = Instant::now();
    
    let reporter = tokio::spawn(async move {
        for i in 0..30 {
            sleep(Duration::from_secs(1)).await;
            
            let msgs = messages_sent_reporter.load(Ordering::Relaxed);
            let bytes = bytes_sent_reporter.load(Ordering::Relaxed);
            let elapsed = start_time.elapsed().as_secs_f64();
            
            let msg_rate = msgs as f64 / elapsed;
            let throughput = (bytes as f64 / elapsed) / 1_048_576.0; // MB/s
            
            println!("[{:2}s] Messages: {} | Rate: {:.0} msg/s | Throughput: {:.2} MB/s",
                i + 1, msgs, msg_rate, throughput);
        }
    });
    
    // Wait for completion
    let _ = tokio::join!(sender, reporter);
    receiver.abort();
    
    // Final statistics
    println!("\n{}", "=".repeat(50));
    println!("📊 Benchmark Results");
    println!("{}", "=".repeat(50));
    
    let total_msgs_sent = messages_sent.load(Ordering::Relaxed);
    let total_bytes_sent = bytes_sent.load(Ordering::Relaxed);
    let total_msgs_recv = messages_received.load(Ordering::Relaxed);
    let total_bytes_recv = bytes_received.load(Ordering::Relaxed);
    
    println!("\n📤 Sent:");
    println!("  Messages: {}", total_msgs_sent);
    println!("  Bytes: {} ({:.2} MB)", total_bytes_sent, total_bytes_sent as f64 / 1_048_576.0);
    println!("  Avg Rate: {:.0} msg/s", total_msgs_sent as f64 / 30.0);
    println!("  Avg Throughput: {:.2} MB/s", (total_bytes_sent as f64 / 30.0) / 1_048_576.0);
    
    println!("\n📥 Received:");
    println!("  Messages: {}", total_msgs_recv);
    println!("  Bytes: {} ({:.2} MB)", total_bytes_recv, total_bytes_recv as f64 / 1_048_576.0);
    
    // Latency statistics
    let mut lats = latencies.lock().await;
    if !lats.is_empty() {
        lats.sort();
        let p50 = lats[lats.len() / 2];
        let p95 = lats[(lats.len() * 95) / 100];
        let p99 = lats[(lats.len() * 99) / 100];
        
        println!("\n⏱️  Latency (send time):");
        println!("  p50: {} μs", p50);
        println!("  p95: {} μs", p95);
        println!("  p99: {} μs", p99);
    }
    
    // Close
    let mut conn = conn.lock().await;
    conn.close(CloseReason::Normal, Some("Benchmark complete".to_string())).await?;
    
    println!("\n✅ Benchmark complete!");
    
    Ok(())
}
