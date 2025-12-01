use sled::Db;
use anyhow::Result;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
pub struct MessageQueue {
    db: Db,
}

impl MessageQueue {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// Enqueue a message to a specific topic
    pub fn enqueue<T: Serialize>(&self, topic: &str, message: &T) -> Result<u64> {
        let tree = self.db.open_tree(topic)?;
        let _id = self.db.generate_id()?; // Global ID, but we can use it for unique ordering if we want, or manage our own counters
        
        // Better approach for FIFO: use a counter for tail
        let tail_key = b"meta:tail";
        let tail_idx = tree.update_and_fetch(tail_key, |old| {
            let num = old.map(|b| {
                let mut arr = [0u8; 8];
                arr.copy_from_slice(b);
                u64::from_be_bytes(arr)
            }).unwrap_or(0);
            Some((num + 1).to_be_bytes().to_vec())
        })?.map(|b| {
            let mut arr = [0u8; 8];
            arr.copy_from_slice(&b);
            u64::from_be_bytes(arr)
        }).unwrap_or(0);

        let bytes = bincode::serialize(message)?;
        tree.insert(&tail_idx.to_be_bytes(), bytes)?;
        
        Ok(tail_idx)
    }

    /// Dequeue a message from a specific topic
    pub fn dequeue<T: for<'a> Deserialize<'a>>(&self, topic: &str) -> Result<Option<T>> {
        let tree = self.db.open_tree(topic)?;
        
        // Get head index
        let head_key = b"meta:head";
        let head_idx = tree.get(head_key)?.map(|b| {
            let mut arr = [0u8; 8];
            arr.copy_from_slice(&b);
            u64::from_be_bytes(arr)
        }).unwrap_or(0);

        // Try to get the item at head_idx + 1 (since we increment tail before insert, indices are 1-based)
        // Wait, if we start at 0, and increment to 1 for first item.
        // Let's say tail starts at 0.
        // Enqueue: increment tail to 1. Insert at 1.
        // Head starts at 0.
        // Dequeue: check head+1 (1). If exists, return and increment head to 1.
        
        let target_idx = head_idx + 1;
        
        if let Some(bytes) = tree.remove(&target_idx.to_be_bytes())? {
            // Update head
            tree.insert(head_key, &target_idx.to_be_bytes())?;
            let msg = bincode::deserialize(&bytes)?;
            Ok(Some(msg))
        } else {
            Ok(None)
        }
    }

    /// Peek at the next message without removing it
    pub fn peek<T: for<'a> Deserialize<'a>>(&self, topic: &str) -> Result<Option<T>> {
        let tree = self.db.open_tree(topic)?;
        let head_key = b"meta:head";
        let head_idx = tree.get(head_key)?.map(|b| {
            let mut arr = [0u8; 8];
            arr.copy_from_slice(&b);
            u64::from_be_bytes(arr)
        }).unwrap_or(0);

        let target_idx = head_idx + 1;
        
        if let Some(bytes) = tree.get(&target_idx.to_be_bytes())? {
            let msg = bincode::deserialize(&bytes)?;
            Ok(Some(msg))
        } else {
            Ok(None)
        }
    }
    
    /// Get queue length
    pub fn len(&self, topic: &str) -> Result<u64> {
        let tree = self.db.open_tree(topic)?;
        let head_key = b"meta:head";
        let tail_key = b"meta:tail";
        
        let head = tree.get(head_key)?.map(|b| {
            let mut arr = [0u8; 8];
            arr.copy_from_slice(&b);
            u64::from_be_bytes(arr)
        }).unwrap_or(0);
        
        let tail = tree.get(tail_key)?.map(|b| {
            let mut arr = [0u8; 8];
            arr.copy_from_slice(&b);
            u64::from_be_bytes(arr)
        }).unwrap_or(0);
        
        if tail >= head {
            Ok(tail - head)
        } else {
            Ok(0) // Should not happen
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_fifo() -> Result<()> {
        let db = sled::Config::new().temporary(true).open()?;
        let queue = MessageQueue::new(db);
        
        queue.enqueue("chat", &"msg1".to_string())?;
        queue.enqueue("chat", &"msg2".to_string())?;
        
        assert_eq!(queue.len("chat")?, 2);
        
        let m1: Option<String> = queue.peek("chat")?;
        assert_eq!(m1, Some("msg1".to_string()));
        
        let m1: Option<String> = queue.dequeue("chat")?;
        assert_eq!(m1, Some("msg1".to_string()));
        
        let m2: Option<String> = queue.dequeue("chat")?;
        assert_eq!(m2, Some("msg2".to_string()));
        
        let m3: Option<String> = queue.dequeue("chat")?;
        assert!(m3.is_none());
        
        Ok(())
    }
}
