//! Controller - Management API for Multi-Hop Engine
//! 
//! Provides runtime control and monitoring capabilities for DevBook integration.

use std::sync::Arc;
use tokio::sync::Mutex;
use crate::multihop::MultiHopEngine;

/// Controller for managing multi-hop engine at runtime
pub struct Controller {
    /// Reference to the engine
    engine: Arc<Mutex<MultiHopEngine>>,
}

impl Controller {
    /// Create a new controller for the given engine
    pub fn new(engine: Arc<Mutex<MultiHopEngine>>) -> Self {
        Self { engine }
    }
    
    /// Get engine status
    pub async fn status(&self) -> String {
        let engine = self.engine.lock().await;
        format!("{:?}", engine.state().await)
    }
    
    /// Get hop count
    pub async fn hop_count(&self) -> usize {
        let engine = self.engine.lock().await;
        if let Some(router) = engine.router() {
            router.hop_count()
        } else {
            0
        }
    }
    
    /// Get total latency
    pub async fn total_latency_ms(&self) -> f64 {
        let engine = self.engine.lock().await;
        if let Some(router) = engine.router() {
            router.total_latency_ms().await
        } else {
            0.0
        }
    }
    
    /// Get total throughput
    pub async fn total_throughput_bps(&self) -> u64 {
        let engine = self.engine.lock().await;
        if let Some(router) = engine.router() {
            router.total_throughput().await
        } else {
            0
        }
    }
}
