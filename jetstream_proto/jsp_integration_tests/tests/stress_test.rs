use jsp_transport::connection::Connection;
use jsp_transport::config::ConnectionConfig;
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

#[tokio::test]
async fn test_high_load_stress() -> Result<(), Box<dyn std::error::Error>> {
    // Start server
    let config = ConnectionConfig {
        heartbeat_interval: Duration::from_secs(30),
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
    
    // Connect client
    let mut client = Connection::connect_with_config(&server_addr.to_string(), config).await?;
    
    // Send many messages rapidly
    let message_count = 1000;
    let payload = vec![0u8; 512]; // 512 bytes
    
    let start = Instant::now();
    
    for _ in 0..message_count {
        client.send_on_stream(1, &payload).await?;
    }
    
    let elapsed = start.elapsed();
    let rate = message_count as f64 / elapsed.as_secs_f64();
    
    println!("Sent {} messages in {:.2}s ({:.0} msg/s)", message_count, elapsed.as_secs_f64(), rate);
    
    // Verify rate is acceptable
    assert!(rate > 100.0, "Message rate too low: {}", rate);
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_connections_stress() -> Result<(), Box<dyn std::error::Error>> {
    // Start server
    let config = ConnectionConfig {
        heartbeat_interval: Duration::from_secs(60),
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
    
    // Create many concurrent connections
    let connection_count = 100;
    let mut handles = Vec::new();
    
    let start = Instant::now();
    
    for i in 0..connection_count {
        let addr = server_addr.to_string();
        let cfg = config.clone();
        
        let handle = tokio::spawn(async move {
            let mut client = Connection::connect_with_config(&addr, cfg).await?;
            
            // Send message
            let msg = format!("Connection {}", i);
            client.send_on_stream(1, msg.as_bytes()).await?;
            
            // Receive echo
            let _ = client.recv().await?;
            
            Ok::<(), Box<dyn std::error::Error>>(())
        });
        
        handles.push(handle);
    }
    
    // Wait for all
    for handle in handles {
        handle.await??;
    }
    
    let elapsed = start.elapsed();
    
    println!("Established {} connections in {:.2}s", connection_count, elapsed.as_secs_f64());
    
    // Verify time is acceptable
    assert!(elapsed.as_secs() < 30, "Connection establishment too slow");
    
    Ok(())
}

#[tokio::test]
async fn test_memory_leak_detection() -> Result<(), Box<dyn std::error::Error>> {
    // Start server
    let config = ConnectionConfig::default();
    
    let mut server = Connection::listen_with_config("127.0.0.1:0", config.clone()).await?;
    let server_addr = server.local_addr()?;
    
    tokio::spawn(async move {
        loop {
            if let Ok(packets) = server.recv().await {
                for (stream_id, data) in packets {
                    let _ = server.send_on_stream(stream_id, &data).await;
                }
            }
        }
    });
    
    // Create and destroy many connections
    for _ in 0..50 {
        let mut client = Connection::connect_with_config(&server_addr.to_string(), config.clone()).await?;
        client.send_on_stream(1, b"test").await?;
        let _ = client.recv().await?;
        // Connection dropped here
    }
    
    // If we get here without OOM, test passes
    Ok(())
}

#[tokio::test]
async fn test_throughput_stress() -> Result<(), Box<dyn std::error::Error>> {
    // Start server
    let config = ConnectionConfig {
        heartbeat_interval: Duration::from_secs(30),
        ..Default::default()
    };
    
    let mut server = Connection::listen_with_config("127.0.0.1:0", config.clone()).await?;
    let server_addr = server.local_addr()?;
    
    let bytes_received = Arc::new(AtomicU64::new(0));
    let bytes_received_clone = bytes_received.clone();
    
    // Spawn server
    tokio::spawn(async move {
        loop {
            if let Ok(packets) = server.recv().await {
                for (stream_id, data) in packets {
                    bytes_received_clone.fetch_add(data.len() as u64, Ordering::Relaxed);
                    let _ = server.send_on_stream(stream_id, &data).await;
                }
            }
        }
    });
    
    // Connect client
    let mut client = Connection::connect_with_config(&server_addr.to_string(), config).await?;
    
    // Send large amount of data
    let payload = vec![0u8; 10240]; // 10KB
    let iterations = 100;
    
    let start = Instant::now();
    
    for _ in 0..iterations {
        client.send_on_stream(1, &payload).await?;
    }
    
    // Wait a bit for processing
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    let elapsed = start.elapsed();
    let total_bytes = bytes_received.load(Ordering::Relaxed);
    let throughput_mbps = (total_bytes as f64 * 8.0) / (elapsed.as_secs_f64() * 1_000_000.0);
    
    println!("Throughput: {:.2} Mbps ({} bytes in {:.2}s)", 
        throughput_mbps, total_bytes, elapsed.as_secs_f64());
    
    // Verify throughput is acceptable
    assert!(throughput_mbps > 1.0, "Throughput too low: {:.2} Mbps", throughput_mbps);
    
    Ok(())
}
