use criterion::{black_box, criterion_group, criterion_main, Criterion};
use jsp_core::compression::header_compression::HeaderCompressor;
use jsp_core::types::header::{Header, FRAME_TYPE_DATA};
use jsp_core::types::delivery::DeliveryMode;
use std::time::{SystemTime, UNIX_EPOCH};

fn benchmark_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("header_compression");

    // Benchmark compression of a sequence of 100 headers
    group.bench_function("compress_sequence_100", |b| {
        b.iter_batched(
            || {
                let compressor = HeaderCompressor::new();
                let mut headers = Vec::new();
                let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
                
                for i in 0..100 {
                    headers.push(Header::new(
                        1, // Stream ID
                        FRAME_TYPE_DATA,
                        0,
                        i as u64, // Sequence
                        start_time + i as u64 * 10, // Timestamp
                        0,
                        DeliveryMode::Reliable,
                        None,
                        Some(100),
                    ));
                }
                (compressor, headers)
            },
            |(mut compressor, headers)| {
                for header in headers {
                    let _ = black_box(compressor.compress(&header));
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });
    
    // Benchmark standard CBOR serialization for comparison
    group.bench_function("serialize_cbor_baseline_100", |b| {
        b.iter_batched(
            || {
                 let mut headers = Vec::new();
                let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
                
                for i in 0..100 {
                    headers.push(Header::new(
                        1, // Stream ID
                        FRAME_TYPE_DATA,
                        0,
                        i as u64, // Sequence
                        start_time + i as u64 * 10, // Timestamp
                        0,
                        DeliveryMode::Reliable,
                        None,
                        Some(100),
                    ));
                }
                headers
            },
            |headers| {
                for header in headers {
                    let _ = black_box(serde_cbor::to_vec(&header).unwrap());
                }
            },
             criterion::BatchSize::SmallInput,
        );
    });

    // Benchmark decompression
    group.bench_function("decompress_sequence_100", |b| {
        b.iter_batched(
            || {
                let mut compressor = HeaderCompressor::new();
                let decompressor = HeaderCompressor::new(); // Used as decompressor
                let mut compressed_packets = Vec::new();
                let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
                
                for i in 0..100 {
                    let header = Header::new(
                        1,
                        FRAME_TYPE_DATA,
                        0,
                        i as u64,
                        start_time + i as u64 * 10,
                        0,
                        DeliveryMode::Reliable,
                        None,
                        Some(100),
                    );
                    compressed_packets.push(compressor.compress(&header));
                }
                (decompressor, compressed_packets)
            },
            |(mut decompressor, packets)| {
                for packet in packets {
                    let _ = black_box(decompressor.decompress(&packet));
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(benches, benchmark_compression);
criterion_main!(benches);

#[test]
fn measure_compression_ratio() {
    let mut compressor = HeaderCompressor::new();
    let mut total_original_size = 0;
    let mut total_compressed_size = 0;
    let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;

    for i in 0..100 {
        let header = Header::new(
            1,
            FRAME_TYPE_DATA,
            0,
            i as u64,
            start_time + i as u64 * 10,
            0,
            DeliveryMode::Reliable,
            None,
            Some(100),
        );
        
        let original = serde_cbor::to_vec(&header).unwrap();
        let compressed = compressor.compress(&header);
        
        total_original_size += original.len();
        total_compressed_size += compressed.len();
    }
    
    println!("Total Original Size: {} bytes", total_original_size);
    println!("Total Compressed Size: {} bytes", total_compressed_size);
    println!("Compression Ratio: {:.2}", total_original_size as f64 / total_compressed_size as f64);
    println!("Space Saving: {:.2}%", 100.0 * (1.0 - total_compressed_size as f64 / total_original_size as f64));
}
