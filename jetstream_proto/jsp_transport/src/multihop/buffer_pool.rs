//! Buffer Pool - Optimized Buffer Management
//! 
//! Pre-allocated buffer pool for zero-copy packet forwarding across hops.

use bytes::BytesMut;
use std::sync::Mutex;

/// Buffer pool for efficient memory management
pub struct BufferPool {
    /// Pool of available buffers
    pool: Mutex<Vec<BytesMut>>,
    
    /// Maximum number of buffers to keep
    capacity: usize,
    
    /// Size of each buffer
    buffer_size: usize,
}

impl BufferPool {
    /// Create a new buffer pool
    pub fn new(capacity: usize, buffer_size: usize) -> Self {
        let mut pool = Vec::with_capacity(capacity);
        
        // Pre-allocate buffers
        for _ in 0..capacity {
            pool.push(BytesMut::with_capacity(buffer_size));
        }
        
        Self {
            pool: Mutex::new(pool),
            capacity,
            buffer_size,
        }
    }
    
    /// Acquire a buffer from the pool
    pub fn acquire(&self, min_size: usize) -> BytesMut {
        let mut pool = self.pool.lock().unwrap();
        
        if let Some(mut buf) = pool.pop() {
            buf.clear();
            if buf.capacity() < min_size {
                buf.reserve(min_size - buf.capacity());
            }
            buf
        } else {
            // Pool exhausted, allocate new buffer
            BytesMut::with_capacity(min_size.max(self.buffer_size))
        }
    }
    
    /// Release a buffer back to the pool
    pub fn release(&self, mut buf: BytesMut) {
        let mut pool = self.pool.lock().unwrap();
        
        if pool.len() < self.capacity {
            buf.clear();
            pool.push(buf);
        }
        // Otherwise, let it drop
    }
    
    /// Get current pool size
    pub fn size(&self) -> usize {
        self.pool.lock().unwrap().len()
    }
    
    /// Get pool capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_pool() {
        let pool = BufferPool::new(5, 1024);
        
        assert_eq!(pool.size(), 5);
        assert_eq!(pool.capacity(), 5);
        
        // Acquire buffer
        let buf1 = pool.acquire(512);
        assert_eq!(pool.size(), 4);
        assert!(buf1.capacity() >= 512);
        
        // Release buffer
        pool.release(buf1);
        assert_eq!(pool.size(), 5);
        
        // Acquire multiple buffers
        let buf2 = pool.acquire(1024);
        let buf3 = pool.acquire(2048);
        assert_eq!(pool.size(), 3);
        assert!(buf3.capacity() >= 2048);
        
        pool.release(buf2);
        pool.release(buf3);
        assert_eq!(pool.size(), 5);
    }
    
    #[test]
    fn test_buffer_pool_exhaustion() {
        let pool = BufferPool::new(2, 1024);
        
        let buf1 = pool.acquire(512);
        let buf2 = pool.acquire(512);
        assert_eq!(pool.size(), 0);
        
        // Pool exhausted, should allocate new buffer
        let buf3 = pool.acquire(512);
        assert_eq!(pool.size(), 0);
        
        pool.release(buf1);
        pool.release(buf2);
        pool.release(buf3);
        
        // Only capacity buffers are kept
        assert_eq!(pool.size(), 2);
    }
}
