use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use jsp_core::types::header::Header;
use jsp_core::types::delivery::DeliveryMode;
use jsp_core::types::connection_id::ConnectionId;
use jsp_core::types::handshake::{ClientHello, ServerHello};
use jsp_core::serialization::flatbuffers_codec::FlatBuffersCodec;

fn header_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("header_serialization");
    
    let header = Header {
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
    };

    group.bench_function("serialize_header", |b| {
        b.iter(|| {
            FlatBuffersCodec::serialize_header(black_box(&header))
        })
    });

    let serialized = FlatBuffersCodec::serialize_header(&header);
    group.bench_function("deserialize_header", |b| {
        b.iter(|| {
            FlatBuffersCodec::deserialize_header(black_box(&serialized))
        })
    });

    group.finish();
}

fn handshake_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("handshake_serialization");

    let client_hello = ClientHello {
        version: 1,
        random: [1u8; 32],
        session_id: 12345,
        cipher_suites: vec![1, 2, 3],
        public_key: [2u8; 32],
        kyber_public_key: vec![3u8; 100],
        nonce: 999,
        timestamp: 88888,
        connection_id: ConnectionId::from_u64(54321),
        supported_formats: vec![0, 1],
    };

    group.bench_function("serialize_client_hello", |b| {
        b.iter(|| {
            FlatBuffersCodec::serialize_client_hello(black_box(&client_hello))
        })
    });

    let serialized_ch = FlatBuffersCodec::serialize_client_hello(&client_hello);
    group.bench_function("deserialize_client_hello", |b| {
        b.iter(|| {
            FlatBuffersCodec::deserialize_client_hello(black_box(&serialized_ch))
        })
    });

    let server_hello = ServerHello {
        version: 1,
        random: [4u8; 32],
        session_id: 12345,
        cipher_suite: 2,
        public_key: [5u8; 32],
        kyber_ciphertext: vec![6u8; 100],
        connection_id: ConnectionId::from_u64(98765),
        selected_format: 1,
    };

    group.bench_function("serialize_server_hello", |b| {
        b.iter(|| {
            FlatBuffersCodec::serialize_server_hello(black_box(&server_hello))
        })
    });

    let serialized_sh = FlatBuffersCodec::serialize_server_hello(&server_hello);
    group.bench_function("deserialize_server_hello", |b| {
        b.iter(|| {
            FlatBuffersCodec::deserialize_server_hello(black_box(&serialized_sh))
        })
    });

    group.finish();
}

criterion_group!(benches, header_benchmark, handshake_benchmark);
criterion_main!(benches);
