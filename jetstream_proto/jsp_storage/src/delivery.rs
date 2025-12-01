use sled::Db;
use anyhow::Result;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryGuarantee {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageState {
    Pending,
    Delivered,
    Acked,
    Failed,
}

#[derive(Debug, Clone)]
pub struct DeliveryManager {
    db: Db,
}

impl DeliveryManager {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// Mark a message as processed (for ExactlyOnce deduplication)
    /// Returns true if the message was new, false if it was already processed
    pub fn mark_processed(&self, message_id: u64) -> Result<bool> {
        let tree = self.db.open_tree("processed_messages")?;
        let key = message_id.to_be_bytes();
        
        if tree.contains_key(&key)? {
            Ok(false)
        } else {
            tree.insert(&key, &[])?;
            Ok(true)
        }
    }

    /// Update message state
    pub fn update_state(&self, message_id: u64, state: MessageState) -> Result<()> {
        let tree = self.db.open_tree("message_states")?;
        let key = message_id.to_be_bytes();
        let bytes = bincode::serialize(&state)?;
        tree.insert(&key, bytes)?;
        Ok(())
    }

    /// Get message state
    pub fn get_state(&self, message_id: u64) -> Result<Option<MessageState>> {
        let tree = self.db.open_tree("message_states")?;
        let key = message_id.to_be_bytes();
        
        match tree.get(&key)? {
            Some(bytes) => {
                let state = bincode::deserialize(&bytes)?;
                Ok(Some(state))
            },
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deduplication() -> Result<()> {
        let db = sled::Config::new().temporary(true).open()?;
        let manager = DeliveryManager::new(db);
        
        assert!(manager.mark_processed(1)?);
        assert!(!manager.mark_processed(1)?);
        assert!(manager.mark_processed(2)?);
        
        Ok(())
    }

    #[test]
    fn test_state_tracking() -> Result<()> {
        let db = sled::Config::new().temporary(true).open()?;
        let manager = DeliveryManager::new(db);
        
        manager.update_state(100, MessageState::Pending)?;
        assert_eq!(manager.get_state(100)?, Some(MessageState::Pending));
        
        manager.update_state(100, MessageState::Acked)?;
        assert_eq!(manager.get_state(100)?, Some(MessageState::Acked));
        
        assert!(manager.get_state(999)?.is_none());
        
        Ok(())
    }
}
