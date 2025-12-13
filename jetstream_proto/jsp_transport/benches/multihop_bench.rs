//! Performance Benchmarks for Multi-Hop Tunnel Manager

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use jsp_transport::multihop::buffer_pool::BufferPool;
use jsp_transport::multihop::metrics::HopMetrics;
use std::time::Duration;

fn benchmark_buffer_pool(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_pool");
    
    for size in [1024, 4096, 16384, 65536].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let pool = BufferPool::new(100, size);
            b.iter(|| {
                let buf = pool.acquire(black_box(size));
                pool.release(buf);
            });
        });
    }
    
    group.finish();
}

fn benchmark_metrics_recording(c: &mut Criterion) {
    let mut group = c.benchmark_group("metrics");
    
    group.bench_function("record_sent", |b| {
        let metrics = HopMetrics::new();
        b.iter(|| {
            metrics.record_sent(black_box(1000), black_box(Duration::from_millis(10)));
        });
    });
    
    group.bench_function("record_received", |b| {
        let metrics = HopMetrics::new();
        b.iter(|| {
            metrics.record_received(black_box(1000), black_box(Duration::from_millis(10)));
        });
    });
    
    group.bench_function("get_metrics", |b| {
        let metrics = HopMetrics::new();
        // Pre-populate with some data
        for _ in 0..100 {
            metrics.record_sent(1000, Duration::from_millis(10));
        }
        
        b.iter(|| {
            black_box(metrics.avg_latency_ms());
            black_box(metrics.throughput_bps());
        });
    });
    
    group.finish();
}

fn benchmark_config_validation(c: &mut Criterion) {
    use jsp_transport::multihop::config::{WireGuardConfig, ShadowsocksConfig, XRayConfig};
    
    let mut group = c.benchmark_group("config_validation");
    
    group.bench_function("wireguard", |b| {
        let config = WireGuardConfig {
            endpoint: "127.0.0.1:51820".to_string(),
            private_key: "test_key".to_string(),
            peer_public_key: "test_peer_key".to_string(),
            allowed_ips: vec!["0.0.0.0/0".to_string()],
            persistent_keepalive: 25,
            listen_port: 0,
        };
        
        b.iter(|| {
            black_box(config.validate()).ok();
        });
    });
    
    group.bench_function("shadowsocks", |b| {
        let config = ShadowsocksConfig {
            endpoint: "127.0.0.1:8388".to_string(),
            password: "test_password".to_string(),
            method: "aes-256-gcm".to_string(),
            obfs: "tls".to_string(),
            udp_relay: false,
            local_port: 0,
        };
        
        b.iter(|| {
            black_box(config.validate()).ok();
        });
    });
    
    group.bench_function("xray", |b| {
        let config = XRayConfig {
            endpoint: "127.0.0.1:443".to_string(),
            server_name: "example.com".to_string(),
            uuid: "test-uuid".to_string(),
            tls: true,
            websocket: false,
            ws_path: "/".to_string(),
            local_port: 0,
        };
        
        b.iter(|| {
            black_box(config.validate()).ok();
        });
    });
    
    group.finish();
}

fn benchmark_yaml_serialization(c: &mut Criterion) {
    use jsp_transport::multihop::{MultiHopConfig, HopConfig};
    use jsp_transport::multihop::config::WireGuardConfig;
    
    let config = MultiHopConfig {
        enabled: true,
        hop_timeout_secs: 10,
        auto_failover: true,
        health_check_interval_secs: 30,
        chain: vec![
            HopConfig::WireGuard(WireGuardConfig {
                endpoint: "127.0.0.1:51820".to_string(),
                private_key: "test_key".to_string(),
                peer_public_key: "test_peer_key".to_string(),
                allowed_ips: vec!["0.0.0.0/0".to_string()],
                persistent_keepalive: 25,
                listen_port: 0,
            }),
        ],
    };
    
    let mut group = c.benchmark_group("yaml");
    
    group.bench_function("serialize", |b| {
        b.iter(|| {
            black_box(serde_yaml::to_string(&config)).ok();
        });
    });
    
    let yaml = serde_yaml::to_string(&config).unwrap();
    group.bench_function("deserialize", |b| {
        b.iter(|| {
            black_box(serde_yaml::from_str::<MultiHopConfig>(&yaml)).ok();
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_buffer_pool,
    benchmark_metrics_recording,
    benchmark_config_validation,
    benchmark_yaml_serialization
);

criterion_main!(benches);
