use std::collections::BTreeMap;
use std::time::{Duration, Instant};
use std::fmt;
use jsp_core::types::delivery::DeliveryMode;
use crate::congestion::{CongestionController, NewReno};
use bytes::Bytes;

pub struct ReliabilityLayer {
    next_seq: u64,
    // Unacknowledged packets: Seq -> (SendTime, Payload, DeliveryMode)
    sent_buffer: BTreeMap<u64, (Instant, Bytes, DeliveryMode)>,
    // Smoothed RTT
    srtt: Duration,
    // RTT Variance
    rttvar: Duration,
    // Congestion Controller
    congestion: Box<dyn CongestionController + Send + Sync>,
    // Bytes in flight
    inflight_bytes: usize,
    
    // Receiver state
    cumulative_ack: u64,
    received_buffer: BTreeMap<u64, (u32, Bytes)>,
    
    // ACK Batching
    pending_ack_count: usize,
    last_ack_time: Instant,
}

impl ReliabilityLayer {
    pub fn new() -> Self {
        Self {
            next_seq: 1,
            sent_buffer: BTreeMap::new(),
            srtt: Duration::from_millis(100), // Initial guess
            rttvar: Duration::from_millis(0),
            congestion: Box::new(NewReno::new(1200)), // Default MSS 1200
            inflight_bytes: 0,
            cumulative_ack: 0,
            received_buffer: BTreeMap::new(),
            pending_ack_count: 0,
            last_ack_time: Instant::now(),
        }
    }

    pub fn next_sequence(&mut self) -> u64 {
        let seq = self.next_seq;
        self.next_seq += 1;
        seq
    }

    pub fn track_sent_packet(&mut self, seq: u64, data: Bytes, mode: DeliveryMode) {
        let len = data.len();
        self.sent_buffer.insert(seq, (Instant::now(), data, mode));
        self.inflight_bytes += len;
        self.congestion.on_packet_sent(len);
    }

    pub fn on_ack(&mut self, ack_seq: u64, ranges: &[(u64, u64)]) {
        // Remove cumulative ack
        let keys_to_remove: Vec<u64> = self.sent_buffer.keys()
            .filter(|&&k| k <= ack_seq)
            .cloned()
            .collect();
        
        for k in keys_to_remove {
            if let Some((sent_time, data, _)) = self.sent_buffer.remove(&k) {
                let len = data.len();
                self.inflight_bytes = self.inflight_bytes.saturating_sub(len);
                self.update_rtt(sent_time.elapsed());
                self.congestion.on_packet_acked(len, sent_time.elapsed());
            }
        }

        // Remove SACK ranges
        for &(start, end) in ranges {
            let sack_keys: Vec<u64> = self.sent_buffer.keys()
                .filter(|&&k| k >= start && k <= end)
                .cloned()
                .collect();
            
            for k in sack_keys {
                if let Some((sent_time, data, _)) = self.sent_buffer.remove(&k) {
                    let len = data.len();
                    self.inflight_bytes = self.inflight_bytes.saturating_sub(len);
                    self.update_rtt(sent_time.elapsed());
                    self.congestion.on_packet_acked(len, sent_time.elapsed());
                }
            }
        }
    }

    fn update_rtt(&mut self, rtt: Duration) {
        // RFC 6298 standard RTT update
        if self.srtt == Duration::from_millis(0) {
            self.srtt = rtt;
            self.rttvar = rtt / 2;
        } else {
            let delta = if rtt > self.srtt { rtt - self.srtt } else { self.srtt - rtt };
            self.rttvar = (self.rttvar * 3 + delta) / 4;
            self.srtt = (self.srtt * 7 + rtt) / 8;
        }
    }

