use std::time::Duration;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

/// Circuit Breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Normal operation, requests allowed
    Closed,
    /// Failure threshold exceeded, requests blocked
    Open,
    /// Trial period, limited requests allowed
    HalfOpen,
}

/// Circuit Breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening the circuit
    pub failure_threshold: u32,
    /// Duration to wait before switching from Open to HalfOpen
    pub reset_timeout: Duration,
    /// Number of successful requests in HalfOpen to switch back to Closed
    pub success_threshold: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            reset_timeout: Duration::from_secs(10),
            success_threshold: 2,
        }
    }
}

/// Circuit Breaker implementation
#[derive(Debug)]
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Arc<AtomicU32>, // 0: Closed, 1: Open, 2: HalfOpen
    failures: Arc<AtomicU32>,
    successes: Arc<AtomicU32>,
    last_failure_time: Arc<AtomicU64>, // Milliseconds since epoch
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: Arc::new(AtomicU32::new(0)),
            failures: Arc::new(AtomicU32::new(0)),
            successes: Arc::new(AtomicU32::new(0)),
            last_failure_time: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Check if a request is allowed
    pub fn allow_request(&self) -> bool {
        let state = self.state.load(Ordering::Relaxed);
        
        match state {
            0 => true, // Closed
            1 => { // Open
                // Check if reset timeout has passed
                let last_failure = self.last_failure_time.load(Ordering::Relaxed);
                let now = self.now_ms();
                
                if now.saturating_sub(last_failure) >= self.config.reset_timeout.as_millis() as u64 {
                    // Switch to HalfOpen
                    if self.state.compare_exchange(1, 2, Ordering::SeqCst, Ordering::Relaxed).is_ok() {
                        self.successes.store(0, Ordering::Relaxed);
                        return true;
                    }
                }
                false
            },
            2 => true, // HalfOpen (allow probing)
            _ => true,
        }
    }

    /// Record a successful request
    pub fn record_success(&self) {
        let state = self.state.load(Ordering::Relaxed);
        
        if state == 2 { // HalfOpen
            let successes = self.successes.fetch_add(1, Ordering::Relaxed) + 1;
            if successes >= self.config.success_threshold {
                // Reset to Closed
                self.state.store(0, Ordering::SeqCst);
                self.failures.store(0, Ordering::Relaxed);
                self.successes.store(0, Ordering::Relaxed);
            }
        } else if state == 0 { // Closed
            // Reset failure count on success in Closed state (optional, depends on strategy)
            self.failures.store(0, Ordering::Relaxed);
        }
    }

    /// Record a failed request
    pub fn record_failure(&self) {
        let state = self.state.load(Ordering::Relaxed);
        
        if state == 0 { // Closed
            let failures = self.failures.fetch_add(1, Ordering::Relaxed) + 1;
            if failures >= self.config.failure_threshold {
                // Trip to Open
                self.trip_to_open();
            }
        } else if state == 2 { // HalfOpen
            // Immediate trip back to Open on failure in HalfOpen
            self.trip_to_open();
        }
        
        self.last_failure_time.store(self.now_ms(), Ordering::Relaxed);
    }
    
    fn trip_to_open(&self) {
        self.state.store(1, Ordering::SeqCst);
    }
    
    fn now_ms(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
    
    pub fn state(&self) -> State {
        match self.state.load(Ordering::Relaxed) {
            0 => State::Closed,
            1 => State::Open,
            2 => State::HalfOpen,
            _ => State::Closed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_initial_state() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        assert_eq!(cb.state(), State::Closed);
        assert!(cb.allow_request());
    }

    #[test]
    fn test_trip_to_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            ..Default::default()
        };
        let cb = CircuitBreaker::new(config);

        cb.record_failure();
        assert_eq!(cb.state(), State::Closed);
        
        cb.record_failure();
        assert_eq!(cb.state(), State::Open);
        assert!(!cb.allow_request());
    }

    #[test]
    fn test_half_open_recovery() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            reset_timeout: Duration::from_millis(100),
            success_threshold: 2,
        };
        let cb = CircuitBreaker::new(config);

        // Trip to Open
        cb.record_failure();
        assert_eq!(cb.state(), State::Open);

        // Wait for timeout
        thread::sleep(Duration::from_millis(150));

        // Should switch to HalfOpen on next check
        assert!(cb.allow_request());
        assert_eq!(cb.state(), State::HalfOpen);

        // Success 1
        cb.record_success();
        assert_eq!(cb.state(), State::HalfOpen);

        // Success 2 -> Closed
        cb.record_success();
        assert_eq!(cb.state(), State::Closed);
    }
    
    #[test]
    fn test_half_open_failure() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            reset_timeout: Duration::from_millis(100),
            success_threshold: 2,
        };
        let cb = CircuitBreaker::new(config);

        // Trip to Open
        cb.record_failure();
        thread::sleep(Duration::from_millis(150));
        
        // Enter HalfOpen
        assert!(cb.allow_request());
        assert_eq!(cb.state(), State::HalfOpen);
        
        // Fail again -> Open
        cb.record_failure();
        assert_eq!(cb.state(), State::Open);
        assert!(!cb.allow_request());
    }
}
