use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use crate::rate_limit::RateLimiter;

#[derive(Debug, Clone)]
pub struct DdosConfig {
    /// Max packets per second per IP
    pub max_packets_per_ip: u32,
    /// Max bytes per second per IP
    pub max_bytes_per_ip: u64,
    /// Max handshakes (ClientHello) per second per IP
    pub max_handshakes_per_ip: u32,
    /// Duration to ban an IP if it exceeds limits significantly (optional)
    pub ban_duration: Option<Duration>,
    /// Cleanup interval for removing stale IP records
    pub cleanup_interval: Duration,
}

impl Default for DdosConfig {
    fn default() -> Self {
        Self {
            max_packets_per_ip: 1000,
            max_bytes_per_ip: 1024 * 1024, // 1 MB/s
            max_handshakes_per_ip: 5,
            ban_duration: Some(Duration::from_secs(60)),
            cleanup_interval: Duration::from_secs(60),
        }
    }
}

struct IpState {
    packet_limiter: RateLimiter,
    handshake_limiter: RateLimiter,
    last_activity: Instant,
    banned_until: Option<Instant>,
}

impl IpState {
    fn new(config: &DdosConfig) -> Self {
        Self {
            packet_limiter: RateLimiter::new(config.max_packets_per_ip, config.max_bytes_per_ip),
            // For handshakes, we only care about count, so use a large byte limit
            handshake_limiter: RateLimiter::new(config.max_handshakes_per_ip, u64::MAX),
            last_activity: Instant::now(),
            banned_until: None,
        }
    }
}

#[derive(Clone)]
pub struct DdosProtection {
    config: DdosConfig,
    ip_states: Arc<RwLock<HashMap<IpAddr, IpState>>>,
    // cleanup_task is spawned detached, so we don't hold a handle here to simplify cloning
    // In a real app we might want to control it better
}

impl DdosProtection {
    pub fn new(config: DdosConfig) -> Self {
        let protection = Self {
            config: config.clone(),
            ip_states: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Start cleanup task
        let ip_states = protection.ip_states.clone();
        let interval = config.cleanup_interval;
        
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                let mut states = ip_states.write().await;
                let now = Instant::now();
                
                // Remove entries inactive for more than 5 minutes (or configurable)
                states.retain(|_, state| {
                    now.duration_since(state.last_activity) < Duration::from_secs(300)
                });
            }
        });
        
        protection
    }

    /// Check if a packet from the given IP is allowed
    pub async fn check_packet(&self, ip: IpAddr, len: usize) -> bool {
        let mut states = self.ip_states.write().await;
        
        let state = states.entry(ip).or_insert_with(|| IpState::new(&self.config));
        state.last_activity = Instant::now();
        
        // Check ban
        if let Some(banned_until) = state.banned_until {
            if Instant::now() < banned_until {
                return false;
            } else {
                state.banned_until = None;
            }
        }
        
        if !state.packet_limiter.check_and_consume(len) {
            // If limit exceeded, potentially ban
            // For now, just reject.
            return false;
        }
        
        true
    }

    /// Check if a handshake attempt from the given IP is allowed
    pub async fn check_handshake(&self, ip: IpAddr) -> bool {
        let mut states = self.ip_states.write().await;
        
        let state = states.entry(ip).or_insert_with(|| IpState::new(&self.config));
        state.last_activity = Instant::now();
        
        // Check ban
        if let Some(banned_until) = state.banned_until {
            if Instant::now() < banned_until {
                return false;
            } else {
                state.banned_until = None;
            }
        }
        
        // Handshake limit check (size 0 because we only count messages)
        if !state.handshake_limiter.check_and_consume(0) {
            // Ban on handshake flood
            if let Some(ban_duration) = self.config.ban_duration {
                state.banned_until = Some(Instant::now() + ban_duration);
                tracing::warn!(%ip, "IP banned due to handshake flood");
            }
            return false;
        }
        
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_packet_limit() {
        let config = DdosConfig {
            max_packets_per_ip: 5,
            ..Default::default()
        };
        let ddos = DdosProtection::new(config);
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        
        for _ in 0..5 {
            assert!(ddos.check_packet(ip, 100).await);
        }
        
        assert!(!ddos.check_packet(ip, 100).await);
    }

    #[tokio::test]
    async fn test_handshake_ban() {
        let config = DdosConfig {
            max_handshakes_per_ip: 2,
            ban_duration: Some(Duration::from_millis(100)),
            ..Default::default()
        };
        let ddos = DdosProtection::new(config);
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        
        assert!(ddos.check_handshake(ip).await);
        assert!(ddos.check_handshake(ip).await);
        
        // Next one fails and bans
        assert!(!ddos.check_handshake(ip).await);
        
        // Should still be banned even if packet limit is fine
        assert!(!ddos.check_packet(ip, 100).await);
    }
}