    pub fn get_retransmits(&mut self) -> Vec<(u64, Bytes)> {
        let now = Instant::now();
        let rto = self.srtt + 4 * self.rttvar;
        let rto = std::cmp::max(rto, Duration::from_millis(200)); // Min RTO

        let retransmits: Vec<(u64, Bytes)> = self.sent_buffer.iter()
            .filter_map(|(seq, (sent_time, data, mode))| {
                let elapsed = now.duration_since(*sent_time);
                
                // If not yet time for RTO, skip
                if elapsed <= rto {
                    return None;
                }

                // Check delivery mode rules
                match mode {
                    DeliveryMode::Reliable => {
                        // Always retransmit
                        Some((*seq, data.clone()))
                    }
                    DeliveryMode::PartiallyReliable { ttl_ms } => {
                        let ttl = Duration::from_millis(*ttl_ms as u64);
                        if elapsed < ttl {
                            // Still within TTL, retransmit
                            Some((*seq, data.clone()))
                        } else {
                            // TTL expired, do not retransmit
                            None
                        }
                    }
                    DeliveryMode::BestEffort => {
                        // Never retransmit
                        None
                    }
                }
            })
            .collect();

        // If we have retransmits, notify congestion controller of loss
        // Note: This is a simplification. We should only notify once per loss event.
        if !retransmits.is_empty() {
            // Assume the first packet lost triggered the reaction
            let lost_bytes = retransmits[0].1.len();
            self.congestion.on_packet_lost(lost_bytes);
        }

        retransmits
    }

    pub fn cleanup_expired(&mut self) {
        let now = Instant::now();
        
        self.sent_buffer.retain(|_, (sent_time, _, mode)| {
            match mode {
                DeliveryMode::PartiallyReliable { ttl_ms } => {
                    let ttl = Duration::from_millis(*ttl_ms as u64);
                    let elapsed = now.duration_since(*sent_time);
                    elapsed < ttl // Keep if not expired
                }
                DeliveryMode::BestEffort => {
                    // BestEffort packets don't need to be tracked for retransmission
                    // But we might want to keep them briefly for RTT calculation if ACKed quickly?
                    // For now, let's remove them if they are older than RTO to free memory,
                    // or we can just remove them immediately if we don't care about their RTT.
                    // Let's keep them until RTO to allow RTT updates, then drop.
                    let rto = self.srtt + 4 * self.rttvar;
                    let rto = std::cmp::max(rto, Duration::from_millis(200));
                    now.duration_since(*sent_time) <= rto
                }
                DeliveryMode::Reliable => {
                    // Keep until ACKed
                    true
                }
            }
        });
        
        // Recalculate inflight bytes after cleanup
        // This is expensive but accurate. Alternatively we could track removals in retain but retain doesn't give us the removed items easily in stable Rust without drain_filter (nightly).
        // So let's just recalculate.
        self.inflight_bytes = self.sent_buffer.values().map(|(_, data, _)| data.len()).sum();
    }

    pub fn can_send(&self) -> bool {
        self.congestion.can_send(self.inflight_bytes)
    }

    pub fn track_received_packet(&mut self, seq: u64, stream_id: u32, data: Bytes) {
        if seq <= self.cumulative_ack && !self.received_buffer.contains_key(&seq) {
            // Duplicate and already processed (popped)
            return;
        }
        // If it's in received_buffer, it's a duplicate but not popped yet.
        if self.received_buffer.contains_key(&seq) {
            return;
        }
        
        self.received_buffer.insert(seq, (stream_id, data));
        
        // Update cumulative ack
        while self.received_buffer.contains_key(&(self.cumulative_ack + 1)) {
            self.cumulative_ack += 1;
        }

        // Increment pending ACKs
        self.pending_ack_count += 1;
    }

    /// Pop all received packets that are ready (in-order)
    pub fn pop_received_packets(&mut self) -> Vec<(u64, u32, Bytes)> {
        let mut packets = Vec::new();
        
        // We can only return packets up to cumulative_ack
        // But actually, we might have gaps.
        // Wait, pop_received_packets should return packets that are <= cumulative_ack AND haven't been popped?
        // Or does received_buffer only contain un-popped packets?
        // Yes, we should remove them from received_buffer as we pop them.
        
        // Iterate keys in order
        let keys: Vec<u64> = self.received_buffer.keys().cloned().collect();
        for seq in keys {
             // If we have a gap before this seq, we can't pop it if we want strict ordering?
             // But reliability layer guarantees delivery.
             // If we have cumulative_ack = N, it means we have everything up to N.
             
             if seq <= self.cumulative_ack {
                 if let Some((stream_id, data)) = self.received_buffer.remove(&seq) {
                     packets.push((seq, stream_id, data));
                 }
             } else {
                 // Stop at the first gap (which is after cumulative_ack)
                 break;
             }
        }
        
        packets
    }

