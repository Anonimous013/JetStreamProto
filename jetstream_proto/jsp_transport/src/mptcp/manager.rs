//! MPTCP Manager

use super::{MptcpConfig, Subflow, InterfaceWatcher, Scheduler, create_scheduler};
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;

pub struct MptcpManager {
    config: MptcpConfig,
    subflows: Arc<Mutex<Vec<Arc<Subflow>>>>,
    watcher: InterfaceWatcher,
    scheduler: Box<dyn Scheduler>,
    remote_addr: SocketAddr,
}

impl MptcpManager {
    pub fn new(config: MptcpConfig, remote_addr: SocketAddr) -> Self {
        let scheduler = create_scheduler(config.scheduler_algo);
        
        Self {
            config,
            subflows: Arc::new(Mutex::new(Vec::new())),
            watcher: InterfaceWatcher::new(),
            scheduler,
            remote_addr,
        }
    }

    pub async fn start(&self) {
        if !self.config.enabled {
            return;
        }
        
        // Watch for interfaces and create subflows
        let mut rx = self.watcher.subscribe();
        let subflows = self.subflows.clone();
        let remote = self.remote_addr;
        
        tokio::spawn(async move {
            while let Ok(_) = rx.changed().await {
                let ifaces = rx.borrow().clone();
                // Simple logic: one subflow per non-loopback interface
                for iface in ifaces {
                    if iface.is_loopback { continue; }
                    
                    // Check if exists
                    let exists = {
                         let current_subflows = subflows.lock().unwrap();
                         current_subflows.iter().any(|s| s.local_addr.ip() == iface.ip)
                    };

                    if !exists {
                        let id = {
                            let current_subflows = subflows.lock().unwrap();
                            current_subflows.len() as u32
                        };
                        let local = SocketAddr::new(iface.ip, 0); // Ephemeral port
                        
                        match Subflow::new(id, local, remote).await {
                            Ok(subflow) => {
                                tracing::info!("Created new subflow on {}", iface.name);
                                let mut current_subflows = subflows.lock().unwrap();
                                current_subflows.push(Arc::new(subflow));
                            }
                            Err(e) => {
                                tracing::warn!("Failed to create subflow on {}: {}", iface.name, e);
                            }
                        }
                    }
                }
            }
        });
    }

    pub async fn send(&self, data: &[u8]) -> std::io::Result<()> {
        let subflows_lock = self.subflows.lock().unwrap();
        
        if let Some(subflow) = self.scheduler.select_subflow(&subflows_lock) {
            subflow.send(data).await.map(|_| ())
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::NotConnected, "No available subflows"))
        }
    }
}
