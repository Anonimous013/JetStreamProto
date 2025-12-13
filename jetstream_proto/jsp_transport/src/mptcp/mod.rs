//! Multi-path TCP Support
//! 
//! Provides MPTCP capabilities by managing multiple subflows over different network interfaces.

pub mod watcher;
pub mod subflow;
pub mod manager;
pub mod scheduler;

pub use watcher::InterfaceWatcher;
pub use subflow::Subflow;
pub use manager::MptcpManager;

pub use scheduler::{Scheduler, create_scheduler};

/// MPTCP Configuration
#[derive(Debug, Clone)]
pub struct MptcpConfig {
    pub enabled: bool,
    pub max_subflows: usize,
    pub scheduler_algo: SchedulerAlgorithm,
}

impl Default for MptcpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_subflows: 4,
            scheduler_algo: SchedulerAlgorithm::MinRtt,
        }
    }
}

/// Scheduling algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerAlgorithm {
    RoundRobin,
    MinRtt,
    Redundant, // Send on all
}
