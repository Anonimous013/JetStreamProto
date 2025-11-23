use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy)]
pub enum Strategy {
    RoundRobin,
    // LeastConnections would require tracking active connections count
}

#[derive(Debug)]
pub struct LoadBalancer {
    backends: Arc<RwLock<Vec<SocketAddr>>>,
    strategy: Strategy,
    // Round Robin counter
    rr_counter: AtomicUsize,
}

impl LoadBalancer {
    pub fn new(backends: Vec<SocketAddr>, strategy: Strategy) -> Self {
        Self {
            backends: Arc::new(RwLock::new(backends)),
            strategy,
            rr_counter: AtomicUsize::new(0),
        }
    }

    pub async fn select_backend(&self, _client_addr: SocketAddr) -> SocketAddr {
        let backends = self.backends.read().await;
        if backends.is_empty() {
            // Fallback or error? For now panic or return a dummy
            panic!("No backends available");
        }

        match self.strategy {
            Strategy::RoundRobin => {
                let idx = self.rr_counter.fetch_add(1, Ordering::Relaxed);
                backends[idx % backends.len()]
            }
        }
    }

    pub async fn add_backend(&self, addr: SocketAddr) {
        let mut backends = self.backends.write().await;
        if !backends.contains(&addr) {
            backends.push(addr);
            tracing::info!("Backend added: {}", addr);
        }
    }

    pub async fn remove_backend(&self, addr: SocketAddr) {
        let mut backends = self.backends.write().await;
        if let Some(pos) = backends.iter().position(|x| *x == addr) {
            backends.remove(pos);
            tracing::info!("Backend removed: {}", addr);
        }
    }
}
