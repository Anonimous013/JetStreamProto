//! Multi-Hop Engine - Orchestrates the Hop Chain
//! 
//! Manages the lifecycle of the multi-hop tunnel chain, including initialization,
//! health monitoring, and failover.

use std::sync::Arc;
use tokio::sync::Mutex;
use crate::multihop::{
    config::MultiHopConfig,
    hop::{Hop, HopHealth},
    router::Router,
    buffer_pool::BufferPool,
    Result,
};

/// Multi-hop engine state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineState {
    /// Engine is not initialized
    Uninitialized,
    
    /// Engine is initializing hops
    Initializing,
    
    /// Engine is running
    Running,
    
    /// Engine is stopping
    Stopping,
    
    /// Engine has stopped
    Stopped,
    
    /// Engine is in error state
    Failed,
}

/// Multi-hop engine orchestrates the entire hop chain
pub struct MultiHopEngine {
    /// Configuration
    config: MultiHopConfig,
    
    /// Chain of hops
    hops: Vec<Arc<Mutex<dyn Hop>>>,
    
    /// Router for traffic management
    router: Option<Arc<Router>>,
    
    /// Buffer pool
    buffer_pool: Arc<BufferPool>,
    
    /// Current engine state
    state: Arc<Mutex<EngineState>>,
    
    /// Health monitoring task
    health_task: Option<tokio::task::JoinHandle<()>>,
}

impl MultiHopEngine {
    /// Create a new multi-hop engine with the given configuration
    pub fn new(config: MultiHopConfig) -> Self {
        let buffer_pool = Arc::new(BufferPool::new(100, 65536));
        
        Self {
            config,
            hops: Vec::new(),
            router: None,
            buffer_pool,
            state: Arc::new(Mutex::new(EngineState::Uninitialized)),
            health_task: None,
        }
    }
    
    /// Initialize the engine and start all hops
    pub async fn start(&mut self) -> Result<()> {
        *self.state.lock().await = EngineState::Initializing;
        
        tracing::info!("Starting multi-hop engine with {} hops", self.config.chain.len());
        
        // Build hop chain
        self.build_hop_chain().await?;
        
        // Start all hops in parallel
        let mut start_tasks = Vec::new();
        for (idx, hop) in self.hops.iter().enumerate() {
            let hop = Arc::clone(hop);
            start_tasks.push(tokio::spawn(async move {
                let mut hop_guard = hop.lock().await;
                let result = hop_guard.start().await;
                drop(hop_guard);
                (idx, result)
            }));
        }
        
        // Wait for all hops to start
        for task in start_tasks {
            let (idx, result) = task.await.map_err(|e| {
                crate::multihop::MultiHopError::Engine(format!("Hop start task failed: {}", e))
            })?;
            
            result.map_err(|e| {
                crate::multihop::MultiHopError::Engine(
                    format!("Failed to start hop {}: {}", idx, e)
                )
            })?;
            
            tracing::info!(hop_index = idx, "Hop started successfully");
        }
        
        // Create router
        self.router = Some(Arc::new(Router::new(
            self.hops.clone(),
            Arc::clone(&self.buffer_pool),
        )));
        
        *self.state.lock().await = EngineState::Running;
        
        // Start health monitoring
        self.start_health_monitoring();
        
        tracing::info!("Multi-hop engine started successfully");
        
        Ok(())
    }
    
    /// Stop the engine and all hops
    pub async fn stop(&mut self) -> Result<()> {
        *self.state.lock().await = EngineState::Stopping;
        
        tracing::info!("Stopping multi-hop engine");
        
        // Stop health monitoring
        if let Some(task) = self.health_task.take() {
            task.abort();
        }
        
        // Stop all hops
        for (idx, hop) in self.hops.iter().enumerate() {
            let mut hop_guard = hop.lock().await;
            if let Err(e) = hop_guard.stop().await {
                tracing::warn!(hop_index = idx, error = %e, "Failed to stop hop");
            }
        }
        
        *self.state.lock().await = EngineState::Stopped;
        
        tracing::info!("Multi-hop engine stopped");
        
        Ok(())
    }
    
    /// Send data through the multi-hop chain
    pub async fn send(&self, data: &[u8]) -> Result<()> {
        let state = *self.state.lock().await;
        if state != EngineState::Running {
            return Err(crate::multihop::MultiHopError::Engine(
                format!("Engine not running (state: {:?})", state)
            ));
        }
        
        let router = self.router.as_ref().ok_or_else(|| {
            crate::multihop::MultiHopError::Engine("Router not initialized".to_string())
        })?;
        
        router.route_outbound(data).await
    }
    
