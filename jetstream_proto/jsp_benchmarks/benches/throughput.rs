use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput, BenchmarkId};
use jsp_benchmarks::utils::{setup_connection_pair, setup_connection_pair_with_config};
use jsp_transport::config::ConnectionConfig;
use jsp_core::types::delivery::DeliveryMode;
use tokio::runtime::Runtime;

fn throughput_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("throughput");
    
    // Test different message sizes
    for size in [1024, 4096, 16384, 65536].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.to_async(&rt).iter(|| async {
                let (mut client, mut server) = setup_connection_pair().await;
                
                // Open stream
                let stream_id = client.session
                    .open_best_effort_stream(1)
                    .expect("Failed to open stream");
                
                // Prepare data
                let data = vec![0u8; size];
                
                // Start server receiver
                let server_task = tokio::spawn(async move {
                    loop {
                        match server.recv().await {
                            Ok(_) => {},
                            Err(_) => break,
                        }
                    }
                });
                
                // Send data
                client
                    .send_on_stream(stream_id, black_box(&data))
                    .await
                    .expect("Failed to send");
                
                // Cleanup
                drop(client);
                let _ = server_task.await;
            });
        });
    }
    
    group.finish();
}

fn bulk_transfer_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("bulk_transfer");
    group.sample_size(10); // Fewer samples for long-running benchmarks
    
    // Test transferring 1MB of data
    let total_bytes = 1024 * 1024;
    let chunk_size = 16384;
    
    group.throughput(Throughput::Bytes(total_bytes as u64));
    
    group.bench_function("1MB_transfer", |b| {
        b.to_async(&rt).iter(|| async {
            let (mut client, mut server) = setup_connection_pair().await;
            
            // Open stream
            let stream_id = client.session
                .open_best_effort_stream(1)
                .expect("Failed to open stream");
            
            // Prepare data
            let data = vec![0u8; chunk_size];
            let num_chunks = total_bytes / chunk_size;
            
            // Start server receiver
            let server_task = tokio::spawn(async move {
                let mut received = 0;
                while received < total_bytes {
                    match server.recv().await {
                        Ok(packet) => {
                            received += packet.len();
                        },
                        Err(_) => break,
                    }
                }
            });
            
            // Send chunks
            for _ in 0..num_chunks {
                client
                    .send_on_stream(stream_id, black_box(&data))
                    .await
                    .expect("Failed to send");
            }
            
            // Cleanup
            drop(client);
            let _ = server_task.await;
        });
    });
    
    group.finish();
}



fn ack_batching_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("ack_batching");
    group.sample_size(10);
    
    let total_bytes = 1024 * 1024; // 1MB
    let chunk_size = 1400; // MTU-ish
    
    group.throughput(Throughput::Bytes(total_bytes as u64));
    
    for batch_size in [1, 10].iter() {
        group.bench_with_input(BenchmarkId::new("batch_size", batch_size), batch_size, |b, &batch_size| {
            b.to_async(&rt).iter(|| async {
                let mut config = ConnectionConfig::default();
                config.ack_batch_size = batch_size;
                // Make sure timeout doesn't trigger before batch size
                config.ack_batch_timeout_ms = 100; 
                
                let (mut client, mut server) = setup_connection_pair_with_config(config).await;
                
                // Open reliable stream (ACKs matter here)
                let stream_id = client.session
                    .open_reliable_stream(1)
                    .expect("Failed to open stream");
                
                let data = vec![0u8; chunk_size];
                let num_chunks = total_bytes / chunk_size;
                
                let server_task = tokio::spawn(async move {
                    let mut packets_received = 0;
                    while packets_received < num_chunks {
                        match server.recv().await {
                            Ok(packets) => {
                                packets_received += packets.len();
                            },
                            Err(_) => break,
                        }
                    }
                });
                
                for _ in 0..num_chunks {
                    client
                        .send_on_stream(stream_id, black_box(&data))
                        .await
                        .expect("Failed to send");
                }
                
                // Wait for server to finish receiving
                let _ = server_task.await;
                
                // Close connection to flush any pending ACKs/data
                client.close(jsp_core::types::control::CloseReason::Normal, None).await.unwrap();
            });
        });
    }
    group.finish();
}

criterion_group!(benches, throughput_benchmark, bulk_transfer_benchmark, ack_batching_benchmark);
criterion_main!(benches);
