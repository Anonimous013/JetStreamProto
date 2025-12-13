//! Hop Trait and Common Functionality
//! 
//! Defines the core trait that all hop implementations must implement,
//! along with common types and utilities.

use async_trait::async_trait;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use crate::multihop::Result;

/// Health status of a hop
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HopHealth {
    /// Hop is healthy and operational
    Healthy,
    
    /// Hop is degraded but still functional
    Degraded,
    
    /// Hop is unhealthy and should be avoided
    Unhealthy,
    
    /// Hop status is unknown (not yet checked)
    Unknown,
}

/// Operational status of a hop
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HopStatus {
    /// Hop is not started
    Stopped,
    
    /// Hop is starting up
    Starting,
    
    /// Hop is running
    Running,
    
    /// Hop is stopping
    Stopping,
    
    /// Hop has failed
    Failed,
}

/// Statistics for a hop
#[derive(Debug, Clone, Default)]
pub struct HopStats {
    /// Total bytes sent through this hop
    pub bytes_sent: u64,
    
    /// Total bytes received through this hop
    pub bytes_received: u64,
    
    /// Number of packets sent
    pub packets_sent: u64,
    
    /// Number of packets received
    pub packets_received: u64,
    
    /// Average latency (milliseconds)
    pub avg_latency_ms: f64,
    
    /// Packet loss rate (0.0 - 1.0)
    pub packet_loss_rate: f64,
    
    /// Time when hop was started
    pub started_at: Option<Instant>,
    
    /// Last successful health check
    pub last_health_check: Option<Instant>,
}

impl HopStats {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Get uptime duration
    pub fn uptime(&self) -> Option<Duration> {
        self.started_at.map(|start| start.elapsed())
    }
    
    /// Record sent packet
    pub fn record_sent(&mut self, bytes: usize) {
        self.bytes_sent += bytes as u64;
        self.packets_sent += 1;
    }
    
    /// Record received packet
    pub fn record_received(&mut self, bytes: usize) {
        self.bytes_received += bytes as u64;
        self.packets_received += 1;
    }
    
    /// Update latency (exponential moving average)
    pub fn update_latency(&mut self, latency_ms: f64) {
        const ALPHA: f64 = 0.2; // Smoothing factor
        if self.avg_latency_ms == 0.0 {
            self.avg_latency_ms = latency_ms;
        } else {
            self.avg_latency_ms = ALPHA * latency_ms + (1.0 - ALPHA) * self.avg_latency_ms;
        }
    }
}

/// Core trait that all hop implementations must implement
#[async_trait]
pub trait Hop: Send + Sync {
    /// Start the hop (establish connection, start processes, etc.)
    async fn start(&mut self) -> Result<()>;
    
    /// Stop the hop gracefully
    async fn stop(&mut self) -> Result<()>;
    
    /// Perform health check on the hop
    async fn health_check(&self) -> Result<HopHealth>;
    
    /// Send data through this hop
    /// Returns the number of bytes sent
    async fn send(&self, data: &[u8]) -> Result<usize>;
    
    /// Receive data from this hop
    /// Returns the number of bytes received
    async fn recv(&self, buf: &mut [u8]) -> Result<usize>;
    
    /// Get the local endpoint (where this hop listens)
    fn local_endpoint(&self) -> SocketAddr;
    
    /// Get the remote endpoint (where this hop connects to)
    fn remote_endpoint(&self) -> SocketAddr;
    
    /// Get current hop status
    fn status(&self) -> HopStatus;
    
    /// Get hop statistics
    fn stats(&self) -> &HopStats;
    
    /// Get hop type name
    fn hop_type(&self) -> &str;
    
    /// Check if hop supports UDP
    fn supports_udp(&self) -> bool {
        true // Default: most hops support UDP
    }
    
    /// Check if hop supports TCP
    fn supports_tcp(&self) -> bool {
        true // Default: most hops support TCP
    }
}

/// Helper function to measure latency
pub async fn measure_latency<F, Fut>(f: F) -> (Duration, Result<()>)
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    let start = Instant::now();
    let result = f().await;
    let duration = start.elapsed();
    (duration, result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hop_stats() {
        let mut stats = HopStats::new();
        
        stats.record_sent(1000);
        assert_eq!(stats.bytes_sent, 1000);
        assert_eq!(stats.packets_sent, 1);
        
        stats.record_received(500);
        assert_eq!(stats.bytes_received, 500);
        assert_eq!(stats.packets_received, 1);
        
        stats.update_latency(10.0);
        assert_eq!(stats.avg_latency_ms, 10.0);
        
        stats.update_latency(20.0);
        // Should be exponential moving average
        assert!(stats.avg_latency_ms > 10.0 && stats.avg_latency_ms < 20.0);
    }
}
