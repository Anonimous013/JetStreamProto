use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use bytes::Bytes;
use jsp_transport::reliability::ReliabilityLayer;
use jsp_core::types::delivery::DeliveryMode;

fn reliability_tracking_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("reliability_tracking");
    
    for size in [1024, 64*1024].iter() {
        group.bench_with_input(
            BenchmarkId::new("track_sent_packet", size),
            size,
            |b, &size| {
                let data = Bytes::from(vec![0u8; size]);
                let mut reliability = ReliabilityLayer::new();
                let mut seq = 1;
                
                b.iter(|| {
                    reliability.track_sent_packet(seq, data.clone(), DeliveryMode::Reliable);
                    seq += 1;
                    // Cleanup to avoid OOM
                    if seq % 1000 == 0 {
                        reliability = ReliabilityLayer::new();
                    }
                });
            },
        );
    }
    group.finish();
}

fn bytes_cloning_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("bytes_cloning");

    for size in [1024, 64*1024, 1024*1024].iter() {
        group.bench_with_input(
            BenchmarkId::new("clone_bytes", size),
            size,
            |b, &size| {
                let data = Bytes::from(vec![0u8; size]);
                b.iter(|| {
                    black_box(data.clone());
                });
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("clone_vec", size),
            size,
            |b, &size| {
                let data = vec![0u8; size];
                b.iter(|| {
                    black_box(data.clone());
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, reliability_tracking_benchmark, bytes_cloning_benchmark);
criterion_main!(benches);
