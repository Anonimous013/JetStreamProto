use std::sync::{Arc, Mutex};

/// A simple memory pool for reusable byte buffers
/// 
/// This pool reduces allocations by reusing Vec<u8> buffers for packet sending/receiving.
/// Buffers are pre-allocated to a maximum size and returned to the pool when no longer needed.
#[derive(Debug, Clone)]
pub struct PacketPool {
    inner: Arc<Mutex<PacketPoolInner>>,
}

#[derive(Debug)]
struct PacketPoolInner {
    /// Pool of available buffers
    pool: Vec<Vec<u8>>,
    /// Maximum number of buffers to keep in pool
    max_capacity: usize,
    /// Maximum size of each buffer
    max_packet_size: usize,
    /// Metrics
    metrics: PoolMetrics,
}

/// Metrics for monitoring pool performance
#[derive(Debug, Clone, Default)]
pub struct PoolMetrics {
    /// Total number of buffers acquired
    pub total_acquired: u64,
    /// Total number of buffers released back to pool
    pub total_released: u64,
    /// Total number of buffers allocated (not from pool)
    pub total_allocated: u64,
    /// Current number of buffers in pool
    pub current_pool_size: usize,
}

impl PacketPool {
    /// Create a new packet pool
    /// 
    /// # Arguments
    /// * `max_capacity` - Maximum number of buffers to keep in pool
    /// * `max_packet_size` - Maximum size of each buffer in bytes
    pub fn new(max_capacity: usize, max_packet_size: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(PacketPoolInner {
                pool: Vec::with_capacity(max_capacity),
                max_capacity,
                max_packet_size,
                metrics: PoolMetrics::default(),
            })),
        }
    }

    /// Acquire a buffer from the pool
    /// 
    /// If the pool is empty, a new buffer is allocated.
    /// The returned buffer is cleared and ready to use.
    pub fn acquire(&self) -> Vec<u8> {
        let mut inner = self.inner.lock().unwrap();
        inner.metrics.total_acquired += 1;

        if let Some(mut buffer) = inner.pool.pop() {
            buffer.clear();
            inner.metrics.current_pool_size = inner.pool.len();
            buffer
        } else {
            inner.metrics.total_allocated += 1;
            Vec::with_capacity(inner.max_packet_size)
        }
    }

    /// Release a buffer back to the pool
    /// 
    /// If the pool is at capacity, the buffer is dropped.
    /// Buffers larger than max_packet_size are also dropped to prevent memory bloat.
    pub fn release(&self, mut buffer: Vec<u8>) {
        let mut inner = self.inner.lock().unwrap();
        inner.metrics.total_released += 1;

        // Only keep buffers that are within size limits
        if inner.pool.len() < inner.max_capacity && buffer.capacity() <= inner.max_packet_size {
            buffer.clear();
            inner.pool.push(buffer);
            inner.metrics.current_pool_size = inner.pool.len();
        }
        // Otherwise, buffer is dropped
    }

    /// Get current pool metrics
    pub fn metrics(&self) -> PoolMetrics {
        let inner = self.inner.lock().unwrap();
        inner.metrics.clone()
    }

    /// Get pool hit rate (percentage of acquires served from pool)
    pub fn hit_rate(&self) -> f64 {
        let metrics = self.metrics();
        if metrics.total_acquired == 0 {
            return 0.0;
        }
        let hits = metrics.total_acquired - metrics.total_allocated;
        (hits as f64 / metrics.total_acquired as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_acquire_release() {
        let pool = PacketPool::new(10, 1500);

        // Acquire a buffer
        let buffer = pool.acquire();
        assert_eq!(buffer.len(), 0);

        // Release it back
        pool.release(buffer);

        // Metrics should reflect this
        let metrics = pool.metrics();
        assert_eq!(metrics.total_acquired, 1);
        assert_eq!(metrics.total_released, 1);
        assert_eq!(metrics.current_pool_size, 1);
    }

    #[test]
    fn test_pool_reuse() {
        let pool = PacketPool::new(10, 1500);

        // Acquire and release a buffer
        let mut buffer = pool.acquire();
        buffer.extend_from_slice(&[1, 2, 3, 4, 5]);
        pool.release(buffer);

        // Acquire again - should get the same buffer (cleared)
        let buffer2 = pool.acquire();
        assert_eq!(buffer2.len(), 0); // Should be cleared

        let metrics = pool.metrics();
        assert_eq!(metrics.total_acquired, 2);
        assert_eq!(metrics.total_allocated, 1); // Only one allocation
    }

    #[test]
    fn test_pool_capacity_limit() {
        let pool = PacketPool::new(2, 1500);

        // Acquire and release 3 buffers
        let b1 = pool.acquire();
        let b2 = pool.acquire();
        let b3 = pool.acquire();

        pool.release(b1);
        pool.release(b2);
        pool.release(b3); // This one should be dropped

        let metrics = pool.metrics();
        assert_eq!(metrics.current_pool_size, 2); // Max capacity
    }

    #[test]
    fn test_pool_size_limit() {
        let pool = PacketPool::new(10, 1000);

        // Create a buffer larger than max_packet_size
        let mut large_buffer = Vec::with_capacity(2000);
        large_buffer.extend_from_slice(&vec![0u8; 2000]);

        pool.release(large_buffer); // Should be dropped

        let metrics = pool.metrics();
        assert_eq!(metrics.current_pool_size, 0); // Not added to pool
    }

    #[test]
    fn test_hit_rate() {
        let pool = PacketPool::new(10, 1500);

        // First acquire - miss
        let b1 = pool.acquire();
        assert_eq!(pool.hit_rate(), 0.0);

        // Release and acquire again - hit
        pool.release(b1);
        let _b2 = pool.acquire();
        assert_eq!(pool.hit_rate(), 50.0); // 1 hit out of 2 acquires
    }

    #[test]
    fn test_concurrent_access() {
        use std::thread;

        let pool = PacketPool::new(100, 1500);
        let mut handles = vec![];

        // Spawn multiple threads
        for _ in 0..10 {
            let pool_clone = pool.clone();
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    let buffer = pool_clone.acquire();
                    pool_clone.release(buffer);
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Check metrics
        let metrics = pool.metrics();
        assert_eq!(metrics.total_acquired, 1000);
        assert_eq!(metrics.total_released, 1000);
    }
}
