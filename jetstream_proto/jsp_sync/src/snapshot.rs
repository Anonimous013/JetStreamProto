use serde::{Serialize, Deserialize};
use crate::delta::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub state: State,
    pub version: u64,
}

pub struct SnapshotSync {
    // In a real system, this would likely wrap the same state storage as DeltaSync
    // or interact with the storage layer.
}

impl SnapshotSync {
    pub fn create_snapshot(state: &State) -> Snapshot {
        Snapshot {
            state: state.clone(),
            version: state.last_modified,
        }
    }

    pub fn apply_snapshot(current_state: &mut State, snapshot: Snapshot) {
        // For LWW registers, we can just merge all registers from the snapshot
        for (key, reg) in snapshot.state.registers {
            if let Some(existing) = current_state.registers.get_mut(&key) {
                existing.merge(reg);
            } else {
                current_state.registers.insert(key, reg);
            }
        }
        
        if snapshot.version > current_state.last_modified {
            current_state.last_modified = snapshot.version;
        }
    }
}
