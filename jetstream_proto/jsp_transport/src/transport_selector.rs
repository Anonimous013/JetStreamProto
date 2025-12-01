use serde::{Serialize, Deserialize};
use std::time::Duration;

/// Transport type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportType {
    /// UDP transport (default, lowest latency)
    Udp,
    /// TCP transport (reliable, firewall-friendly)
    Tcp,
    /// QUIC transport (best of both worlds)
    Quic,
}

/// Network conditions
#[derive(Debug, Clone)]
pub struct NetworkConditions {
    /// Packet loss rate (0.0 - 1.0)
    pub packet_loss: f32,
    /// Average RTT
    pub rtt: Duration,
    /// Bandwidth (bytes/sec)
    pub bandwidth: u64,
    /// Jitter (RTT variance)
    pub jitter: Duration,
    /// NAT type detected
    pub behind_nat: bool,
}

impl NetworkConditions {
    pub fn new() -> Self {
        Self {
            packet_loss: 0.0,
            rtt: Duration::from_millis(50),
            bandwidth: 10_000_000, // 10 Mbps
            jitter: Duration::from_millis(5),
            behind_nat: false,
        }
    }

    /// Check if conditions are good
    pub fn is_good(&self) -> bool {
        self.packet_loss < 0.01 && self.rtt < Duration::from_millis(100)
    }

    /// Check if conditions are poor
    pub fn is_poor(&self) -> bool {
        self.packet_loss > 0.05 || self.rtt > Duration::from_millis(500)
    }
}

impl Default for NetworkConditions {
    fn default() -> Self {
        Self::new()
    }
}

/// Transport selector
#[derive(Debug, Clone)]
pub struct TransportSelector {
    /// Current transport
    current: TransportType,
    /// Preferred transport
    preferred: TransportType,
    /// Fallback chain
    fallback: Vec<TransportType>,
    /// Last switch time
    last_switch: Option<std::time::Instant>,
    /// Minimum time between switches
    min_switch_interval: Duration,
}

impl TransportSelector {
    /// Create new transport selector
    pub fn new() -> Self {
        Self {
            current: TransportType::Udp,
            preferred: TransportType::Udp,
            fallback: vec![TransportType::Quic, TransportType::Tcp],
            last_switch: None,
            min_switch_interval: Duration::from_secs(30),
        }
    }

    /// Create with preferred transport
    pub fn with_preferred(preferred: TransportType) -> Self {
        let fallback = match preferred {
            TransportType::Udp => vec![TransportType::Quic, TransportType::Tcp],
            TransportType::Quic => vec![TransportType::Udp, TransportType::Tcp],
            TransportType::Tcp => vec![TransportType::Quic, TransportType::Udp],
        };

        Self {
            current: preferred,
            preferred,
            fallback,
            last_switch: None,
            min_switch_interval: Duration::from_secs(30),
        }
    }

    /// Select optimal transport based on conditions
    pub fn select_transport(&self, conditions: &NetworkConditions) -> TransportType {
        // If behind NAT, prefer QUIC or TCP
        if conditions.behind_nat {
            return if self.is_available(TransportType::Quic) {
                TransportType::Quic
            } else {
                TransportType::Tcp
            };
        }

        // If high packet loss, use TCP
        if conditions.packet_loss > 0.1 {
            return TransportType::Tcp;
        }

        // If moderate packet loss, use QUIC
        if conditions.packet_loss > 0.03 {
            return if self.is_available(TransportType::Quic) {
                TransportType::Quic
            } else {
                TransportType::Tcp
            };
        }

        // Good conditions, use preferred (usually UDP)
        self.preferred
    }

    /// Check if should switch transport
    pub fn should_switch(&self, conditions: &NetworkConditions) -> bool {
        // Don't switch too frequently
        if let Some(last_switch) = self.last_switch {
            if last_switch.elapsed() < self.min_switch_interval {
                return false;
            }
        }

        let optimal = self.select_transport(conditions);
        optimal != self.current
    }

