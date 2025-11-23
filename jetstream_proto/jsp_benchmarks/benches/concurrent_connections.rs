use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use jsp_transport::connection::Connection;
use jsp_transport::config::ConnectionConfig;
use std::time::Duration;
use tokio::runtime::Runtime;

fn concurrent_connections_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_connections");
    group.sample_size(10); // Fewer samples for expensive benchmarks
    
    let connection_counts = vec![10, 50, 100, 500];
    
    for count in connection_counts {
        group.bench_with_input(
            BenchmarkId::new("establish", count),
            &count,
            |b, &count| {
                b.to_async(Runtime::new().unwrap()).iter(|| async move {
                    black_box(establish_connections(count).await)
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("send_receive", count),
            &count,
            |b, &count| {
                b.to_async(Runtime::new().unwrap()).iter(|| async move {
                    black_box(send_receive_concurrent(count).await)
                });
            },
        );
    }
    
    group.finish();
}

async fn establish_connections(count: usize) -> Result<(), Box<dyn std::error::Error>> {
    // Start server
    let config = ConnectionConfig {
        heartbeat_interval: Duration::from_secs(60),
        ..Default::default()
    };
    
    let server = Connection::listen_with_config("127.0.0.1:0", config.clone()).await?;
    let server_addr = server.local_addr()?;
    
    // Spawn server task
    tokio::spawn(async move {
        let _server = server;
        tokio::time::sleep(Duration::from_secs(10)).await;
    });
    
    // Create connections
    let mut handles = Vec::new();
    
    for _ in 0..count {
        let addr = server_addr.to_string();
        let cfg = config.clone();
        
        let handle = tokio::spawn(async move {
            Connection::connect_with_config(&addr, cfg).await
        });
        
        handles.push(handle);
    }
    
    // Wait for all connections
    for handle in handles {
        let _ = handle.await?;
    }
    
    Ok(())
}

async fn send_receive_concurrent(count: usize) -> Result<(), Box<dyn std::error::Error>> {
    // Start server
    let config = ConnectionConfig {
        heartbeat_interval: Duration::from_secs(60),
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
    
    // Create connections and send/receive
    let mut handles = Vec::new();
    
    for _ in 0..count {
        let addr = server_addr.to_string();
        let cfg = config.clone();
        
        let handle = tokio::spawn(async move {
            let mut conn = Connection::connect_with_config(&addr, cfg).await?;
            
            // Send message
            conn.send_on_stream(1, b"test").await?;
            
            // Receive echo
            let _ = conn.recv().await?;
            
            Ok::<(), Box<dyn std::error::Error>>(())
        });
        
        handles.push(handle);
    }
    
    // Wait for all
    for handle in handles {
        let _ = handle.await?;
    }
    
    Ok(())
}

criterion_group!(benches, concurrent_connections_benchmark);
criterion_main!(benches);
