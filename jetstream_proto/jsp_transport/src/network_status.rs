use std::sync::Arc;
use tokio::sync::RwLock;

/// Network connection type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkType {
    Wifi,
    Cellular,
    Ethernet,
    Unknown,
}

/// Network status manager
#[derive(Debug)]
pub struct NetworkStatus {
    current_type: Arc<RwLock<NetworkType>>,
}

impl NetworkStatus {
    pub fn new() -> Self {
        Self {
            current_type: Arc::new(RwLock::new(NetworkType::Unknown)),
        }
    }

    /// Update network type
    pub async fn set_network_type(&self, net_type: NetworkType) {
        let mut current = self.current_type.write().await;
        if *current != net_type {
            *current = net_type;
            tracing::info!(network_type = ?net_type, "Network type changed");
        }
    }

    /// Get current network type
    pub async fn get_network_type(&self) -> NetworkType {
        *self.current_type.read().await
    }

    /// Check if network is metered (e.g. cellular)
    pub async fn is_metered(&self) -> bool {
        matches!(*self.current_type.read().await, NetworkType::Cellular)
    }
}

impl Default for NetworkStatus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_status() {
        let status = NetworkStatus::new();
        
        // Default is Unknown
        assert_eq!(status.get_network_type().await, NetworkType::Unknown);
        assert!(!status.is_metered().await);
        
        // Switch to Cellular
        status.set_network_type(NetworkType::Cellular).await;
        assert_eq!(status.get_network_type().await, NetworkType::Cellular);
        assert!(status.is_metered().await);
        
        // Switch to Wifi
        status.set_network_type(NetworkType::Wifi).await;
        assert_eq!(status.get_network_type().await, NetworkType::Wifi);
        assert!(!status.is_metered().await);
    }
}
