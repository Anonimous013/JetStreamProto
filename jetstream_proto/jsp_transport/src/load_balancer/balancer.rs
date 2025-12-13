//! Load Balancer

use super::{Backend, BalancingAlgorithm, HealthChecker, HealthStatus};
use super::health::HealthCheck;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

/// Load balancer
pub struct LoadBalancer {
    backends: Arc<RwLock<Vec<Arc<Backend>>>>,
    algorithm: Arc<dyn BalancingAlgorithm>,
    health_checker: HealthChecker,
    health_status: Arc<RwLock<HashMap<String, HealthStatus>>>,
}

impl LoadBalancer {
    /// Create a new load balancer
    pub fn new(
        backends: Vec<Backend>,
        algorithm: Arc<dyn BalancingAlgorithm>,
        health_checker: HealthChecker,
    ) -> Self {
        let backends: Vec<Arc<Backend>> = backends.into_iter().map(Arc::new).collect();
        let health_status = Arc::new(RwLock::new(HashMap::new()));

        // Initialize health status
        {
            let mut status = health_status.write().unwrap();
            for backend in &backends {
                status.insert(backend.id.clone(), HealthStatus::Healthy);
            }
        }

        Self {
            backends: Arc::new(RwLock::new(backends)),
            algorithm,
            health_checker,
            health_status,
        }
    }

    /// Start health checking
    pub async fn start_health_checks(&self) {
        let backends = self.backends.read().unwrap().clone();
        let health_status = self.health_status.clone();

        self.health_checker.start_checking(backends, move |check: HealthCheck| {
            let mut status = health_status.write().unwrap();
            status.insert(check.backend_id.clone(), check.status);
            
            tracing::debug!(
                "Health check: {} = {:?} ({}ms)",
                check.backend_id,
                check.status,
                check.latency_ms
            );
        }).await;
    }

    /// Select a backend for a request
    pub fn select_backend(&self, key: Option<&str>) -> Option<Arc<Backend>> {
        let backends = self.backends.read().unwrap();
        let health_status = self.health_status.read().unwrap();

        // Filter only healthy backends
        let healthy_backends: Vec<Arc<Backend>> = backends
            .iter()
            .filter(|b| {
                health_status.get(&b.id)
                    .map(|s| *s == HealthStatus::Healthy || *s == HealthStatus::Degraded)
                    .unwrap_or(false)
            })
            .cloned()
            .collect();

        if healthy_backends.is_empty() {
            tracing::warn!("No healthy backends available");
            return None;
        }

        self.algorithm.select(&healthy_backends, key)
    }

    /// Add a backend
    pub fn add_backend(&self, backend: Backend) {
        let mut backends = self.backends.write().unwrap();
        let mut health_status = self.health_status.write().unwrap();
        
        health_status.insert(backend.id.clone(), HealthStatus::Healthy);
        backends.push(Arc::new(backend));
    }

    /// Remove a backend
    pub fn remove_backend(&self, backend_id: &str) {
        let mut backends = self.backends.write().unwrap();
        let mut health_status = self.health_status.write().unwrap();
        
        backends.retain(|b| b.id != backend_id);
        health_status.remove(backend_id);
    }

    /// Get backend count
    pub fn backend_count(&self) -> usize {
        self.backends.read().unwrap().len()
    }

    /// Get healthy backend count
    pub fn healthy_backend_count(&self) -> usize {
        let backends = self.backends.read().unwrap();
        let health_status = self.health_status.read().unwrap();

        backends.iter()
            .filter(|b| {
                health_status.get(&b.id)
                    .map(|s| *s == HealthStatus::Healthy)
                    .unwrap_or(false)
            })
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load_balancer::algorithms::RoundRobin;

    #[test]
    fn test_load_balancer_creation() {
        let backends = vec![
            Backend::new("b1", "127.0.0.1:8001", 1),
            Backend::new("b2", "127.0.0.1:8002", 1),
        ];

        let lb = LoadBalancer::new(
            backends,
            Arc::new(RoundRobin::new()),
            HealthChecker::default(),
        );

        assert_eq!(lb.backend_count(), 2);
        assert_eq!(lb.healthy_backend_count(), 2);
    }

    #[test]
    fn test_backend_selection() {
        let backends = vec![
            Backend::new("b1", "127.0.0.1:8001", 1),
            Backend::new("b2", "127.0.0.1:8002", 1),
        ];

        let lb = LoadBalancer::new(
            backends,
            Arc::new(RoundRobin::new()),
            HealthChecker::default(),
        );

        let backend = lb.select_backend(None);
        assert!(backend.is_some());
    }

    #[test]
    fn test_add_remove_backend() {
        let backends = vec![
            Backend::new("b1", "127.0.0.1:8001", 1),
        ];

        let lb = LoadBalancer::new(
            backends,
            Arc::new(RoundRobin::new()),
            HealthChecker::default(),
        );

        assert_eq!(lb.backend_count(), 1);

        lb.add_backend(Backend::new("b2", "127.0.0.1:8002", 1));
        assert_eq!(lb.backend_count(), 2);

        lb.remove_backend("b1");
        assert_eq!(lb.backend_count(), 1);
    }
}
