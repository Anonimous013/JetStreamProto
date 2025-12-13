//! Health Checking

use super::Backend;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::interval;

/// Health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Health check result
#[derive(Debug, Clone)]
pub struct HealthCheck {
    pub backend_id: String,
    pub status: HealthStatus,
    pub latency_ms: u64,
    pub last_check: Instant,
}

/// Health checker
pub struct HealthChecker {
    check_interval: Duration,
    timeout: Duration,
}

impl HealthChecker {
    /// Create a new health checker
    pub fn new(check_interval: Duration, timeout: Duration) -> Self {
        Self {
            check_interval,
            timeout,
        }
    }

    /// Start health checking for backends
    pub async fn start_checking(
        &self,
        backends: Vec<Arc<Backend>>,
        mut health_callback: impl FnMut(HealthCheck) + Send + 'static,
    ) {
        let check_interval = self.check_interval;
        let timeout = self.timeout;

        tokio::spawn(async move {
            let mut interval = interval(check_interval);

            loop {
                interval.tick().await;

                for backend in &backends {
                    let start = Instant::now();
                    let status = Self::check_backend(backend, timeout).await;
                    let latency = start.elapsed().as_millis() as u64;

                    health_callback(HealthCheck {
                        backend_id: backend.id.clone(),
                        status,
                        latency_ms: latency,
                        last_check: Instant::now(),
                    });
                }
            }
        });
    }

    /// Check a single backend
    async fn check_backend(backend: &Backend, timeout: Duration) -> HealthStatus {
        // Simplified health check - in production, this would make actual TCP/HTTP requests
        let check_result = tokio::time::timeout(timeout, async {
            // Simulate health check
            tokio::time::sleep(Duration::from_millis(10)).await;
            Ok::<_, ()>(())
        })
        .await;

        match check_result {
            Ok(Ok(_)) => {
                // Check connection count for degradation
                if backend.connections() > 100 {
                    HealthStatus::Degraded
                } else {
                    HealthStatus::Healthy
                }
            }
            Ok(Err(_)) => HealthStatus::Unhealthy,
            Err(_) => HealthStatus::Unhealthy, // Timeout
        }
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new(Duration::from_secs(10), Duration::from_secs(5))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_checker() {
        let checker = HealthChecker::default();
        let backend = Arc::new(Backend::new("test", "127.0.0.1:8000", 1));
        
        let status = HealthChecker::check_backend(&backend, Duration::from_secs(1)).await;
        assert_eq!(status, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_degraded_status() {
        let backend = Arc::new(Backend::new("test", "127.0.0.1:8000", 1));
        
        // Simulate high connection count
        for _ in 0..101 {
            backend.inc_connections();
        }
        
        let status = HealthChecker::check_backend(&backend, Duration::from_secs(1)).await;
        assert_eq!(status, HealthStatus::Degraded);
    }
}
