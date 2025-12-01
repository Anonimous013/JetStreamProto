use crate::transport_selector::{TransportSelector, NetworkConditions, TransportType};
use jsp_core::crypto_selector::{CryptoSelector, CipherSuite};
use jsp_core::compression_selector::{CompressionSelector, CompressionAlgorithm};
use std::time::{Duration, Instant};

/// Performance metrics
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Average throughput (bytes/sec)
    pub throughput: u64,
    /// Average latency
    pub latency: Duration,
    /// Packet loss rate
    pub packet_loss: f32,
    /// CPU usage (0.0 - 1.0)
    pub cpu_usage: f32,
    /// Last update time
    pub last_update: Instant,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            throughput: 0,
            latency: Duration::from_millis(50),
            packet_loss: 0.0,
            cpu_usage: 0.5,
            last_update: Instant::now(),
        }
    }

    /// Update metrics
    pub fn update(&mut self, throughput: u64, latency: Duration, packet_loss: f32, cpu_usage: f32) {
        self.throughput = throughput;
        self.latency = latency;
        self.packet_loss = packet_loss;
        self.cpu_usage = cpu_usage;
        self.last_update = Instant::now();
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Adaptive configuration
#[derive(Debug, Clone)]
pub struct AdaptiveConfig {
    pub transport: TransportType,
    pub cipher: CipherSuite,
    pub compression: CompressionAlgorithm,
}

/// Adaptive optimizer
#[derive(Debug)]
pub struct AdaptiveOptimizer {
    transport_selector: TransportSelector,
    crypto_selector: CryptoSelector,
    compression_selector: CompressionSelector,
    metrics: PerformanceMetrics,
    last_adaptation: Option<Instant>,
    min_adaptation_interval: Duration,
}

impl AdaptiveOptimizer {
    /// Create new adaptive optimizer
    pub fn new() -> Self {
        Self {
            transport_selector: TransportSelector::new(),
            crypto_selector: CryptoSelector::new(),
            compression_selector: CompressionSelector::default(),
            metrics: PerformanceMetrics::new(),
            last_adaptation: None,
            min_adaptation_interval: Duration::from_secs(10),
        }
    }

    /// Get current adaptive configuration
    pub fn get_config(&self) -> AdaptiveConfig {
        let conditions = self.get_network_conditions();
        
        AdaptiveConfig {
            transport: self.transport_selector.get_optimal_transport(&conditions),
            cipher: self.crypto_selector.select_cipher(),
            compression: CompressionAlgorithm::Zstd, // Default, actual selection per-packet
        }
    }

    /// Update performance metrics
    pub fn update_metrics(&mut self, throughput: u64, latency: Duration, packet_loss: f32, cpu_usage: f32) {
        self.metrics.update(throughput, latency, packet_loss, cpu_usage);
        
        // Update compression selector with new metrics
        self.compression_selector.update_bandwidth(throughput);
        self.compression_selector.update_cpu(1.0 - cpu_usage); // Available CPU
    }

    /// Check if should adapt
    pub fn should_adapt(&self) -> bool {
        if let Some(last) = self.last_adaptation {
            if last.elapsed() < self.min_adaptation_interval {
                return false;
            }
        }

        // Adapt if conditions changed significantly
        let conditions = self.get_network_conditions();
        self.transport_selector.should_switch(&conditions)
    }

    /// Perform adaptation
    pub fn adapt(&mut self) -> AdaptiveConfig {
        self.last_adaptation = Some(Instant::now());
        
        let conditions = self.get_network_conditions();
        let optimal_transport = self.transport_selector.get_optimal_transport(&conditions);
        
        // Update transport if needed
        if optimal_transport != self.transport_selector.current_transport() {
            self.transport_selector.set_current(optimal_transport);
        }

        self.get_config()
    }

    /// Get network conditions from metrics
    fn get_network_conditions(&self) -> NetworkConditions {
        NetworkConditions {
            packet_loss: self.metrics.packet_loss,
            rtt: self.metrics.latency,
            bandwidth: self.metrics.throughput,
            jitter: Duration::from_millis(5), // Simplified
            behind_nat: false, // Would be detected separately
        }
    }

    /// Select compression for data
    pub fn select_compression(&self, data: &[u8]) -> CompressionAlgorithm {
        self.compression_selector.select_algorithm(data)
    }

    /// Get current transport
    pub fn current_transport(&self) -> TransportType {
        self.transport_selector.current_transport()
    }

    /// Get current cipher
    pub fn current_cipher(&self) -> CipherSuite {
        self.crypto_selector.select_cipher()
    }

    /// Get metrics
    pub fn metrics(&self) -> &PerformanceMetrics {
        &self.metrics
    }

    /// Set minimum adaptation interval
    pub fn set_min_adaptation_interval(&mut self, interval: Duration) {
        self.min_adaptation_interval = interval;
    }
}

impl Default for AdaptiveOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimizer_creation() {
        let optimizer = AdaptiveOptimizer::new();
        assert_eq!(optimizer.current_transport(), TransportType::Udp);
    }

    #[test]
    fn test_get_config() {
        let optimizer = AdaptiveOptimizer::new();
        let config = optimizer.get_config();
        assert_eq!(config.transport, TransportType::Udp);
    }

    #[test]
    fn test_update_metrics() {
        let mut optimizer = AdaptiveOptimizer::new();
        optimizer.update_metrics(5_000_000, Duration::from_millis(100), 0.02, 0.3);
        
        assert_eq!(optimizer.metrics.throughput, 5_000_000);
        assert_eq!(optimizer.metrics.latency, Duration::from_millis(100));
        assert_eq!(optimizer.metrics.packet_loss, 0.02);
    }

    #[test]
    fn test_should_adapt() {
        let mut optimizer = AdaptiveOptimizer::new();
        optimizer.set_min_adaptation_interval(Duration::from_millis(100));
        
        // Initially should not adapt (no significant change)
        assert!(!optimizer.should_adapt());
        
        // Update with poor conditions
        optimizer.update_metrics(1_000_000, Duration::from_millis(500), 0.15, 0.8);
        
        // Should adapt due to high packet loss
        assert!(optimizer.should_adapt());
    }

    #[test]
    fn test_adapt() {
        let mut optimizer = AdaptiveOptimizer::new();
        
        // Set poor conditions
        optimizer.update_metrics(1_000_000, Duration::from_millis(500), 0.15, 0.8);
        
        let config = optimizer.adapt();
        
        // Should switch to TCP due to high packet loss
        assert_eq!(config.transport, TransportType::Tcp);
    }

    #[test]
    fn test_compression_selection() {
        let optimizer = AdaptiveOptimizer::new();
        
        let text_data = b"This is some text data that should be compressed. \
                          Adding more text to exceed minimum size for compression. \
                          This is a test of the compression selector integration.";
        
        let algorithm = optimizer.select_compression(text_data);
        assert!(algorithm != CompressionAlgorithm::None);
    }

    #[test]
    fn test_metrics_update() {
        let mut metrics = PerformanceMetrics::new();
        metrics.update(10_000_000, Duration::from_millis(20), 0.01, 0.4);
        
        assert_eq!(metrics.throughput, 10_000_000);
        assert_eq!(metrics.latency, Duration::from_millis(20));
        assert_eq!(metrics.packet_loss, 0.01);
        assert_eq!(metrics.cpu_usage, 0.4);
    }
}