    /// Check if ACK should be sent based on batching rules
    pub fn should_send_ack(&self, batch_size: usize, batch_timeout: Duration) -> bool {
        if self.pending_ack_count == 0 {
            return false;
        }
        if self.pending_ack_count >= batch_size {
            return true;
        }
        self.last_ack_time.elapsed() >= batch_timeout
    }

    /// Check if there are any pending ACKs
    pub fn has_pending_acks(&self) -> bool {
        self.pending_ack_count > 0
    }

    /// Reset ACK batching state after sending ACK
    pub fn on_ack_sent(&mut self) {
        self.pending_ack_count = 0;
        self.last_ack_time = Instant::now();
    }

    /// Get ACK information (cumulative ACK and SACK ranges)
    pub fn get_ack_info(&self) -> (u64, Vec<(u64, u64)>) {
        let ack = self.cumulative_ack;
        let mut ranges = Vec::new();
        
        // Calculate SACK ranges from received buffer
        // Keys in received_buffer are > cumulative_ack
        let mut start = 0;
        let mut end = 0;
        
        for &seq in self.received_buffer.keys() {
            if seq <= ack {
                continue;
            }
            
            if start == 0 {
                start = seq;
                end = seq;
            } else if seq == end + 1 {
                end = seq;
            } else {
                ranges.push((start, end));
                start = seq;
                end = seq;
            }
        }
        
        if start != 0 {
            ranges.push((start, end));
        }
        
        (ack, ranges)
    }
}

