//! Load Balancing Algorithms

use super::Backend;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Load balancing algorithm trait
pub trait BalancingAlgorithm: Send + Sync {
    /// Select a backend for the request
    fn select(&self, backends: &[Arc<Backend>], key: Option<&str>) -> Option<Arc<Backend>>;
}

/// Round-robin algorithm
pub struct RoundRobin {
    counter: AtomicUsize,
}

impl RoundRobin {
    pub fn new() -> Self {
        Self {
            counter: AtomicUsize::new(0),
        }
    }
}

impl Default for RoundRobin {
    fn default() -> Self {
        Self::new()
    }
}

impl BalancingAlgorithm for RoundRobin {
    fn select(&self, backends: &[Arc<Backend>], _key: Option<&str>) -> Option<Arc<Backend>> {
        if backends.is_empty() {
            return None;
        }

        let index = self.counter.fetch_add(1, Ordering::Relaxed) % backends.len();
        Some(backends[index].clone())
    }
}

/// Least connections algorithm
pub struct LeastConnections;

impl LeastConnections {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LeastConnections {
    fn default() -> Self {
        Self::new()
    }
}

impl BalancingAlgorithm for LeastConnections {
    fn select(&self, backends: &[Arc<Backend>], _key: Option<&str>) -> Option<Arc<Backend>> {
        backends.iter()
            .min_by_key(|b| b.connections())
            .cloned()
    }
}

/// Weighted round-robin algorithm
pub struct WeightedRoundRobin {
    counter: AtomicUsize,
}

impl WeightedRoundRobin {
    pub fn new() -> Self {
        Self {
            counter: AtomicUsize::new(0),
        }
    }
}

impl Default for WeightedRoundRobin {
    fn default() -> Self {
        Self::new()
    }
}

impl BalancingAlgorithm for WeightedRoundRobin {
    fn select(&self, backends: &[Arc<Backend>], _key: Option<&str>) -> Option<Arc<Backend>> {
        if backends.is_empty() {
            return None;
        }

        // Calculate total weight
        let total_weight: u32 = backends.iter().map(|b| b.weight).sum();
        if total_weight == 0 {
            return None;
        }

        // Select based on weight
        let mut target = (self.counter.fetch_add(1, Ordering::Relaxed) as u32) % total_weight;
        
        for backend in backends {
            if target < backend.weight {
                return Some(backend.clone());
            }
            target -= backend.weight;
        }

        Some(backends[0].clone())
    }
}

/// Consistent hashing algorithm
pub struct ConsistentHash {
    virtual_nodes: usize,
}

impl ConsistentHash {
    pub fn new(virtual_nodes: usize) -> Self {
        Self { virtual_nodes }
    }
}

impl Default for ConsistentHash {
    fn default() -> Self {
        Self::new(150)
    }
}

impl BalancingAlgorithm for ConsistentHash {
    fn select(&self, backends: &[Arc<Backend>], key: Option<&str>) -> Option<Arc<Backend>> {
        if backends.is_empty() {
            return None;
        }

        let key = key.unwrap_or("default");
        let hash = Self::hash_key(key);

        // Find the backend with the closest hash
        backends.iter()
            .min_by_key(|b| {
                let backend_hash = Self::hash_key(&format!("{}:{}", b.id, hash % (self.virtual_nodes as u64)));
                hash.wrapping_sub(backend_hash)
            })
            .cloned()
    }
}

impl ConsistentHash {
    fn hash_key(key: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_backends() -> Vec<Arc<Backend>> {
        vec![
            Arc::new(Backend::new("backend1", "127.0.0.1:8001", 1)),
            Arc::new(Backend::new("backend2", "127.0.0.1:8002", 2)),
            Arc::new(Backend::new("backend3", "127.0.0.1:8003", 1)),
        ]
    }

    #[test]
    fn test_round_robin() {
        let backends = create_backends();
        let rr = RoundRobin::new();

        let b1 = rr.select(&backends, None).unwrap();
        let b2 = rr.select(&backends, None).unwrap();
        let b3 = rr.select(&backends, None).unwrap();
        let b4 = rr.select(&backends, None).unwrap();

        assert_eq!(b1.id, "backend1");
        assert_eq!(b2.id, "backend2");
        assert_eq!(b3.id, "backend3");
        assert_eq!(b4.id, "backend1"); // Wraps around
    }

    #[test]
    fn test_least_connections() {
        let backends = create_backends();
        let lc = LeastConnections::new();

        backends[1].inc_connections();
        backends[1].inc_connections();

        let selected = lc.select(&backends, None).unwrap();
        assert!(selected.id == "backend1" || selected.id == "backend3");
    }

    #[test]
    fn test_consistent_hash() {
        let backends = create_backends();
        let ch = ConsistentHash::new(100);

        let b1 = ch.select(&backends, Some("user123")).unwrap();
        let b2 = ch.select(&backends, Some("user123")).unwrap();

        // Same key should always return same backend
        assert_eq!(b1.id, b2.id);
    }
}
