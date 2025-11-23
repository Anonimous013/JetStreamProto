use criterion::{black_box, criterion_group, criterion_main, Criterion};
use jsp_benchmarks::utils::setup_connection_pair;
use jsp_core::types::delivery::DeliveryMode;
use tokio::runtime::Runtime;
use std::time::Instant;

fn ping_pong_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("ping_pong_rtt", |b| {
        b.to_async(&rt).iter(|| async {
            let (mut client, mut server) = setup_connection_pair().await;
            
            // Open stream
            let stream_id = client.session
                .open_reliable_stream(1)
                .expect("Failed to open stream");
            
            // Prepare ping data
            let ping_data = vec![1u8; 64];
            let pong_data = vec![2u8; 64];
            
            // Start server echo
            let server_task = tokio::spawn(async move {
                // Receive ping
                let packet = server.recv().await.expect("Failed to receive ping");
                
                // Send pong
                let stream_id = packet[4] as u32; // Extract stream_id from header (simplified)
                server
                    .send_on_stream(stream_id, &pong_data)
                    .await
                    .expect("Failed to send pong");
                
                server
            });
            
            // Measure RTT
            let start = Instant::now();
            
            // Send ping
            client
                .send_on_stream(stream_id, black_box(&ping_data))
                .await
                .expect("Failed to send ping");
            
            // Receive pong
            let _ = client.recv().await.expect("Failed to receive pong");
            
            let _rtt = start.elapsed();
            
            // Cleanup
            drop(client);
            let _ = server_task.await;
        });
    });
}

fn handshake_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("handshake_1rtt", |b| {
        b.to_async(&rt).iter(|| async {
            // Measure handshake time (included in setup_connection_pair)
            let start = Instant::now();
            
            let (_client, _server) = setup_connection_pair().await;
            
            let _handshake_time = start.elapsed();
            
            // Cleanup happens automatically when dropped
        });
    });
}

fn stream_open_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("stream_open", |b| {
        b.to_async(&rt).iter(|| async {
            let (mut client, server) = setup_connection_pair().await;
            
            // Measure stream open time
            let start = Instant::now();
            
            let _stream_id = client.session
                .open_best_effort_stream(1)
                .expect("Failed to open stream");
            
            let _open_time = start.elapsed();
            
            // Cleanup
            drop(client);
            drop(server);
        });
    });
}

criterion_group!(benches, ping_pong_latency, handshake_latency, stream_open_latency);
criterion_main!(benches);
