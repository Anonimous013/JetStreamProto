//! Metrics - Per-Hop Performance Metrics
//! 
//! Tracks latency, throughput, and packet statistics for each hop.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Metrics for a single hop
pub struct HopMetrics {
    /// Total bytes sent
    bytes_sent: AtomicU64,
    
    /// Total bytes received
    bytes_received: AtomicU64,
    
    /// Total packets sent
    packets_sent: AtomicU64,
    
    /// Total packets received
    packets_received: AtomicU64,
    
    /// Average latency tracking
    latency_tracker: Mutex<LatencyTracker>,
    
    /// Start time for throughput calculation
    start_time: Instant,
}

impl HopMetrics {
    pub fn new() -> Self {
        Self {
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            packets_sent: AtomicU64::new(0),
            packets_received: AtomicU64::new(0),
            latency_tracker: Mutex::new(LatencyTracker::new()),
            start_time: Instant::now(),
        }
    }
    
    /// Record a sent packet
    pub fn record_sent(&self, bytes: usize, latency: Duration) {
        self.bytes_sent.fetch_add(bytes as u64, Ordering::Relaxed);
        self.packets_sent.fetch_add(1, Ordering::Relaxed);
        
        let mut tracker = self.latency_tracker.lock().unwrap();
        tracker.record(latency.as_secs_f64() * 1000.0); // Convert to ms
    }
    
    /// Record a received packet
    pub fn record_received(&self, bytes: usize, latency: Duration) {
        self.bytes_received.fetch_add(bytes as u64, Ordering::Relaxed);
        self.packets_received.fetch_add(1, Ordering::Relaxed);
        
        let mut tracker = self.latency_tracker.lock().unwrap();
        tracker.record(latency.as_secs_f64() * 1000.0);
    }
    
    /// Get total bytes sent
    pub fn bytes_sent(&self) -> u64 {
        self.bytes_sent.load(Ordering::Relaxed)
    }
    
    /// Get total bytes received
    pub fn bytes_received(&self) -> u64 {
        self.bytes_received.load(Ordering::Relaxed)
    }
    
    /// Get total packets sent
    pub fn packets_sent(&self) -> u64 {
        self.packets_sent.load(Ordering::Relaxed)
    }
    
    /// Get total packets received
    pub fn packets_received(&self) -> u64 {
        self.packets_received.load(Ordering::Relaxed)
    }
    
    /// Get average latency in milliseconds
    pub fn avg_latency_ms(&self) -> f64 {
        self.latency_tracker.lock().unwrap().average()
    }
    
    /// Get p50 latency in milliseconds
    pub fn p50_latency_ms(&self) -> f64 {
        self.latency_tracker.lock().unwrap().percentile(50.0)
    }
    
    /// Get p99 latency in milliseconds
    pub fn p99_latency_ms(&self) -> f64 {
        self.latency_tracker.lock().unwrap().percentile(99.0)
    }
    
    /// Get throughput in bytes per second
    pub fn throughput_bps(&self) -> u64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            let total_bytes = self.bytes_sent() + self.bytes_received();
            (total_bytes as f64 / elapsed) as u64
        } else {
            0
        }
    }
    
    /// Reset all metrics
    pub fn reset(&self) {
        self.bytes_sent.store(0, Ordering::Relaxed);
        self.bytes_received.store(0, Ordering::Relaxed);
        self.packets_sent.store(0, Ordering::Relaxed);
        self.packets_received.store(0, Ordering::Relaxed);
        self.latency_tracker.lock().unwrap().reset();
    }
}

impl Default for HopMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Latency tracker using exponential moving average and histogram
struct LatencyTracker {
    /// Exponential moving average
    ema: f64,
    
    /// Smoothing factor for EMA
    alpha: f64,
    
    /// Sample count
    count: u64,
    
    /// Recent samples for percentile calculation (circular buffer)
    samples: Vec<f64>,
    
    /// Current position in circular buffer
    pos: usize,
}

impl LatencyTracker {
    fn new() -> Self {
        Self {
            ema: 0.0,
            alpha: 0.2,
            count: 0,
            samples: Vec::with_capacity(1000),
            pos: 0,
        }
    }
    
    fn record(&mut self, latency_ms: f64) {
        self.count += 1;
        
        // Update EMA
        if self.ema == 0.0 {
            self.ema = latency_ms;
        } else {
            self.ema = self.alpha * latency_ms + (1.0 - self.alpha) * self.ema;
        }
        
        // Store sample for percentile calculation
        if self.samples.len() < 1000 {
            self.samples.push(latency_ms);
        } else {
            self.samples[self.pos] = latency_ms;
            self.pos = (self.pos + 1) % 1000;
        }
    }
    
    fn average(&self) -> f64 {
        self.ema
    }
    
    fn percentile(&self, p: f64) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        
        let mut sorted = self.samples.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let index = ((p / 100.0) * sorted.len() as f64) as usize;
        sorted[index.min(sorted.len() - 1)]
    }
    
    fn reset(&mut self) {
        self.ema = 0.0;
        self.count = 0;
        self.samples.clear();
        self.pos = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hop_metrics() {
        let metrics = HopMetrics::new();
        
        metrics.record_sent(1000, Duration::from_millis(10));
        assert_eq!(metrics.bytes_sent(), 1000);
        assert_eq!(metrics.packets_sent(), 1);
        assert!(metrics.avg_latency_ms() > 0.0);
        
        metrics.record_received(500, Duration::from_millis(15));
        assert_eq!(metrics.bytes_received(), 500);
        assert_eq!(metrics.packets_received(), 1);
    }
    
    #[test]
    fn test_latency_tracker() {
        let mut tracker = LatencyTracker::new();
        
        tracker.record(10.0);
        assert_eq!(tracker.average(), 10.0);
        
        tracker.record(20.0);
        assert!(tracker.average() > 10.0 && tracker.average() < 20.0);
        
        // Record many samples
        for i in 0..100 {
            tracker.record(i as f64);
        }
        
        let p50 = tracker.percentile(50.0);
        let p99 = tracker.percentile(99.0);
        assert!(p50 < p99);
    }
}
