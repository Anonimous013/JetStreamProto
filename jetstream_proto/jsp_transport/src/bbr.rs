use std::time::{Duration, Instant};
use std::cmp;

/// BBRv2 congestion control state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BbrState {
    /// Initial slow start to find bandwidth
    Startup,
    /// Drain excess queue after startup
    Drain,
    /// Probe for more bandwidth
    ProbeBW,
    /// Probe for minimum RTT
    ProbeRTT,
}

/// BBRv2 congestion control implementation
/// Based on Google's BBRv2 algorithm for better performance than loss-based CC
#[derive(Debug)]
pub struct BbrCongestionControl {
    /// Current state
    state: BbrState,
    
    /// Estimated bottleneck bandwidth (bytes/sec)
    btlbw: u64,
    
    /// Minimum RTT observed
    min_rtt: Duration,
    
    /// Time when min_rtt was last updated
    min_rtt_timestamp: Instant,
    
    /// Pacing rate (bytes/sec)
    pacing_rate: u64,
    
    /// Congestion window (bytes)
    cwnd: usize,
    
    /// Maximum segment size
    mss: usize,
    
    /// Delivery rate estimator
    delivered_bytes: u64,
    delivered_time: Instant,
    
    /// Round trip counter
    round_count: u64,
    round_start: bool,
    
    /// Startup parameters
    #[allow(dead_code)]
    startup_gain: f64,
    drain_gain: f64,
    
    /// ProbeBW cycle
    probe_bw_cycle_idx: usize,
    probe_bw_cycle_start: Instant,
    
    /// ProbeRTT parameters
    probe_rtt_min_duration: Duration,
    probe_rtt_start: Option<Instant>,
    
    /// Pacing gain for current state
    pacing_gain: f64,
    
    /// CWND gain for current state
    cwnd_gain: f64,
    
    /// Full pipe detection
    full_pipe_count: u32,
    filled_pipe: bool,
}

impl BbrCongestionControl {
    /// Create new BBRv2 congestion controller
    pub fn new(mss: usize) -> Self {
        let now = Instant::now();
        Self {
            state: BbrState::Startup,
            btlbw: 0,
            min_rtt: Duration::from_millis(10), // Initial estimate
            min_rtt_timestamp: now,
            pacing_rate: 0,
            cwnd: 10 * mss, // Initial window
            mss,
            delivered_bytes: 0,
            delivered_time: now,
            round_count: 0,
            round_start: false,
            startup_gain: 2.77, // BBRv2 startup gain
            drain_gain: 1.0 / 2.77,
            probe_bw_cycle_idx: 0,
            probe_bw_cycle_start: now,
            probe_rtt_min_duration: Duration::from_millis(200),
            probe_rtt_start: None,
            pacing_gain: 2.77, // Start with startup gain
            cwnd_gain: 2.77,
            full_pipe_count: 0,
            filled_pipe: false,
        }
    }
    
    /// Update on packet acknowledgment
    pub fn on_ack(&mut self, acked_bytes: usize, rtt: Duration, now: Instant) {
        // Update delivery rate
        self.delivered_bytes += acked_bytes as u64;
        let elapsed = now.duration_since(self.delivered_time);
        
        if elapsed > Duration::from_millis(10) {
            let delivery_rate = (self.delivered_bytes as f64 / elapsed.as_secs_f64()) as u64;
            
            // Update bottleneck bandwidth estimate
            if delivery_rate > self.btlbw {
                self.btlbw = delivery_rate;
            }
            
            self.delivered_time = now;
            self.delivered_bytes = 0;
        }
        
        // Update min RTT
        if rtt < self.min_rtt {
            self.min_rtt = rtt;
            self.min_rtt_timestamp = now;
        }
        
        // Check if we need to probe RTT
        if now.duration_since(self.min_rtt_timestamp) > Duration::from_secs(10) {
            self.enter_probe_rtt(now);
        }
        
        // Update state machine
        self.update_state(now);
        
        // Update pacing rate and cwnd
        self.update_pacing_rate();
        self.update_cwnd();
        
        // Simple round counting approximation
        if rtt > Duration::from_micros(1) {
             self.round_start = true; // Simplified
        }
    }
    
    /// Update on packet loss
    pub fn on_loss(&mut self, _lost_bytes: usize) {
        // BBR is not loss-based, but we can use loss as a signal
        // In BBRv2, we might reduce pacing slightly on persistent loss
    }
    
    /// Enter ProbeRTT state
    fn enter_probe_rtt(&mut self, now: Instant) {
        self.state = BbrState::ProbeRTT;
        self.probe_rtt_start = Some(now);
        self.pacing_gain = 1.0;
        self.cwnd_gain = 1.0;
        // Reduce cwnd to 4 MSS to drain queue
        self.cwnd = 4 * self.mss;
    }
    
