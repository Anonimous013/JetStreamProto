use std::time::Duration;
use std::cmp::{max, min};

/// Configuration for adaptive compression
#[derive(Debug, Clone)]
pub struct AdaptiveCompressionConfig {
    /// Minimum compression level (e.g., 0 for no compression or low level)
    pub min_level: i32,
    /// Maximum compression level
    pub max_level: i32,
    /// RTT threshold above which to decrease compression (latency bottleneck)
    pub high_rtt_threshold: Duration,
    /// RTT threshold below which to increase compression (bandwidth optimization)
    pub low_rtt_threshold: Duration,
    /// Packet loss threshold above which to decrease compression
    pub packet_loss_threshold: f64,
}

impl Default for AdaptiveCompressionConfig {
    fn default() -> Self {
        Self {
            min_level: 0,
            max_level: 9, // Standard max for many algos
            high_rtt_threshold: Duration::from_millis(200),
            low_rtt_threshold: Duration::from_millis(50),
            packet_loss_threshold: 0.05, // 5%
        }
    }
}

/// Adaptive compression manager
pub struct AdaptiveCompression {
    current_level: i32,
    config: AdaptiveCompressionConfig,
    last_update: std::time::Instant,
    update_interval: Duration,
}

impl AdaptiveCompression {
    pub fn new(config: AdaptiveCompressionConfig) -> Self {
        Self {
            current_level: (config.min_level + config.max_level) / 2,
            config,
            last_update: std::time::Instant::now(),
            update_interval: Duration::from_secs(5),
        }
    }

    pub fn default_config() -> Self {
        Self::new(AdaptiveCompressionConfig::default())
    }

    /// Update compression level based on network metrics
    pub fn update_metrics(&mut self, rtt: Duration, packet_loss: f64) {
        if self.last_update.elapsed() < self.update_interval {
            return;
        }
        self.last_update = std::time::Instant::now();

        let old_level = self.current_level;

        // Logic:
        // 1. High packet loss -> Reduce compression (reduce CPU/latency impact of retransmissions)
        // 2. High RTT -> Reduce compression (latency bottleneck, CPU might add to it)
        // 3. Low RTT -> Increase compression (optimize bandwidth)
        
        if packet_loss > self.config.packet_loss_threshold {
            self.current_level = max(self.config.min_level, self.current_level - 2);
        } else if rtt > self.config.high_rtt_threshold {
            self.current_level = max(self.config.min_level, self.current_level - 1);
        } else if rtt < self.config.low_rtt_threshold {
            self.current_level = min(self.config.max_level, self.current_level + 1);
        }

        if self.current_level != old_level {
            tracing::debug!(
                old_level = old_level,
                new_level = self.current_level,
                rtt = ?rtt,
                loss = packet_loss,
                "Adaptive compression level updated"
            );
        }
    }

    /// Get current compression level
    pub fn get_level(&self) -> i32 {
        self.current_level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_logic() {
        let config = AdaptiveCompressionConfig {
            min_level: 1,
            max_level: 5,
            high_rtt_threshold: Duration::from_millis(100),
            low_rtt_threshold: Duration::from_millis(20),
            packet_loss_threshold: 0.1,
        };
        let mut adaptive = AdaptiveCompression::new(config);
        
        // Initial level should be mid-range (3)
        assert_eq!(adaptive.get_level(), 3);

        // Force update (bypass interval check for test)
        adaptive.update_interval = Duration::ZERO;

        // Case 1: High packet loss -> Decrease
        adaptive.update_metrics(Duration::from_millis(50), 0.15);
        assert_eq!(adaptive.get_level(), 1); // 3 - 2 = 1

        // Case 2: Low RTT -> Increase
        adaptive.update_metrics(Duration::from_millis(10), 0.0);
        assert_eq!(adaptive.get_level(), 2);

        // Case 3: High RTT -> Decrease
        adaptive.update_metrics(Duration::from_millis(150), 0.0);
        assert_eq!(adaptive.get_level(), 1);
    }
}
