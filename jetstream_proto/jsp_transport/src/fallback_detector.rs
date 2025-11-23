use std::time::{Duration, Instant};

/// Configuration for fallback detection
#[derive(Debug, Clone)]
pub struct FallbackConfig {
    /// Number of consecutive UDP failures before fallback
    pub failure_threshold: u32,
    
    /// Timeout for UDP response before considering it failed
    pub udp_timeout: Duration,
    
    /// Enable automatic TCP fallback
    pub enable_fallback: bool,
    
    /// Time window for failure counting (reset after this duration of success)
    pub failure_window: Duration,
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 3,
            udp_timeout: Duration::from_secs(5),
            enable_fallback: true,
            failure_window: Duration::from_secs(30),
        }
    }
}

/// Detector for UDP failures to trigger TCP fallback
pub struct FallbackDetector {
    udp_failures: u32,
    last_udp_success: Option<Instant>,
    last_udp_attempt: Option<Instant>,
    config: FallbackConfig,
}

impl FallbackDetector {
    /// Create new fallback detector with configuration
    pub fn new(config: FallbackConfig) -> Self {
        Self {
            udp_failures: 0,
            last_udp_success: None,
            last_udp_attempt: None,
            config,
        }
    }
    
    /// Create detector with default configuration
    pub fn default_config() -> Self {
        Self::new(FallbackConfig::default())
    }
    
    /// Record UDP send attempt
    pub fn record_udp_attempt(&mut self) {
        self.last_udp_attempt = Some(Instant::now());
        
        tracing::trace!(
            failures = self.udp_failures,
            "UDP attempt recorded"
        );
    }
    
    /// Record UDP success (received response)
    pub fn record_udp_success(&mut self) {
        let now = Instant::now();
        self.udp_failures = 0;
        self.last_udp_success = Some(now);
        
        tracing::debug!(
            "UDP success - failures reset"
        );
    }
    
    /// Record UDP failure (timeout, ICMP unreachable, etc.)
    pub fn record_udp_failure(&mut self) {
        self.udp_failures += 1;
        
        tracing::warn!(
            failures = self.udp_failures,
            threshold = self.config.failure_threshold,
            "UDP failure recorded"
        );
        
        if self.should_fallback() {
            tracing::error!(
                failures = self.udp_failures,
                "UDP failure threshold reached - fallback recommended"
            );
        }
    }
    
    /// Check if should fallback to TCP
    pub fn should_fallback(&self) -> bool {
        if !self.config.enable_fallback {
            return false;
        }
        
        // Primary check: exceeded failure threshold
        self.udp_failures >= self.config.failure_threshold
    }
    
    /// Check if UDP attempt has timed out
    pub fn is_udp_timeout(&self) -> bool {
        if let Some(last_attempt) = self.last_udp_attempt {
            last_attempt.elapsed() > self.config.udp_timeout
        } else {
            false
        }
    }
    
    /// Reset detector (e.g., after successful TCP connection or manual reset)
    pub fn reset(&mut self) {
        self.udp_failures = 0;
        self.last_udp_success = None;
        self.last_udp_attempt = None;
        
        tracing::info!("Fallback detector reset");
    }
    
    /// Get current failure count
    pub fn failure_count(&self) -> u32 {
        self.udp_failures
    }
    
    /// Get time since last UDP success
    pub fn time_since_last_success(&self) -> Option<Duration> {
        self.last_udp_success.map(|t| t.elapsed())
    }
    
    /// Get configuration
    pub fn config(&self) -> &FallbackConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    
    #[test]
    fn test_fallback_threshold() {
        let mut detector = FallbackDetector::default_config();
        
        // Should not fallback initially
        assert!(!detector.should_fallback());
        
        // Record failures
        detector.record_udp_failure();
        assert!(!detector.should_fallback()); // 1 failure
        
        detector.record_udp_failure();
        assert!(!detector.should_fallback()); // 2 failures
        
        detector.record_udp_failure();
        assert!(detector.should_fallback()); // 3 failures - threshold reached
    }
    
    #[test]
    fn test_success_resets_failures() {
        let mut detector = FallbackDetector::default_config();
        
        // Record failures
        detector.record_udp_failure();
        detector.record_udp_failure();
        assert_eq!(detector.failure_count(), 2);
        
        // Success resets
        detector.record_udp_success();
        assert_eq!(detector.failure_count(), 0);
        assert!(!detector.should_fallback());
    }
    
    #[test]
    fn test_timeout_detection() {
        let config = FallbackConfig {
            udp_timeout: Duration::from_millis(100),
            ..Default::default()
        };
        let mut detector = FallbackDetector::new(config);
        
        // No timeout initially
        assert!(!detector.is_udp_timeout());
        
        // Record attempt
        detector.record_udp_attempt();
        assert!(!detector.is_udp_timeout());
        
        // Wait for timeout
        sleep(Duration::from_millis(150));
        assert!(detector.is_udp_timeout());
    }
    
    #[test]
    fn test_disabled_fallback() {
        let config = FallbackConfig {
            enable_fallback: false,
            ..Default::default()
        };
        let mut detector = FallbackDetector::new(config);
        
        // Even with failures, should not fallback if disabled
        detector.record_udp_failure();
        detector.record_udp_failure();
        detector.record_udp_failure();
        
        assert!(!detector.should_fallback());
    }
    
    #[test]
    fn test_reset() {
        let mut detector = FallbackDetector::default_config();
        
        detector.record_udp_failure();
        detector.record_udp_failure();
        detector.record_udp_success();
        
        assert_eq!(detector.failure_count(), 0);
        assert!(detector.time_since_last_success().is_some());
        
        detector.reset();
        
        assert_eq!(detector.failure_count(), 0);
        assert!(detector.time_since_last_success().is_none());
    }
}
