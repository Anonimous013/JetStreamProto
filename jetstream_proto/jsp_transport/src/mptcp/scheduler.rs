//! MPTCP Scheduler

use super::{Subflow, SchedulerAlgorithm};
use std::sync::Arc;

pub trait Scheduler: Send + Sync {
    fn select_subflow<'a>(&self, subflows: &'a [Arc<Subflow>]) -> Option<&'a Arc<Subflow>>;
}

pub struct MinRttScheduler;

impl Scheduler for MinRttScheduler {
    fn select_subflow<'a>(&self, subflows: &'a [Arc<Subflow>]) -> Option<&'a Arc<Subflow>> {
        subflows.iter()
            .min_by_key(|s| s.rtt)
    }
}

pub struct RoundRobinScheduler {
    counter: std::sync::atomic::AtomicUsize,
}

impl RoundRobinScheduler {
    pub fn new() -> Self {
        Self {
            counter: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

impl Scheduler for RoundRobinScheduler {
    fn select_subflow<'a>(&self, subflows: &'a [Arc<Subflow>]) -> Option<&'a Arc<Subflow>> {
        if subflows.is_empty() {
            return None;
        }
        let idx = self.counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % subflows.len();
        Some(&subflows[idx])
    }
}

pub fn create_scheduler(algo: SchedulerAlgorithm) -> Box<dyn Scheduler> {
    match algo {
        SchedulerAlgorithm::MinRtt => Box::new(MinRttScheduler),
        SchedulerAlgorithm::RoundRobin => Box::new(RoundRobinScheduler::new()),
        SchedulerAlgorithm::Redundant => Box::new(MinRttScheduler), // Fallback
    }
}
