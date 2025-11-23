use jsp_transport::heartbeat::AppState;
use jsp_transport::network_status::NetworkType;
use jsp_transport::compression::adaptive::AdaptiveCompression;
use std::time::Duration;

#[tokio::test]
async fn test_mobile_optimizations_integration() {
    // Test 1: Network Status
    let network_status = jsp_transport::network_status::NetworkStatus::new();
    
    network_status.set_network_type(NetworkType::Cellular).await;
    assert!(network_status.is_metered().await);
    
    network_status.set_network_type(NetworkType::Wifi).await;
    assert!(!network_status.is_metered().await);
    
    // Test 2: Heartbeat with AppState
    let heartbeat_config = jsp_transport::heartbeat::HeartbeatConfig {
        foreground_interval: Duration::from_secs(5),
        background_interval: Duration::from_secs(30),
        timeout_count: 3,
    };
    let heartbeat = jsp_transport::heartbeat::HeartbeatManager::new(heartbeat_config);
    
    // Default is Foreground (5s)
    assert_eq!(heartbeat.current_interval().await, Duration::from_secs(5));
    
    heartbeat.set_app_state(AppState::Background).await;
    assert_eq!(heartbeat.current_interval().await, Duration::from_secs(30));
    
    // Test 3: Adaptive Compression
    let mut adaptive = AdaptiveCompression::default_config();
    
    // Simulate high RTT
    adaptive.update_metrics(Duration::from_millis(300), 0.0);
    assert!(adaptive.get_level() < 5); // Should decrease from default
    
    // Simulate low RTT
    adaptive.update_metrics(Duration::from_millis(20), 0.0);
    // Level should increase or stay same
}
