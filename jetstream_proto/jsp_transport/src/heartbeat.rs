use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::interval;
use anyhow::Result;

/// Application state for battery optimization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    Foreground,
    Background,
}

/// Configuration for heartbeat intervals
#[derive(Debug, Clone)]
pub struct HeartbeatConfig {
    pub foreground_interval: Duration,
    pub background_interval: Duration,
    pub timeout_count: u32,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            foreground_interval: Duration::from_secs(5),
            background_interval: Duration::from_secs(30),
            timeout_count: 3,
        }
    }
}

/// Heartbeat manager for connection liveness checks
#[derive(Debug)]
pub struct HeartbeatManager {
    /// Heartbeat sequence number
    sequence: Arc<RwLock<u64>>,
    /// Last heartbeat sent timestamp
    last_sent: Arc<RwLock<Instant>>,
    /// Last heartbeat received timestamp
    last_received: Arc<RwLock<Instant>>,
    /// Current configuration
    config: HeartbeatConfig,
    /// Current application state
    app_state: Arc<RwLock<AppState>>,
}

impl HeartbeatManager {
    pub fn new(config: HeartbeatConfig) -> Self {
        let now = Instant::now();
        Self {
            sequence: Arc::new(RwLock::new(0)),
            last_sent: Arc::new(RwLock::new(now)),
            last_received: Arc::new(RwLock::new(now)),
            config,
            app_state: Arc::new(RwLock::new(AppState::Foreground)),
        }
    }

    /// Set application state (Foreground/Background)
    pub async fn set_app_state(&self, state: AppState) {
        let mut current = self.app_state.write().await;
        if *current != state {
            *current = state;
            tracing::info!(state = ?state, "App state changed, adjusting heartbeat interval");
        }
    }

    /// Get current heartbeat interval based on app state
    pub async fn current_interval(&self) -> Duration {
        let state = self.app_state.read().await;
        match *state {
            AppState::Foreground => self.config.foreground_interval,
            AppState::Background => self.config.background_interval,
        }
    }

    /// Get next heartbeat sequence number
    pub async fn next_sequence(&self) -> u64 {
        let mut seq = self.sequence.write().await;
        *seq += 1;
        *seq
    }

    /// Update last sent timestamp
    pub async fn mark_sent(&self) {
        let mut last = self.last_sent.write().await;
        *last = Instant::now();
    }

    /// Update last received timestamp
    pub async fn mark_received(&self) {
        let mut last = self.last_received.write().await;
        *last = Instant::now();
    }

    /// Check if heartbeat should be sent
    pub async fn should_send(&self) -> bool {
        let last = self.last_sent.read().await;
        last.elapsed() >= self.current_interval().await
    }

    /// Check if connection has timed out
    pub async fn is_timed_out(&self) -> bool {
        let last = self.last_received.read().await;
        let interval = self.current_interval().await;
        let timeout_duration = interval * self.config.timeout_count;
        last.elapsed() >= timeout_duration
    }

    /// Get time since last heartbeat received
    pub async fn time_since_last_received(&self) -> Duration {
        let last = self.last_received.read().await;
        last.elapsed()
    }

    /// Start heartbeat monitoring task
    pub fn start_monitoring<F>(
        self: Arc<Self>,
        mut send_heartbeat: F,
    ) -> tokio::task::JoinHandle<()>
    where
        F: FnMut(u64) -> Result<()> + Send + 'static,
    {
        tokio::spawn(async move {
            // Check frequently enough to catch interval changes, but not too busy loop
            let mut ticker = interval(Duration::from_secs(1));
            
            loop {
                ticker.tick().await;
                
                // Check if connection timed out
                if self.is_timed_out().await {
                    tracing::warn!("Connection timed out - no heartbeat received");
                    break;
                }
                
                // Send heartbeat if needed
                if self.should_send().await {
                    let seq = self.next_sequence().await;
                    if let Err(e) = send_heartbeat(seq) {
                        tracing::error!("Failed to send heartbeat: {}", e);
                        break;
                    }
                    self.mark_sent().await;
                    tracing::debug!("Heartbeat sent: seq={}", seq);
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_heartbeat_sequence() {
        let config = HeartbeatConfig {
            foreground_interval: Duration::from_secs(5),
            background_interval: Duration::from_secs(30),
            timeout_count: 3,
        };
        let manager = HeartbeatManager::new(config);
        
        let seq1 = manager.next_sequence().await;
        let seq2 = manager.next_sequence().await;
        
        assert_eq!(seq1, 1);
        assert_eq!(seq2, 2);
    }

    #[tokio::test]
    async fn test_heartbeat_timing() {
        let config = HeartbeatConfig {
            foreground_interval: Duration::from_secs(1),
            background_interval: Duration::from_secs(5),
            timeout_count: 3,
        };
        let manager = HeartbeatManager::new(config);
        
        // Initially should not send
        assert!(!manager.should_send().await);
        
        // Wait for interval
        sleep(Duration::from_millis(1100)).await;
        
        // Now should send
        assert!(manager.should_send().await);
    }

    #[tokio::test]
    async fn test_heartbeat_timeout() {
        let config = HeartbeatConfig {
            foreground_interval: Duration::from_secs(1),
            background_interval: Duration::from_secs(5),
            timeout_count: 2,
        };
        let manager = HeartbeatManager::new(config);
        
        // Initially not timed out
        assert!(!manager.is_timed_out().await);
        
        // Wait for timeout (2 intervals)
        sleep(Duration::from_millis(2100)).await;
        
        // Should be timed out
        assert!(manager.is_timed_out().await);
    }

    #[tokio::test]
    async fn test_heartbeat_received_resets_timeout() {
        let config = HeartbeatConfig {
            foreground_interval: Duration::from_secs(1),
            background_interval: Duration::from_secs(5),
            timeout_count: 2,
        };
        let manager = HeartbeatManager::new(config);
        
        sleep(Duration::from_millis(1500)).await;
        
        // Mark as received
        manager.mark_received().await;
        
        // Should not be timed out
        assert!(!manager.is_timed_out().await);
    }

    #[tokio::test]
    async fn test_app_state_change() {
        let config = HeartbeatConfig {
            foreground_interval: Duration::from_secs(1),
            background_interval: Duration::from_secs(5),
            timeout_count: 3,
        };
        let manager = HeartbeatManager::new(config);

        // Default is Foreground (1s)
        assert_eq!(manager.current_interval().await, Duration::from_secs(1));

        // Switch to Background (5s)
        manager.set_app_state(AppState::Background).await;
        assert_eq!(manager.current_interval().await, Duration::from_secs(5));
    }
}