    /// Get current transport
    pub fn current_transport(&self) -> TransportType {
        self.current
    }

    /// Set current transport
    pub fn set_current(&mut self, transport: TransportType) {
        self.current = transport;
        self.last_switch = Some(std::time::Instant::now());
    }

    /// Get optimal transport
    pub fn get_optimal_transport(&self, conditions: &NetworkConditions) -> TransportType {
        self.select_transport(conditions)
    }

    /// Check if transport is available
    fn is_available(&self, transport: TransportType) -> bool {
        // In real implementation, this would check actual availability
        // For now, assume all are available except we prefer certain ones
        match transport {
            TransportType::Quic => true, // QUIC available if supported
            TransportType::Tcp => true,  // TCP always available
            TransportType::Udp => true,  // UDP always available
        }
    }

    /// Get fallback transport
    pub fn get_fallback(&self) -> Option<TransportType> {
        self.fallback.first().copied()
    }

    /// Set minimum switch interval
    pub fn set_min_switch_interval(&mut self, interval: Duration) {
        self.min_switch_interval = interval;
    }
}

impl Default for TransportSelector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_conditions() {
        let conditions = NetworkConditions::new();
        assert!(conditions.is_good());
        assert!(!conditions.is_poor());
    }

    #[test]
    fn test_poor_conditions() {
        let mut conditions = NetworkConditions::new();
        conditions.packet_loss = 0.1;
        assert!(!conditions.is_good());
        assert!(conditions.is_poor());
    }

    #[test]
    fn test_transport_selection_good_conditions() {
        let selector = TransportSelector::new();
        let conditions = NetworkConditions::new();
        assert_eq!(selector.select_transport(&conditions), TransportType::Udp);
    }

    #[test]
    fn test_transport_selection_high_loss() {
        let selector = TransportSelector::new();
        let mut conditions = NetworkConditions::new();
        conditions.packet_loss = 0.15;
        assert_eq!(selector.select_transport(&conditions), TransportType::Tcp);
    }

    #[test]
    fn test_transport_selection_moderate_loss() {
        let selector = TransportSelector::new();
        let mut conditions = NetworkConditions::new();
        conditions.packet_loss = 0.05;
        assert_eq!(selector.select_transport(&conditions), TransportType::Quic);
    }

    #[test]
    fn test_transport_selection_behind_nat() {
        let selector = TransportSelector::new();
        let mut conditions = NetworkConditions::new();
        conditions.behind_nat = true;
        let transport = selector.select_transport(&conditions);
        assert!(transport == TransportType::Quic || transport == TransportType::Tcp);
    }

    #[test]
    fn test_should_switch() {
        let mut selector = TransportSelector::new();
        selector.set_min_switch_interval(Duration::from_millis(100));
        
        let mut conditions = NetworkConditions::new();
        conditions.packet_loss = 0.15; // Should prefer TCP
        
        assert!(selector.should_switch(&conditions));
        
        selector.set_current(TransportType::Tcp);
        assert!(!selector.should_switch(&conditions));
    }

    #[test]
    fn test_preferred_transport() {
        let selector = TransportSelector::with_preferred(TransportType::Quic);
        assert_eq!(selector.current_transport(), TransportType::Quic);
        
        let conditions = NetworkConditions::new();
        assert_eq!(selector.select_transport(&conditions), TransportType::Quic);
    }

    #[test]
    fn test_fallback() {
        let selector = TransportSelector::new();
        let fallback = selector.get_fallback();
        assert!(fallback.is_some());
        assert_eq!(fallback.unwrap(), TransportType::Quic);
    }

    #[test]
    fn test_current_transport() {
        let mut selector = TransportSelector::new();
        assert_eq!(selector.current_transport(), TransportType::Udp);
        
        selector.set_current(TransportType::Tcp);
        assert_eq!(selector.current_transport(), TransportType::Tcp);
    }
}
