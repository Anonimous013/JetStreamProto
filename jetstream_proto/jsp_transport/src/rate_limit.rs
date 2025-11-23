use std::time::Instant;
use std::sync::{Arc, Mutex};

/// Token bucket rate limiter for per-connection rate limiting
#[derive(Debug)]
pub struct RateLimiter {
    /// Maximum tokens (burst capacity)
    capacity: u32,
    /// Current token count
    tokens: f64,
    /// Tokens added per second
    refill_rate: f64,
    /// Last refill timestamp
    last_refill: Instant,
    /// Maximum bytes per second
    bytes_capacity: u64,
    /// Current byte tokens
    byte_tokens: f64,
    /// Bytes added per second
    byte_refill_rate: f64,
}

impl RateLimiter {
    pub fn new(messages_per_second: u32, bytes_per_second: u64) -> Self {
        Self {
            capacity: messages_per_second,
            tokens: messages_per_second as f64,
            refill_rate: messages_per_second as f64,
            last_refill: Instant::now(),
            bytes_capacity: bytes_per_second,
            byte_tokens: bytes_per_second as f64,
            byte_refill_rate: bytes_per_second as f64,
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        
        // Refill message tokens
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity as f64);
        
        // Refill byte tokens
        self.byte_tokens = (self.byte_tokens + elapsed * self.byte_refill_rate)
            .min(self.bytes_capacity as f64);
        
        self.last_refill = now;
    }

    /// Check if a message of given size can be sent, and consume tokens if so
    pub fn check_and_consume(&mut self, message_size: usize) -> bool {
        self.refill();
        
        // Check both message count and byte limits
        if self.tokens >= 1.0 && self.byte_tokens >= message_size as f64 {
            self.tokens -= 1.0;
            self.byte_tokens -= message_size as f64;
            true
        } else {
            false
        }
    }

    /// Get current available tokens
    pub fn available_tokens(&mut self) -> u32 {
        self.refill();
        self.tokens as u32
    }

    /// Get current available byte tokens
    pub fn available_bytes(&mut self) -> u64 {
        self.refill();
        self.byte_tokens as u64
    }
}

/// Global rate limiter for server-wide limits
#[derive(Debug, Clone)]
pub struct GlobalRateLimiter {
    inner: Arc<Mutex<RateLimiter>>,
}

impl GlobalRateLimiter {
    pub fn new(messages_per_second: u32, bytes_per_second: u64) -> Self {
        Self {
            inner: Arc::new(Mutex::new(RateLimiter::new(messages_per_second, bytes_per_second))),
        }
    }

    pub fn check_and_consume(&self, message_size: usize) -> bool {
        self.inner.lock().unwrap().check_and_consume(message_size)
    }

    pub fn available_tokens(&self) -> u32 {
        self.inner.lock().unwrap().available_tokens()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_rate_limiter_basic() {
        let mut limiter = RateLimiter::new(10, 1000);
        
        // Should allow first message
        assert!(limiter.check_and_consume(100));
        
        // Should allow up to capacity
        for _ in 0..9 {
            assert!(limiter.check_and_consume(100));
        }
        
        // Should deny when tokens exhausted
        assert!(!limiter.check_and_consume(100));
    }

    #[test]
    fn test_rate_limiter_refill() {
        let mut limiter = RateLimiter::new(10, 10000);
        
        // Consume all tokens
        for _ in 0..10 {
            limiter.check_and_consume(100);
        }
        
        // Should be denied
        assert!(!limiter.check_and_consume(100));
        
        // Wait for refill
        thread::sleep(Duration::from_millis(200));
        
        // Should have refilled some tokens (10 tokens/sec = 2 tokens in 200ms)
        assert!(limiter.check_and_consume(100));
    }

    #[test]
    fn test_rate_limiter_bytes() {
        let mut limiter = RateLimiter::new(100, 1000);
        
        // Large message should be denied if exceeds byte limit
        assert!(!limiter.check_and_consume(2000));
        
        // Small messages should work
        assert!(limiter.check_and_consume(100));
    }

    #[test]
    fn test_global_rate_limiter() {
        let limiter = GlobalRateLimiter::new(10, 1000);
        
        assert!(limiter.check_and_consume(100));
        assert_eq!(limiter.available_tokens(), 9);
    }
}
