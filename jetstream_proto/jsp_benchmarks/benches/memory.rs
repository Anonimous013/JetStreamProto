use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use jsp_benchmarks::utils::setup_connection_pair;
use jsp_core::types::delivery::DeliveryMode;
use tokio::runtime::Runtime;

fn allocation_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("allocations");
    
    // Test memory allocations for different operations
    for num_messages in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("send_messages", num_messages),
            num_messages,
            |b, &num_messages| {
                b.to_async(&rt).iter(|| async {
                    let (mut client, mut server) = setup_connection_pair().await;
                    
                    // Open stream
                    let stream_id = client.session
                        .open_best_effort_stream(1)
                        .expect("Failed to open stream");
                    
                    // Prepare data
                    let data = vec![0u8; 1024];
                    
                    // Start server receiver
                    let server_task = tokio::spawn(async move {
                        for _ in 0..num_messages {
                            let _ = server.recv().await;
                        }
                    });
                    
                    // Send messages
                    for _ in 0..num_messages {
                        client
                            .send_on_stream(stream_id, black_box(&data))
                            .await
                            .expect("Failed to send");
                    }
                    
                    // Cleanup
                    drop(client);
                    let _ = server_task.await;
                });
            },
        );
    }
    
    group.finish();
}

fn stream_creation_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("stream_creation");
    
    // Test memory allocations for stream creation
    for num_streams in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("create_streams", num_streams),
            num_streams,
            |b, &num_streams| {
                b.to_async(&rt).iter(|| async {
                    let (mut client, server) = setup_connection_pair().await;
                    
                    // Create multiple streams
                    let mut streams = Vec::new();
                    for _ in 0..num_streams {
                        let stream_id = client.session
                            .open_best_effort_stream(1)
                            .expect("Failed to open stream");
                        streams.push(stream_id);
                    }
                    
                    // Cleanup
                    drop(client);
                    drop(server);
                });
            },
        );
    }
    
    group.finish();
}

fn connection_creation_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("connection_creation", |b| {
        b.to_async(&rt).iter(|| async {
            let (_client, _server) = setup_connection_pair().await;
            // Cleanup happens automatically when dropped
        });
    });
}

criterion_group!(
    benches,
    allocation_benchmark,
    stream_creation_benchmark,
    connection_creation_benchmark
);
criterion_main!(benches);
