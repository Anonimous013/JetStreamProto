use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use crate::crdt::LWWRegister;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub registers: HashMap<String, LWWRegister<Vec<u8>>>,
    pub last_modified: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    pub changes: HashMap<String, LWWRegister<Vec<u8>>>,
}

pub struct DeltaSync {
    state: State,
}

impl DeltaSync {
    pub fn new() -> Self {
        Self {
            state: State {
                registers: HashMap::new(),
                last_modified: 0,
            },
        }
    }

    pub fn update(&mut self, key: String, value: Vec<u8>, timestamp: u64, node_id: String) {
        let register = LWWRegister::new(value, timestamp, node_id);
        self.state.registers.insert(key, register);
        if timestamp > self.state.last_modified {
            self.state.last_modified = timestamp;
        }
    }

    pub fn get_delta(&self, since: u64) -> Delta {
        let mut changes = HashMap::new();
        for (key, reg) in &self.state.registers {
            if reg.timestamp() > since {
                changes.insert(key.clone(), reg.clone());
            }
        }
        Delta { changes }
    }

    pub fn apply_delta(&mut self, delta: Delta) {
        for (key, reg) in delta.changes {
            if let Some(existing) = self.state.registers.get_mut(&key) {
                existing.merge(reg);
            } else {
                self.state.registers.insert(key, reg);
            }
        }
    }
}