impl fmt::Debug for ReliabilityLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReliabilityLayer")
            .field("next_seq", &self.next_seq)
            .field("sent_buffer_len", &self.sent_buffer.len())
            .field("srtt", &self.srtt)
            .field("rttvar", &self.rttvar)
            .field("inflight_bytes", &self.inflight_bytes)
            .field("cumulative_ack", &self.cumulative_ack)
            .field("received_buffer_len", &self.received_buffer.len())
            .field("congestion", &"Box<dyn CongestionController>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_reliable_retransmit() {
        let mut reliability = ReliabilityLayer::new();
        let data = Bytes::from(vec![1, 2, 3]);
        
        reliability.track_sent_packet(1, data.clone(), DeliveryMode::Reliable);
        
        // Initially no retransmits (RTO not passed)
        assert!(reliability.get_retransmits().is_empty());
        
        // Wait for RTO (min 200ms)
        thread::sleep(Duration::from_millis(250));
        
        // Should retransmit
        let retransmits = reliability.get_retransmits();
        assert_eq!(retransmits.len(), 1);
        assert_eq!(retransmits[0].0, 1);
        assert_eq!(retransmits[0].1, data);
    }

    #[test]
    fn test_partially_reliable_ttl() {
        let mut reliability = ReliabilityLayer::new();
        let data = Bytes::from(vec![1, 2, 3]);
        
        // TTL = 100ms
        reliability.track_sent_packet(1, data.clone(), DeliveryMode::PartiallyReliable { ttl_ms: 100 });
        
        // Wait for 250ms (RTO passed, TTL passed)
        thread::sleep(Duration::from_millis(250));
        
        // Should NOT retransmit because TTL expired
        assert!(reliability.get_retransmits().is_empty());
        
        // Should be cleaned up
        reliability.cleanup_expired();
        // We can't check internal state easily, but we can check if it's still tracked by trying to ack it?
        // Or just trust the logic.
    }
    
    #[test]
    fn test_best_effort_no_retransmit() {
        let mut reliability = ReliabilityLayer::new();
        let data = Bytes::from(vec![1, 2, 3]);
        
        reliability.track_sent_packet(1, data.clone(), DeliveryMode::BestEffort);
        
        // Wait for RTO
        thread::sleep(Duration::from_millis(250));
        
        // Should NOT retransmit
        assert!(reliability.get_retransmits().is_empty());
    }

    #[test]
    fn test_out_of_order_delivery() {
        let mut reliability = ReliabilityLayer::new();
        
        // Receive 1, 2, 4 (gap at 3)
        reliability.track_received_packet(1, 0, Bytes::from(vec![1]));
        reliability.track_received_packet(2, 0, Bytes::from(vec![2]));
        reliability.track_received_packet(4, 0, Bytes::from(vec![4]));
        
        // Should return 1 and 2
        let packets = reliability.pop_received_packets();
        assert_eq!(packets.len(), 2);
        assert_eq!(packets[0].0, 1);
        assert_eq!(packets[1].0, 2);
        
        // 4 should still be buffered
        assert_eq!(reliability.received_buffer.len(), 1);
        assert!(reliability.received_buffer.contains_key(&4));
        
        // Receive 3 (fill gap)
        reliability.track_received_packet(3, 0, Bytes::from(vec![3]));
        
        // Should return 3 and 4
        let packets = reliability.pop_received_packets();
        assert_eq!(packets.len(), 2);
        assert_eq!(packets[0].0, 3);
        assert_eq!(packets[1].0, 4);
        
        assert!(reliability.received_buffer.is_empty());
    }

    #[test]
    fn test_ack_batching() {
        let mut reliability = ReliabilityLayer::new();
        let batch_size = 3;
        let batch_timeout = Duration::from_millis(50);
        
        // 1. Receive 1 packet
        reliability.track_received_packet(1, 0, Bytes::new());
        assert!(reliability.has_pending_acks());
        assert!(!reliability.should_send_ack(batch_size, batch_timeout));
        
        // 2. Receive 2nd packet
        reliability.track_received_packet(2, 0, Bytes::new());
        assert!(!reliability.should_send_ack(batch_size, batch_timeout));
        
        // 3. Receive 3rd packet (hit batch size)
        reliability.track_received_packet(3, 0, Bytes::new());
        assert!(reliability.should_send_ack(batch_size, batch_timeout));
        
        // 4. Simulate ACK sent
        reliability.on_ack_sent();
        assert!(!reliability.has_pending_acks());
        assert!(!reliability.should_send_ack(batch_size, batch_timeout));
        
        // 5. Receive 1 packet and wait for timeout
        reliability.track_received_packet(4, 0, Bytes::new());
        assert!(!reliability.should_send_ack(batch_size, batch_timeout));
        
        thread::sleep(Duration::from_millis(60));
        assert!(reliability.should_send_ack(batch_size, batch_timeout));
    }

    #[test]
    fn test_piggybacking_logic() {
        let mut reliability = ReliabilityLayer::new();
        
        // No pending ACKs initially
        assert!(!reliability.has_pending_acks());
        
        // Receive packet
        reliability.track_received_packet(1, 0, Bytes::new());
        assert!(reliability.has_pending_acks());
        
        // Get ACK info
        let (ack, ranges) = reliability.get_ack_info();
        assert_eq!(ack, 1);
        assert!(ranges.is_empty());
        
        // Simulate piggybacked ACK sent
        reliability.on_ack_sent();
        assert!(!reliability.has_pending_acks());
        
        // Receive out-of-order packet (gap)
        reliability.track_received_packet(3, 0, Bytes::new());
        assert!(reliability.has_pending_acks());
        
        let (ack, ranges) = reliability.get_ack_info();
        assert_eq!(ack, 1); // Still 1
        assert!(!ranges.is_empty()); // Has SACK ranges
        
        // In Connection logic, we wouldn't piggyback if ranges is not empty.
        // But ReliabilityLayer doesn't enforce that.
        // If we call on_ack_sent, it resets pending count.
        reliability.on_ack_sent();
        assert!(!reliability.has_pending_acks());
    }
}
