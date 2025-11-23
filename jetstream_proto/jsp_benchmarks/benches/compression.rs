use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use jsp_transport::compression::lz4::Lz4Compressor;
use bytes::Bytes;

fn compression_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression");
    
    // Test different payload types and sizes
    let test_cases = vec![
        ("json_1kb", generate_json(1024)),
        ("json_10kb", generate_json(10 * 1024)),
        ("text_1kb", generate_text(1024)),
        ("text_10kb", generate_text(10 * 1024)),
        ("binary_1kb", generate_binary(1024)),
        ("binary_10kb", generate_binary(10 * 1024)),
        ("binary_100kb", generate_binary(100 * 1024)),
    ];
    
    for (name, data) in test_cases {
        group.throughput(Throughput::Bytes(data.len() as u64));
        
        // Compression benchmark
        group.bench_with_input(
            BenchmarkId::new("compress", name),
            &data,
            |b, data| {
                let compressor = Lz4Compressor::new();
                b.iter(|| {
                    black_box(compressor.compress(data).unwrap())
                });
            },
        );
        
        // Decompression benchmark
        let compressor = Lz4Compressor::new();
        let compressed = compressor.compress(&data).unwrap();
        
        group.bench_with_input(
            BenchmarkId::new("decompress", name),
            &compressed,
            |b, compressed| {
                let compressor = Lz4Compressor::new();
                b.iter(|| {
                    black_box(compressor.decompress(compressed).unwrap())
                });
            },
        );
    }
    
    group.finish();
}

fn generate_json(size: usize) -> Bytes {
    // Generate JSON-like data (highly compressible)
    let mut data = String::from("{");
    let entry = r#""key":"value","#;
    let entries_needed = size / entry.len();
    
    for i in 0..entries_needed {
        data.push_str(&format!(r#""key{}":"value{}","#, i, i));
    }
    data.push('}');
    
    Bytes::from(data)
}

fn generate_text(size: usize) -> Bytes {
    // Generate repetitive text (compressible)
    let text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ";
    let repetitions = (size / text.len()) + 1;
    let data = text.repeat(repetitions);
    
    Bytes::from(data[..size].to_vec())
}

fn generate_binary(size: usize) -> Bytes {
    // Generate random binary data (not compressible)
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hash, Hasher};
    
    let mut data = Vec::with_capacity(size);
    let hasher_builder = RandomState::new();
    
    for i in 0..size {
        let mut hasher = hasher_builder.build_hasher();
        i.hash(&mut hasher);
        data.push((hasher.finish() & 0xFF) as u8);
    }
    
    Bytes::from(data)
}

criterion_group!(benches, compression_benchmark);
criterion_main!(benches);