    /// Receive data from the multi-hop chain
    pub async fn recv(&self, buf: &mut [u8]) -> Result<usize> {
        let state = *self.state.lock().await;
        if state != EngineState::Running {
            return Err(crate::multihop::MultiHopError::Engine(
                format!("Engine not running (state: {:?})", state)
            ));
        }
        
        let router = self.router.as_ref().ok_or_else(|| {
            crate::multihop::MultiHopError::Engine("Router not initialized".to_string())
        })?;
        
        router.route_inbound(buf).await
    }
    
    /// Get current engine state
    pub async fn state(&self) -> EngineState {
        *self.state.lock().await
    }
    
    /// Get router reference
    pub fn router(&self) -> Option<Arc<Router>> {
        self.router.clone()
    }
    
    /// Build the hop chain from configuration
    async fn build_hop_chain(&mut self) -> Result<()> {
        use crate::multihop::config::HopConfig;
        
        for (idx, hop_config) in self.config.chain.iter().enumerate() {
            let hop: Arc<Mutex<dyn Hop>> = match hop_config {
                HopConfig::WireGuard(cfg) => {
                    let wg_hop = crate::multihop::hop_wireguard::WireGuardHop::new(cfg.clone(), false)?;
                    Arc::new(Mutex::new(wg_hop))
                }
                HopConfig::WireGuardExit(cfg) => {
                    let wg_hop = crate::multihop::hop_wireguard::WireGuardHop::new(cfg.clone(), true)?;
                    Arc::new(Mutex::new(wg_hop))
                }
                HopConfig::Shadowsocks(cfg) => {
                    let ss_hop = crate::multihop::hop_shadowsocks::ShadowsocksHop::new(cfg.clone())?;
                    Arc::new(Mutex::new(ss_hop))
                }
                HopConfig::XRay(cfg) => {
                    let xray_hop = crate::multihop::hop_xray::XRayHop::new(cfg.clone())?;
                    Arc::new(Mutex::new(xray_hop))
                }
            };
            
            self.hops.push(hop);
            tracing::debug!(hop_index = idx, hop_type = hop_config.hop_type(), "Hop added to chain");
        }
        
        Ok(())
    }
    
    /// Start health monitoring task
    fn start_health_monitoring(&mut self) {
        let hops = self.hops.clone();
        let state = Arc::clone(&self.state);
        let interval = std::time::Duration::from_secs(self.config.health_check_interval_secs);
        let auto_failover = self.config.auto_failover;
        
        let task = tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            interval_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            
            loop {
                interval_timer.tick().await;
                
                let current_state = *state.lock().await;
                if current_state != EngineState::Running {
                    break;
                }
                
                // Check health of all hops
                for (idx, hop) in hops.iter().enumerate() {
                    let hop_guard = hop.lock().await;
                    match hop_guard.health_check().await {
                        Ok(HopHealth::Healthy) => {
                            tracing::trace!(hop_index = idx, "Hop health check: healthy");
                        }
                        Ok(HopHealth::Degraded) => {
                            tracing::warn!(hop_index = idx, "Hop health check: degraded");
                        }
                        Ok(HopHealth::Unhealthy) => {
                            tracing::error!(hop_index = idx, "Hop health check: unhealthy");
                            
                            if auto_failover {
                                // TODO: Implement failover logic
                                tracing::warn!(hop_index = idx, "Auto-failover not yet implemented");
                            }
                        }
                        Ok(HopHealth::Unknown) => {
                            tracing::debug!(hop_index = idx, "Hop health check: unknown");
                        }
                        Err(e) => {
                            tracing::error!(hop_index = idx, error = %e, "Hop health check failed");
                        }
                    }
                }
            }
        });
        
        self.health_task = Some(task);
    }
}

impl Drop for MultiHopEngine {
    fn drop(&mut self) {
        if let Some(task) = self.health_task.take() {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_engine_lifecycle() {
        let config = MultiHopConfig {
            enabled: true,
            chain: vec![],
            hop_timeout_secs: 10,
            auto_failover: true,
            health_check_interval_secs: 30,
        };
        
        let mut engine = MultiHopEngine::new(config);
        
        assert_eq!(engine.state().await, EngineState::Uninitialized);
        
        // Note: Can't fully test start() without actual hop implementations
        // This would require mock hops
    }
}
