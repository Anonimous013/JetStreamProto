use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};

/// IP blacklist entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlacklistEntry {
    pub ip: IpAddr,
    pub reason: String,
    pub banned_at: u64,
    pub expires_at: Option<u64>,
}

/// IP blacklist manager
#[derive(Clone)]
pub struct IpBlacklist {
    /// Permanently banned IPs
    permanent: Arc<RwLock<HashSet<IpAddr>>>,
    /// Temporarily banned IPs with expiration
    temporary: Arc<RwLock<Vec<BlacklistEntry>>>,
}

impl IpBlacklist {
    pub fn new() -> Self {
        let blacklist = Self {
            permanent: Arc::new(RwLock::new(HashSet::new())),
            temporary: Arc::new(RwLock::new(Vec::new())),
        };
        
        // Start cleanup task
        let temp = blacklist.temporary.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                let mut entries = temp.write().await;
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                
                entries.retain(|entry| {
                    if let Some(expires) = entry.expires_at {
                        expires > now
                    } else {
                        true
                    }
                });
            }
        });
        
        blacklist
    }
    
    /// Check if an IP is blacklisted
    pub async fn is_blacklisted(&self, ip: IpAddr) -> bool {
        // Check permanent
        if self.permanent.read().await.contains(&ip) {
            return true;
        }
        
        // Check temporary
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let temp = self.temporary.read().await;
        temp.iter().any(|entry| {
            entry.ip == ip && entry.expires_at.map_or(true, |exp| exp > now)
        })
    }
    
    /// Add IP to permanent blacklist
    pub async fn ban_permanent(&self, ip: IpAddr, reason: String) {
        self.permanent.write().await.insert(ip);
        tracing::warn!(%ip, %reason, "IP permanently banned");
    }
    
    /// Add IP to temporary blacklist
    pub async fn ban_temporary(&self, ip: IpAddr, reason: String, duration: Duration) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let entry = BlacklistEntry {
            ip,
            reason: reason.clone(),
            banned_at: now,
            expires_at: Some(now + duration.as_secs()),
        };
        
        self.temporary.write().await.push(entry);
        tracing::warn!(%ip, %reason, duration_secs = duration.as_secs(), "IP temporarily banned");
    }
    
    /// Remove IP from blacklist
    pub async fn unban(&self, ip: IpAddr) {
        self.permanent.write().await.remove(&ip);
        self.temporary.write().await.retain(|entry| entry.ip != ip);
        tracing::info!(%ip, "IP unbanned");
    }
    
    /// Get all blacklisted IPs
    pub async fn get_all(&self) -> Vec<BlacklistEntry> {
        let mut entries = Vec::new();
        
        // Add permanent
        let perm = self.permanent.read().await;
        for ip in perm.iter() {
            entries.push(BlacklistEntry {
                ip: *ip,
                reason: "Permanent ban".to_string(),
                banned_at: 0,
                expires_at: None,
            });
        }
        
        // Add temporary
        let temp = self.temporary.read().await;
        entries.extend(temp.clone());
        
        entries
    }
}

impl Default for IpBlacklist {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_permanent_ban() {
        let blacklist = IpBlacklist::new();
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        
        assert!(!blacklist.is_blacklisted(ip).await);
        
        blacklist.ban_permanent(ip, "Test ban".to_string()).await;
        assert!(blacklist.is_blacklisted(ip).await);
        
        blacklist.unban(ip).await;
        assert!(!blacklist.is_blacklisted(ip).await);
    }

    #[tokio::test]
    async fn test_temporary_ban() {
        let blacklist = IpBlacklist::new();
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));
        
        // Ban for 1 second
        blacklist.ban_temporary(ip, "Test ban".to_string(), Duration::from_secs(1)).await;
        
        // Should be blacklisted immediately
        assert!(blacklist.is_blacklisted(ip).await);
        
        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(1100)).await;
        assert!(!blacklist.is_blacklisted(ip).await);
    }
}
