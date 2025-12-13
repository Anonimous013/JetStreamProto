//! Advanced Load Balancing Module
//! 
//! Provides production-ready load balancing for JetStreamProto.

pub mod algorithms;
pub mod health;
pub mod balancer;

pub use balancer::LoadBalancer;
pub use algorithms::{BalancingAlgorithm, RoundRobin, LeastConnections, WeightedRoundRobin, ConsistentHash};
pub use health::{HealthChecker, HealthStatus};

/// Backend server
#[derive(Debug, Clone)]
pub struct Backend {
    pub id: String,
    pub address: String,
    pub weight: u32,
    pub active_connections: std::sync::Arc<std::sync::atomic::AtomicU32>,
}

impl Backend {
    /// Create a new backend
    pub fn new(id: impl Into<String>, address: impl Into<String>, weight: u32) -> Self {
        Self {
            id: id.into(),
            address: address.into(),
            weight,
            active_connections: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)),
        }
    }

    /// Increment active connections
    pub fn inc_connections(&self) {
        self.active_connections.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Decrement active connections
    pub fn dec_connections(&self) {
        self.active_connections.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get active connections count
    pub fn connections(&self) -> u32 {
        self.active_connections.load(std::sync::atomic::Ordering::Relaxed)
    }
}
