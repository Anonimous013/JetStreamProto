use std::time::{Duration, Instant};

/// Path MTU Discovery
/// 
/// Automatically determines the maximum transmission unit (MTU) for a network path
/// to optimize packet sizes and avoid fragmentation
pub struct MtuDiscovery {
    /// Current MTU estimate
    current_mtu: usize,
    /// Minimum MTU (IPv4 minimum)
    min_mtu: usize,
    /// Maximum MTU to probe
    max_mtu: usize,
    /// Last successful MTU
    last_successful_mtu: usize,
    /// Last probe time
    last_probe_time: Option<Instant>,
    /// Probe interval
    probe_interval: Duration,
}

impl MtuDiscovery {
    /// Create a new MTU discovery instance
    /// 
    /// Starts with conservative 1280 bytes (IPv6 minimum MTU)
    pub fn new() -> Self {
        Self {
            current_mtu: 1280,      // IPv6 minimum MTU
            min_mtu: 576,            // IPv4 minimum MTU
            max_mtu: 1500,           // Ethernet MTU
            last_successful_mtu: 1280,
            last_probe_time: None,
            probe_interval: Duration::from_secs(60), // Probe every 60 seconds
        }
    }
    
    /// Create with custom MTU range
    pub fn with_range(min_mtu: usize, max_mtu: usize) -> Self {
        let initial_mtu = min_mtu;
        Self {
            current_mtu: initial_mtu,
            min_mtu,
            max_mtu,
            last_successful_mtu: initial_mtu,
            last_probe_time: None,
            probe_interval: Duration::from_secs(60),
        }
    }
    
    /// Get current MTU estimate
    pub fn current_mtu(&self) -> usize {
        self.current_mtu
    }
    
    /// Get recommended payload size (MTU - headers)
    /// 
    /// Accounts for:
    /// - IP header: 20 bytes (IPv4) or 40 bytes (IPv6)
    /// - UDP header: 8 bytes
    /// - Protocol overhead: ~50 bytes for safety
    pub fn recommended_payload_size(&self) -> usize {
        self.current_mtu.saturating_sub(100) // Conservative overhead estimate
    }
    
    /// Check if it's time to probe for MTU
    pub fn should_probe(&self) -> bool {
        match self.last_probe_time {
            None => true,
            Some(last_time) => last_time.elapsed() >= self.probe_interval,
        }
    }
    
    /// Get next probe size for binary search
    pub fn next_probe_size(&self) -> Option<usize> {
        if self.current_mtu >= self.max_mtu {
            return None; // Already at maximum
        }
        
        // Binary search between current and max
        let mid = (self.current_mtu + self.max_mtu) / 2;
        if mid <= self.current_mtu {
            return None; // Converged
        }
        
        Some(mid)
    }
    
    /// Report successful transmission at given size
    pub fn report_success(&mut self, size: usize) {
        if size > self.last_successful_mtu {
            self.last_successful_mtu = size;
            self.current_mtu = size;
            tracing::debug!(mtu = size, "MTU increased");
        }
        self.last_probe_time = Some(Instant::now());
    }
    
    /// Report failed transmission (likely MTU exceeded)
    pub fn report_failure(&mut self, size: usize) {
        if size <= self.current_mtu {
            // Failure at or below current MTU - reduce it
            self.current_mtu = self.last_successful_mtu;
            tracing::warn!(
                failed_size = size,
                new_mtu = self.current_mtu,
                "MTU probe failed, reducing MTU"
            );
        }
        self.last_probe_time = Some(Instant::now());
    }
    
    /// Reset to conservative MTU (e.g., after network change)
    pub fn reset(&mut self) {
        self.current_mtu = self.min_mtu;
        self.last_successful_mtu = self.min_mtu;
        self.last_probe_time = None;
        tracing::info!(mtu = self.min_mtu, "MTU reset to minimum");
    }
    
    /// Set probe interval
    pub fn set_probe_interval(&mut self, interval: Duration) {
        self.probe_interval = interval;
    }
    
    /// Get statistics
    pub fn stats(&self) -> MtuStats {
        MtuStats {
            current_mtu: self.current_mtu,
            min_mtu: self.min_mtu,
            max_mtu: self.max_mtu,
            last_successful_mtu: self.last_successful_mtu,
            recommended_payload: self.recommended_payload_size(),
        }
    }
}

impl Default for MtuDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

/// MTU discovery statistics
#[derive(Debug, Clone)]
pub struct MtuStats {
    pub current_mtu: usize,
    pub min_mtu: usize,
    pub max_mtu: usize,
    pub last_successful_mtu: usize,
    pub recommended_payload: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mtu_discovery_initial() {
        let mtu = MtuDiscovery::new();
        assert_eq!(mtu.current_mtu(), 1280);
        assert!(mtu.recommended_payload_size() > 0);
        assert!(mtu.recommended_payload_size() < 1280);
    }
    
    #[test]
    fn test_mtu_probe_sequence() {
        let mut mtu = MtuDiscovery::new();
        
        // Should probe initially
        assert!(mtu.should_probe());
        
        // Get next probe size
        let probe_size = mtu.next_probe_size().unwrap();
        assert!(probe_size > 1280);
        assert!(probe_size <= 1500);
        
        // Report success
        mtu.report_success(probe_size);
        assert_eq!(mtu.current_mtu(), probe_size);
    }
    
    #[test]
    fn test_mtu_failure_handling() {
        let mut mtu = MtuDiscovery::new();
        
        // Try larger size
        mtu.report_success(1400);
        assert_eq!(mtu.current_mtu(), 1400);
        
        // Report failure at even larger size
        mtu.report_failure(1500);
        
        // Should fall back to last successful
        assert_eq!(mtu.current_mtu(), 1400);
    }
    
    #[test]
    fn test_mtu_reset() {
        let mut mtu = MtuDiscovery::new();
        
        mtu.report_success(1400);
        assert_eq!(mtu.current_mtu(), 1400);
        
        mtu.reset();
        assert_eq!(mtu.current_mtu(), 576); // Back to minimum
    }
    
    #[test]
    fn test_custom_mtu_range() {
        let mtu = MtuDiscovery::with_range(1000, 9000);
        assert_eq!(mtu.current_mtu(), 1000);
        assert_eq!(mtu.min_mtu, 1000);
        assert_eq!(mtu.max_mtu, 9000);
    }
    
    #[test]
    fn test_recommended_payload_size() {
        let mtu = MtuDiscovery::new();
        let payload_size = mtu.recommended_payload_size();
        
        // Should account for headers
        assert!(payload_size < mtu.current_mtu());
        assert!(payload_size > 1000); // Should be reasonable
    }
    
    #[test]
    fn test_mtu_stats() {
        let mtu = MtuDiscovery::new();
        let stats = mtu.stats();
        
        assert_eq!(stats.current_mtu, 1280);
        assert_eq!(stats.min_mtu, 576);
        assert_eq!(stats.max_mtu, 1500);
    }
}
