use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use jsp_core::codec::{Codec, SerializationFormat};
use jsp_core::types::header::Header;
use jsp_core::types::delivery::DeliveryMode;
use jsp_core::types::connection_id::ConnectionId;

fn create_test_header() -> Header {
    Header {
        stream_id: 42,
        msg_type: 0x01,
        flags: 0,
        sequence: 1000,
        timestamp: 123456789,
        nonce: 999,
        delivery_mode: DeliveryMode::Reliable,
        piggybacked_ack: Some(500),
        payload_len: Some(1024),
        connection_id: Some(ConnectionId::from_u64(12345)),
    }
}

fn bench_serialization(c: &mut Criterion) {
    let header = create_test_header();
    
    let mut group = c.benchmark_group("header_serialization");
    
    // CBOR serialization
    group.bench_function("cbor", |b| {
        let codec = Codec::cbor();
        b.iter(|| {
            let _ = black_box(codec.encode_header(black_box(&header)).unwrap());
        });
    });
    
    // FlatBuffers serialization
    group.bench_function("flatbuffers", |b| {
        let codec = Codec::flatbuffers();
        b.iter(|| {
            let _ = black_box(codec.encode_header(black_box(&header)).unwrap());
        });
    });
    
    group.finish();
}

fn bench_deserialization(c: &mut Criterion) {
    let header = create_test_header();
    
    let cbor_codec = Codec::cbor();
    let fb_codec = Codec::flatbuffers();
    
    let cbor_data = cbor_codec.encode_header(&header).unwrap();
    let fb_data = fb_codec.encode_header(&header).unwrap();
    
    let mut group = c.benchmark_group("header_deserialization");
    
    // CBOR deserialization
    group.bench_function("cbor", |b| {
        b.iter(|| {
            let _ = black_box(cbor_codec.decode_header(black_box(&cbor_data)).unwrap());
        });
    });
    
    // FlatBuffers deserialization (zero-copy)
    group.bench_function("flatbuffers", |b| {
        b.iter(|| {
            let _ = black_box(fb_codec.decode_header(black_box(&fb_data)).unwrap());
        });
    });
    
    group.finish();
}

fn bench_roundtrip(c: &mut Criterion) {
    let header = create_test_header();
    
    let mut group = c.benchmark_group("header_roundtrip");
    
    // CBOR roundtrip
    group.bench_function("cbor", |b| {
        let codec = Codec::cbor();
        b.iter(|| {
            let encoded = codec.encode_header(black_box(&header)).unwrap();
            let _ = black_box(codec.decode_header(&encoded).unwrap());
        });
    });
    
    // FlatBuffers roundtrip
    group.bench_function("flatbuffers", |b| {
        let codec = Codec::flatbuffers();
        b.iter(|| {
            let encoded = codec.encode_header(black_box(&header)).unwrap();
            let _ = black_box(codec.decode_header(&encoded).unwrap());
        });
    });
    
    group.finish();
}

fn bench_size_comparison(c: &mut Criterion) {
    let header = create_test_header();
    
    let cbor_codec = Codec::cbor();
    let fb_codec = Codec::flatbuffers();
    
    let cbor_data = cbor_codec.encode_header(&header).unwrap();
    let fb_data = fb_codec.encode_header(&header).unwrap();
    
    println!("\n=== Size Comparison ===");
    println!("CBOR size: {} bytes", cbor_data.len());
    println!("FlatBuffers size: {} bytes", fb_data.len());
    println!("Difference: {} bytes ({:.1}%)", 
        fb_data.len() as i32 - cbor_data.len() as i32,
        ((fb_data.len() as f64 / cbor_data.len() as f64) - 1.0) * 100.0
    );
    
    let mut group = c.benchmark_group("size_overhead");
    group.bench_function("cbor", |b| b.iter(|| cbor_data.len()));
    group.bench_function("flatbuffers", |b| b.iter(|| fb_data.len()));
    group.finish();
}

criterion_group!(
    benches,
    bench_serialization,
    bench_deserialization,
    bench_roundtrip,
    bench_size_comparison
);
criterion_main!(benches);
