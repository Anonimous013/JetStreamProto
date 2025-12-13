//! Integration Tests for Multi-Hop Tunnel Manager

use jsp_transport::multihop::{MultiHopConfig, HopConfig, MultiHopEngine};
use jsp_transport::multihop::config::{WireGuardConfig, ShadowsocksConfig, XRayConfig};

#[tokio::test]
async fn test_config_validation() {
    // Valid config should pass
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
    
    assert!(config.validate().is_ok());
    
    // Empty chain with enabled=true should fail
    let invalid_config = MultiHopConfig {
        enabled: true,
        chain: vec![],
        ..Default::default()
    };
    
    assert!(invalid_config.validate().is_err());
}

#[tokio::test]
async fn test_yaml_serialization() {
    let config = MultiHopConfig {
        enabled: true,
        hop_timeout_secs: 10,
        auto_failover: true,
        health_check_interval_secs: 30,
        chain: vec![
            HopConfig::Shadowsocks(ShadowsocksConfig {
                endpoint: "127.0.0.1:8388".to_string(),
                password: "test_password".to_string(),
                method: "aes-256-gcm".to_string(),
                obfs: "tls".to_string(),
                udp_relay: false,
                local_port: 0,
            }),
        ],
    };
    
    // Serialize to YAML
    let yaml = serde_yaml::to_string(&config).unwrap();
    assert!(yaml.contains("shadowsocks"));
    assert!(yaml.contains("aes-256-gcm"));
    
    // Deserialize back
    let deserialized: MultiHopConfig = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(deserialized.chain.len(), 1);
}

#[tokio::test]
async fn test_engine_lifecycle() {
    // Create engine with empty chain (won't actually start hops)
    let config = MultiHopConfig {
        enabled: false,
        chain: vec![],
        ..Default::default()
    };
    
    let engine = MultiHopEngine::new(config);
    
    // Engine should start in Uninitialized state
    use jsp_transport::multihop::engine::EngineState;
    assert_eq!(engine.state().await, EngineState::Uninitialized);
}

#[test]
fn test_buffer_pool() {
    use jsp_transport::multihop::buffer_pool::BufferPool;
    
    let pool = BufferPool::new(10, 1024);
    
    // Acquire and release buffers
    let buf1 = pool.acquire(512);
    assert!(buf1.capacity() >= 512);
    assert_eq!(pool.size(), 9);
    
    pool.release(buf1);
    assert_eq!(pool.size(), 10);
    
    // Acquire larger buffer - just verify it works
    let buf2 = pool.acquire(2048);
    
    pool.release(buf2);
}

#[test]
fn test_hop_metrics() {
    use jsp_transport::multihop::metrics::HopMetrics;
    use std::time::Duration;
    
    let metrics = HopMetrics::new();
    
    // Record some data
    metrics.record_sent(1000, Duration::from_millis(10));
    metrics.record_sent(2000, Duration::from_millis(15));
    
    assert_eq!(metrics.bytes_sent(), 3000);
    assert_eq!(metrics.packets_sent(), 2);
    assert!(metrics.avg_latency_ms() > 0.0);
    
    // Record received data
    metrics.record_received(500, Duration::from_millis(12));
    assert_eq!(metrics.bytes_received(), 500);
    assert_eq!(metrics.packets_received(), 1);
}

#[test]
fn test_hop_stats() {
    use jsp_transport::multihop::hop::HopStats;
    
    let mut stats = HopStats::new();
    
    stats.record_sent(1000);
    stats.record_received(500);
    
    assert_eq!(stats.bytes_sent, 1000);
    assert_eq!(stats.bytes_received, 500);
    assert_eq!(stats.packets_sent, 1);
    assert_eq!(stats.packets_received, 1);
    
    // Test latency tracking
    stats.update_latency(10.0);
    assert_eq!(stats.avg_latency_ms, 10.0);
    
    stats.update_latency(20.0);
    // Should be exponential moving average
    assert!(stats.avg_latency_ms > 10.0 && stats.avg_latency_ms < 20.0);
}

#[test]
fn test_wireguard_config_validation() {
    let config = WireGuardConfig {
        endpoint: "127.0.0.1:51820".to_string(),
        private_key: "test_key".to_string(),
        peer_public_key: "test_peer_key".to_string(),
        allowed_ips: vec!["0.0.0.0/0".to_string()],
        persistent_keepalive: 25,
        listen_port: 0,
    };
    
    assert!(config.validate().is_ok());
    
    // Invalid endpoint
    let invalid = WireGuardConfig {
        endpoint: "invalid".to_string(),
        ..config.clone()
    };
    assert!(invalid.validate().is_err());
    
    // Empty private key
    let invalid = WireGuardConfig {
        private_key: "".to_string(),
        ..config
    };
    assert!(invalid.validate().is_err());
}

#[test]
fn test_shadowsocks_config_validation() {
    let config = ShadowsocksConfig {
        endpoint: "127.0.0.1:8388".to_string(),
        password: "test_password".to_string(),
        method: "aes-256-gcm".to_string(),
        obfs: "tls".to_string(),
        udp_relay: false,
        local_port: 0,
    };
    
    assert!(config.validate().is_ok());
    
    // Invalid method
    let invalid = ShadowsocksConfig {
        method: "invalid_method".to_string(),
        ..config.clone()
    };
    assert!(invalid.validate().is_err());
    
    // Invalid obfs
    let invalid = ShadowsocksConfig {
        obfs: "invalid_obfs".to_string(),
        ..config
    };
    assert!(invalid.validate().is_err());
}

#[test]
fn test_xray_config_validation() {
    let config = XRayConfig {
        endpoint: "127.0.0.1:443".to_string(),
        server_name: "example.com".to_string(),
        uuid: "test-uuid".to_string(),
        tls: true,
        websocket: false,
        ws_path: "/".to_string(),
        local_port: 0,
    };
    
    assert!(config.validate().is_ok());
    
    // Empty UUID
    let invalid = XRayConfig {
        uuid: "".to_string(),
        ..config.clone()
    };
    assert!(invalid.validate().is_err());
    
    // Empty server name
    let invalid = XRayConfig {
        server_name: "".to_string(),
        ..config
    };
    assert!(invalid.validate().is_err());
}
