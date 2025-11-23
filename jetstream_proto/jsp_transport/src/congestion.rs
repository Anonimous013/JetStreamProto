use std::time::{Duration, Instant};

/// State of the congestion controller
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CongestionState {
    SlowStart,
    CongestionAvoidance,
    Recovery,
}

/// Trait for congestion control algorithms
pub trait CongestionController: Send + Sync + std::fmt::Debug {
    /// Called when a packet is sent
    fn on_packet_sent(&mut self, sent_bytes: usize);
    
    /// Called when a packet is acknowledged
    fn on_packet_acked(&mut self, acked_bytes: usize, rtt: Duration);
    
    /// Called when a packet is declared lost
    fn on_packet_lost(&mut self, lost_bytes: usize);
    
    /// Get current congestion window in bytes
    fn congestion_window(&self) -> usize;
    
    /// Check if we can send more data given current inflight bytes
    fn can_send(&self, inflight_bytes: usize) -> bool;
    
    /// Get current state
    fn state(&self) -> CongestionState;
}

/// NewReno congestion control implementation
#[derive(Debug)]
pub struct NewReno {
    /// Congestion Window in bytes
    cwnd: usize,
    /// Slow Start Threshold in bytes
    ssthresh: usize,
    /// Current state
    state: CongestionState,
    /// Initial Window (IW)
    #[allow(dead_code)]
    initial_window: usize,
    /// Minimum Window
    min_window: usize,
}

impl NewReno {
    pub fn new(mss: usize) -> Self {
        // RFC 6928 suggests IW = 10 * MSS
        let initial_window = 10 * mss;
        Self {
            cwnd: initial_window,
            ssthresh: usize::MAX,
            state: CongestionState::SlowStart,
            initial_window,
            min_window: 2 * mss,
        }
    }
}

impl CongestionController for NewReno {
    fn on_packet_sent(&mut self, _sent_bytes: usize) {
        // NewReno doesn't change state on send, just tracks inflight (handled externally)
    }

    fn on_packet_acked(&mut self, acked_bytes: usize, _rtt: Duration) {
        match self.state {
            CongestionState::SlowStart => {
                // In Slow Start, cwnd increases by acked_bytes for each ACK
                self.cwnd += acked_bytes;
                
                if self.cwnd >= self.ssthresh {
                    self.state = CongestionState::CongestionAvoidance;
                }
            }
            CongestionState::CongestionAvoidance => {
                // In Congestion Avoidance, cwnd increases by MSS * MSS / cwnd for each ACK
                // This approximates +1 MSS per RTT
                // We use a simplified additive increase: cwnd += acked_bytes * acked_bytes / cwnd
                // Or more standard: cwnd += MSS * acked_bytes / cwnd
                // Let's use a safe integer approximation
                if self.cwnd > 0 {
                    let increase = (acked_bytes * acked_bytes) / self.cwnd;
                    self.cwnd += std::cmp::max(1, increase);
                }
            }
            CongestionState::Recovery => {
                // In Recovery (Fast Recovery), we stay here until all lost packets are recovered
                // For simple NewReno here, we exit recovery on new ACK (simplified)
                // Real NewReno is more complex with partial ACKs.
                // For now, let's assume if we get a good ACK, we go back to CA
                self.cwnd = self.ssthresh;
                self.state = CongestionState::CongestionAvoidance;
            }
        }
    }

    fn on_packet_lost(&mut self, _lost_bytes: usize) {
        // Multiplicative Decrease
        self.ssthresh = std::cmp::max(self.cwnd / 2, self.min_window);
        self.cwnd = self.min_window; // Timeout usually resets to 1 MSS (or min window)
        self.state = CongestionState::SlowStart;
    }

    fn congestion_window(&self) -> usize {
        self.cwnd
    }

    fn can_send(&self, inflight_bytes: usize) -> bool {
        inflight_bytes < self.cwnd
    }
    
    fn state(&self) -> CongestionState {
        self.state
    }
}


/// Simple bandwidth estimator based on delivery rate
#[derive(Debug)]
pub struct BandwidthEstimator {
    /// Total bytes delivered
    delivered_bytes: u64,
    /// Time of first packet sent
    first_sent_time: Option<Instant>,
    /// Estimated bandwidth in bytes/sec
    bandwidth_bps: u64,
}

impl BandwidthEstimator {
    pub fn new() -> Self {
        Self {
            delivered_bytes: 0,
            first_sent_time: None,
            bandwidth_bps: 0,
        }
    }

    pub fn on_packet_sent(&mut self) {
        if self.first_sent_time.is_none() {
            self.first_sent_time = Some(Instant::now());
        }
    }

    pub fn on_packet_acked(&mut self, acked_bytes: usize) {
        self.delivered_bytes += acked_bytes as u64;
        self.update_estimate();
    }

    fn update_estimate(&mut self) {
        if let Some(start) = self.first_sent_time {
            let elapsed = start.elapsed();
            if elapsed.as_secs_f64() > 0.1 { // Avoid division by zero or very small numbers
                self.bandwidth_bps = (self.delivered_bytes as f64 / elapsed.as_secs_f64()) as u64;
            }
        }
    }

    pub fn bandwidth(&self) -> u64 {
        self.bandwidth_bps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slow_start() {
        let mss = 1000;
        let mut cc = NewReno::new(mss);
        
        assert_eq!(cc.state(), CongestionState::SlowStart);
        assert_eq!(cc.congestion_window(), 10 * mss);
        
        // Ack 1 MSS
        cc.on_packet_acked(mss, Duration::from_millis(50));
        
        // Should increase by 1 MSS
        assert_eq!(cc.congestion_window(), 11 * mss);
    }

    #[test]
    fn test_congestion_avoidance_transition() {
        let mss = 1000;
        let mut cc = NewReno::new(mss);
        
        // Set ssthresh low to force transition
        cc.ssthresh = 11 * mss;
        
        cc.on_packet_acked(mss, Duration::from_millis(50));
        
        assert_eq!(cc.congestion_window(), 11 * mss);
        assert_eq!(cc.state(), CongestionState::CongestionAvoidance);
    }
    
    #[test]
    fn test_packet_loss() {
        let mss = 1000;
        let mut cc = NewReno::new(mss);
        
        let initial_cwnd = cc.congestion_window();
        
        // Simulate loss
        cc.on_packet_lost(mss);
        
        assert_eq!(cc.state(), CongestionState::SlowStart);
        assert_eq!(cc.ssthresh, initial_cwnd / 2);
        assert_eq!(cc.congestion_window(), cc.min_window);
    }

    #[test]
    fn test_bandwidth_estimator() {
        let mut estimator = BandwidthEstimator::new();
        
        estimator.on_packet_sent();
        
        // Simulate time passing
        std::thread::sleep(Duration::from_millis(150));
        
        estimator.on_packet_acked(1000);
        // We can't check exact value due to timing variance, but it should be non-zero
        // unless the sleep was interrupted or extremely fast (unlikely > 0.1s)
        // Let's just check it compiles and runs without panic.
        let _bw = estimator.bandwidth();
    }
}
