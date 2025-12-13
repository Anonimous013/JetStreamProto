//! Interface Watcher
//! 
//! Detects available network interfaces.

use std::net::IpAddr;
use std::time::Duration;
use tokio::sync::watch;

#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub ip: IpAddr,
    pub is_loopback: bool,
}

pub struct InterfaceWatcher {
    #[allow(dead_code)]
    stop_tx: watch::Sender<bool>,
    interfaces: watch::Receiver<Vec<NetworkInterface>>,
}

impl InterfaceWatcher {
    pub fn new() -> Self {
        let (stop_tx, mut stop_rx) = watch::channel(false);
        let (iface_tx, iface_rx) = watch::channel(Vec::new());

        // Spawn watcher task
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Scan interfaces
                        let ifaces = Self::scan_interfaces();
                        let _ = iface_tx.send(ifaces);
                    }
                    _ = stop_rx.changed() => {
                        break;
                    }
                }
            }
        });

        Self {
            stop_tx,
            interfaces: iface_rx,
        }
    }

    pub fn subscribe(&self) -> watch::Receiver<Vec<NetworkInterface>> {
        self.interfaces.clone()
    }

    fn scan_interfaces() -> Vec<NetworkInterface> {
        let mut interfaces = Vec::new();

        match if_addrs::get_if_addrs() {
            Ok(ifaces) => {
                for iface in ifaces {
                     // Filter out loopback if likely not needed, but here we keep logic generic
                     // We only care about IP addresses (v4/v6)
                     let is_loopback = iface.is_loopback();
                     let ip = iface.addr.ip();
                     
                     // Optional: Filter out link-local ipv6 or other non-routable if needed
                     // For now, accept all
                     
                    interfaces.push(NetworkInterface {
                        name: iface.name,
                        ip,
                        is_loopback,
                    });
                }
            }
            Err(e) => {
                tracing::error!("Failed to scan interfaces: {}", e);
            }
        }
        
        interfaces
    }
}