    /// Update BBR state machine
    fn update_state(&mut self, now: Instant) {
        match self.state {
            BbrState::Startup => {
                // Check if pipe is full (bandwidth not growing)
                if self.check_full_pipe() {
                    self.state = BbrState::Drain;
                    self.pacing_gain = self.drain_gain;
                    self.cwnd_gain = self.drain_gain;
                }
            }
            
            BbrState::Drain => {
                // Exit drain when inflight <= BDP
                let bdp = self.btlbw * self.min_rtt.as_secs_f64() as u64;
                if self.cwnd as u64 <= bdp {
                    self.state = BbrState::ProbeBW;
                    self.probe_bw_cycle_idx = 0;
                    self.probe_bw_cycle_start = now;
                    self.update_probe_bw_gain();
                }
            }
            
            BbrState::ProbeBW => {
                // Cycle through pacing gains
                if now.duration_since(self.probe_bw_cycle_start) > self.min_rtt {
                    self.probe_bw_cycle_idx = (self.probe_bw_cycle_idx + 1) % 8;
                    self.probe_bw_cycle_start = now;
                    self.update_probe_bw_gain();
                    
                    // Track rounds for potential mode switching
                    self.round_count += 1;
                }
            }
            
            BbrState::ProbeRTT => {
                if let Some(start) = self.probe_rtt_start {
                    if now.duration_since(start) > self.probe_rtt_min_duration {
                        self.state = BbrState::ProbeBW;
                        self.probe_rtt_start = None;
                        self.probe_bw_cycle_idx = 0;
                        self.update_probe_bw_gain();
                    }
                }
            }
        }
    }

    /// Check if pipe is full (bandwidth plateaued)
    fn check_full_pipe(&mut self) -> bool {
        // Simplified check: if we've been in startup for a while and bandwidth isn't growing fast enough
        // In a real implementation, we'd track max bandwidth over a window
        if self.filled_pipe {
            return true;
        }
        
        // For simulation/simplification, we assume pipe fills after some rounds
        // This logic would normally check if btlbw increased by < 25% over the last round
        if self.btlbw > 0 && self.round_count > 3 {
            self.full_pipe_count += 1;
            if self.full_pipe_count >= 3 {
                self.filled_pipe = true;
                return true;
            }
        }
        
        false
    }
    
    /// Update ProbeBW pacing gain based on cycle
    fn update_probe_bw_gain(&mut self) {
        // BBRv2 ProbeBW cycle: [1.25, 0.75, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]
        let gains = [1.25, 0.75, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        self.pacing_gain = gains[self.probe_bw_cycle_idx];
        self.cwnd_gain = 2.0; // Higher cwnd gain in ProbeBW
    }
    
    /// Update pacing rate
    fn update_pacing_rate(&mut self) {
        if self.btlbw > 0 {
            self.pacing_rate = (self.btlbw as f64 * self.pacing_gain) as u64;
        } else {
            // Initial pacing rate
            self.pacing_rate = (self.cwnd as f64 / self.min_rtt.as_secs_f64()) as u64;
        }
    }
    
    /// Update congestion window
    fn update_cwnd(&mut self) {
        let bdp = if self.btlbw > 0 {
            (self.btlbw as f64 * self.min_rtt.as_secs_f64() * self.cwnd_gain) as usize
        } else {
            self.cwnd
        };
        
        // Ensure minimum cwnd
        self.cwnd = cmp::max(bdp, 4 * self.mss);
    }
    
    /// Get current congestion window
    pub fn congestion_window(&self) -> usize {
        self.cwnd
    }
    
    /// Get current pacing rate (bytes/sec)
    pub fn pacing_rate(&self) -> u64 {
        self.pacing_rate
    }
    
    /// Get current state
    pub fn state(&self) -> BbrState {
        self.state
    }
    
    /// Check if we can send more data
    pub fn can_send(&self, inflight_bytes: usize) -> bool {
        inflight_bytes < self.cwnd
    }
}

use crate::congestion::{CongestionController, CongestionState};

impl CongestionController for BbrCongestionControl {
    fn on_packet_sent(&mut self, _sent_bytes: usize) {
        // BBR doesn't track inflight internally in this implementation
    }

    fn on_packet_acked(&mut self, acked_bytes: usize, rtt: Duration) {
        self.on_ack(acked_bytes, rtt, Instant::now());
    }

    fn on_packet_lost(&mut self, lost_bytes: usize) {
        self.on_loss(lost_bytes);
    }

    fn congestion_window(&self) -> usize {
        self.cwnd
    }

    fn can_send(&self, inflight_bytes: usize) -> bool {
        self.can_send(inflight_bytes)
    }

    fn state(&self) -> CongestionState {
        match self.state {
            BbrState::Startup => CongestionState::Startup,
            BbrState::Drain => CongestionState::Drain,
            BbrState::ProbeBW => CongestionState::ProbeBW,
            BbrState::ProbeRTT => CongestionState::ProbeRTT,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bbr_initialization() {
        let bbr = BbrCongestionControl::new(1000);
        assert_eq!(bbr.state(), BbrState::Startup);
        assert_eq!(bbr.congestion_window(), 10000);
    }

    #[test]
    fn test_bbr_startup_to_drain() {
        let mut bbr = BbrCongestionControl::new(1000);
        let now = Instant::now();
        
        // Simulate bandwidth growth stopping
        bbr.filled_pipe = true;
        bbr.update_state(now);
        
        assert_eq!(bbr.state(), BbrState::Drain);
    }

    #[test]
    fn test_bbr_ack_processing() {
        let mut bbr = BbrCongestionControl::new(1000);
        let now = Instant::now();
        
        bbr.on_ack(1000, Duration::from_millis(50), now);
        
        // Should have updated delivery rate
        assert!(bbr.pacing_rate() > 0);
    }
}
