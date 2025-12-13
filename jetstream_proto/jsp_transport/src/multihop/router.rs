//! Router - Traffic Routing Between Hops
//! 
//! Manages packet forwarding through the multi-hop chain with zero-copy
//! optimization and flow control.

use std::sync::Arc;
use tokio::sync::Mutex;
use crate::multihop::{Hop, Result};

/// Router manages traffic flow through the hop chain
pub struct Router {
    /// Chain of hops
    hops: Vec<Arc<Mutex<dyn Hop>>>,
    
    /// Buffer pool for zero-copy forwarding
    buffer_pool: Arc<crate::multihop::buffer_pool::BufferPool>,
    
    /// Per-hop metrics
    metrics: Vec<Arc<crate::multihop::metrics::HopMetrics>>,
}

impl Router {
    /// Create a new router with the given hop chain
    pub fn new(
        hops: Vec<Arc<Mutex<dyn Hop>>>,
        buffer_pool: Arc<crate::multihop::buffer_pool::BufferPool>,
    ) -> Self {
        let metrics = hops.iter()
            .map(|_| Arc::new(crate::multihop::metrics::HopMetrics::new()))
            .collect();
        
        Self {
            hops,
            buffer_pool,
            metrics,
        }
    }
    
    /// Route data through the entire hop chain
    /// 
    /// Data flows: App → Hop0 → Hop1 → ... → HopN → Internet
    pub async fn route_outbound(&self, data: &[u8]) -> Result<()> {
        if self.hops.is_empty() {
            return Err(crate::multihop::MultiHopError::Routing(
                "No hops configured".to_string()
            ));
        }
        
        // Get buffer from pool
        let mut buffer = self.buffer_pool.acquire(data.len());
        buffer.extend_from_slice(data);
        
        // Route through each hop
        for (idx, hop) in self.hops.iter().enumerate() {
            let start = std::time::Instant::now();
            
            let hop_guard = hop.lock().await;
            let sent = hop_guard.send(&buffer).await?;
            drop(hop_guard);
            
            let latency = start.elapsed();
            
            // Update metrics
            self.metrics[idx].record_sent(sent, latency);
            
            tracing::trace!(
                hop_index = idx,
                hop_type = self.hops[idx].lock().await.hop_type(),
                bytes = sent,
                latency_ms = latency.as_millis(),
                "Routed packet through hop"
            );
        }
        
        // Return buffer to pool
        self.buffer_pool.release(buffer);
        
        Ok(())
    }
    
    /// Route data back from the hop chain
    /// 
    /// Data flows: Internet → HopN → ... → Hop1 → Hop0 → App
    pub async fn route_inbound(&self, buf: &mut [u8]) -> Result<usize> {
        if self.hops.is_empty() {
            return Err(crate::multihop::MultiHopError::Routing(
                "No hops configured".to_string()
            ));
        }
        
        let mut total_received = 0;
        
        // Route through hops in reverse order
        for (idx, hop) in self.hops.iter().enumerate().rev() {
            let start = std::time::Instant::now();
            
            let hop_guard = hop.lock().await;
            let received = hop_guard.recv(buf).await?;
            drop(hop_guard);
            
            let latency = start.elapsed();
            
            // Update metrics
            self.metrics[idx].record_received(received, latency);
            
            total_received = received;
            
            tracing::trace!(
                hop_index = idx,
                hop_type = self.hops[idx].lock().await.hop_type(),
                bytes = received,
                latency_ms = latency.as_millis(),
                "Received packet from hop"
            );
        }
        
        Ok(total_received)
    }
    
    /// Get total latency across all hops
    pub async fn total_latency_ms(&self) -> f64 {
        let mut total = 0.0;
        for metric in &self.metrics {
            total += metric.avg_latency_ms();
        }
        total
    }
    
    /// Get total throughput (bytes/sec)
    pub async fn total_throughput(&self) -> u64 {
        // Use the minimum throughput of all hops (bottleneck)
        self.metrics.iter()
            .map(|m| m.throughput_bps())
            .min()
            .unwrap_or(0)
    }
    
    /// Get metrics for a specific hop
    pub fn hop_metrics(&self, hop_index: usize) -> Option<Arc<crate::multihop::metrics::HopMetrics>> {
        self.metrics.get(hop_index).cloned()
    }
    
    /// Get number of hops in the chain
    pub fn hop_count(&self) -> usize {
        self.hops.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Mock hop for testing
    struct MockHop {
        hop_type: String,
        stats: crate::multihop::hop::HopStats,
    }
    
    #[async_trait::async_trait]
    impl Hop for MockHop {
        async fn start(&mut self) -> Result<()> { Ok(()) }
        async fn stop(&mut self) -> Result<()> { Ok(()) }
        async fn health_check(&self) -> Result<crate::multihop::hop::HopHealth> {
            Ok(crate::multihop::hop::HopHealth::Healthy)
        }
        async fn send(&self, data: &[u8]) -> Result<usize> { Ok(data.len()) }
        async fn recv(&self, buf: &mut [u8]) -> Result<usize> { Ok(buf.len()) }
        fn local_endpoint(&self) -> std::net::SocketAddr { "127.0.0.1:0".parse().unwrap() }
        fn remote_endpoint(&self) -> std::net::SocketAddr { "127.0.0.1:0".parse().unwrap() }
        fn status(&self) -> crate::multihop::hop::HopStatus {
            crate::multihop::hop::HopStatus::Running
        }
        fn stats(&self) -> &crate::multihop::hop::HopStats { &self.stats }
        fn hop_type(&self) -> &str { &self.hop_type }
    }
    
    #[tokio::test]
    async fn test_router_basic() {
        let buffer_pool = Arc::new(crate::multihop::buffer_pool::BufferPool::new(10, 65536));
        
        let hop1: Arc<Mutex<dyn Hop>> = Arc::new(Mutex::new(MockHop {
            hop_type: "test1".to_string(),
            stats: crate::multihop::hop::HopStats::new(),
        }));
        
        let hop2: Arc<Mutex<dyn Hop>> = Arc::new(Mutex::new(MockHop {
            hop_type: "test2".to_string(),
            stats: crate::multihop::hop::HopStats::new(),
        }));
        
        let router = Router::new(vec![hop1, hop2], buffer_pool);
        
        assert_eq!(router.hop_count(), 2);
        
        let data = b"test data";
        let result = router.route_outbound(data).await;
        assert!(result.is_ok());
    }
}
