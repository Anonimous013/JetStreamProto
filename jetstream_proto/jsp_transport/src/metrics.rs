use std::sync::atomic::{AtomicU64, Ordering};
use std::fmt;

/// Collection of metrics for a connection or server
#[derive(Debug, Default)]
pub struct Metrics {
    // Traffic
    pub packets_sent: AtomicU64,
    pub packets_received: AtomicU64,
    pub bytes_sent: AtomicU64,
    pub bytes_received: AtomicU64,
    
    // Reliability
    pub packets_lost: AtomicU64,
    pub packets_retransmitted: AtomicU64,
    pub duplicate_packets_received: AtomicU64,
    
    // Performance
    pub rtt_ms: AtomicU64, // Current RTT in milliseconds
    pub congestion_window: AtomicU64, // Current cwnd in bytes
    
    // Errors
    pub connection_errors: AtomicU64,
    pub timeouts: AtomicU64,
    pub circuit_breaker_trips: AtomicU64,
}

impl Metrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_packet_sent(&self, size: usize) {
        self.packets_sent.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(size as u64, Ordering::Relaxed);
    }

    pub fn record_packet_received(&self, size: usize) {
        self.packets_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(size as u64, Ordering::Relaxed);
    }

    pub fn record_retransmit(&self, size: usize) {
        self.packets_retransmitted.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(size as u64, Ordering::Relaxed); // Retransmits count as sent bytes
    }

    pub fn record_loss(&self) {
        self.packets_lost.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_duplicate(&self) {
        self.duplicate_packets_received.fetch_add(1, Ordering::Relaxed);
    }

    pub fn update_rtt(&self, rtt_ms: u64) {
        self.rtt_ms.store(rtt_ms, Ordering::Relaxed);
    }

    pub fn update_cwnd(&self, cwnd: u64) {
        self.congestion_window.store(cwnd, Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.connection_errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_timeout(&self) {
        self.timeouts.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_circuit_breaker_trip(&self) {
        self.circuit_breaker_trips.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_avg_rtt(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.rtt_ms.load(Ordering::Relaxed))
    }

    /// Get a snapshot of the current metrics
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            packets_sent: self.packets_sent.load(Ordering::Relaxed),
            packets_received: self.packets_received.load(Ordering::Relaxed),
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            packets_lost: self.packets_lost.load(Ordering::Relaxed),
            packets_retransmitted: self.packets_retransmitted.load(Ordering::Relaxed),
            duplicate_packets_received: self.duplicate_packets_received.load(Ordering::Relaxed),
            rtt_ms: self.rtt_ms.load(Ordering::Relaxed),
            congestion_window: self.congestion_window.load(Ordering::Relaxed),
            connection_errors: self.connection_errors.load(Ordering::Relaxed),
            timeouts: self.timeouts.load(Ordering::Relaxed),
            circuit_breaker_trips: self.circuit_breaker_trips.load(Ordering::Relaxed),
        }
    }
}

/// A snapshot of metrics values (immutable)
#[derive(Debug, Clone, Copy)]
pub struct MetricsSnapshot {
    pub packets_sent: u64,
    pub packets_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_lost: u64,
    pub packets_retransmitted: u64,
    pub duplicate_packets_received: u64,
    pub rtt_ms: u64,
    pub congestion_window: u64,
    pub connection_errors: u64,
    pub timeouts: u64,
    pub circuit_breaker_trips: u64,
}

impl fmt::Display for MetricsSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "--- Connection Metrics ---")?;
        writeln!(f, "Traffic:")?;
        writeln!(f, "  Sent: {} pkts / {} bytes", self.packets_sent, self.bytes_sent)?;
        writeln!(f, "  Recv: {} pkts / {} bytes", self.packets_received, self.bytes_received)?;
        writeln!(f, "Reliability:")?;
        writeln!(f, "  Lost: {}", self.packets_lost)?;
        writeln!(f, "  Retransmitted: {}", self.packets_retransmitted)?;
        writeln!(f, "  Duplicates: {}", self.duplicate_packets_received)?;
        writeln!(f, "Performance:")?;
        writeln!(f, "  RTT: {} ms", self.rtt_ms)?;
        writeln!(f, "  Cwnd: {} bytes", self.congestion_window)?;
        writeln!(f, "Errors:")?;
        writeln!(f, "  Errors: {}", self.connection_errors)?;
        writeln!(f, "  Timeouts: {}", self.timeouts)?;
        writeln!(f, "  CB Trips: {}", self.circuit_breaker_trips)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_counting() {
        let metrics = Metrics::new();
        
        metrics.record_packet_sent(100);
        metrics.record_packet_sent(200);
        metrics.record_packet_received(50);
        
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.packets_sent, 2);
        assert_eq!(snapshot.bytes_sent, 300);
        assert_eq!(snapshot.packets_received, 1);
        assert_eq!(snapshot.bytes_received, 50);
    }

    #[test]
    fn test_metrics_updates() {
        let metrics = Metrics::new();
        
        metrics.update_rtt(50);
        metrics.update_cwnd(10000);
        
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.rtt_ms, 50);
        assert_eq!(snapshot.congestion_window, 10000);
        
        metrics.update_rtt(45);
        assert_eq!(metrics.snapshot().rtt_ms, 45);
    }
}
