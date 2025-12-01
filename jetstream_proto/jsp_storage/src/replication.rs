use sled::Db;
use anyhow::Result;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaInfo {
    pub node_id: String,
    pub last_sync_index: u64,
    pub last_seen: u64,
}

#[derive(Debug, Clone)]
pub struct ReplicationManager {
    db: Db,
}

impl ReplicationManager {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// Register or update a replica
    pub fn update_replica(&self, node_id: &str, last_index: u64) -> Result<()> {
        let tree = self.db.open_tree("replicas")?;
        
        let info = ReplicaInfo {
            node_id: node_id.to_string(),
            last_sync_index: last_index,
            last_seen: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        };
        
        let bytes = bincode::serialize(&info)?;
        tree.insert(node_id, bytes)?;
        
        Ok(())
    }

    /// Get replica status
    pub fn get_replica(&self, node_id: &str) -> Result<Option<ReplicaInfo>> {
        let tree = self.db.open_tree("replicas")?;
        match tree.get(node_id)? {
            Some(bytes) => {
                let info = bincode::deserialize(&bytes)?;
                Ok(Some(info))
            },
            None => Ok(None),
        }
    }

    /// Get all replicas
    pub fn get_all_replicas(&self) -> Result<Vec<ReplicaInfo>> {
        let tree = self.db.open_tree("replicas")?;
        let mut replicas = Vec::new();
        
        for item in tree.iter() {
            let (_, value) = item?;
            let info: ReplicaInfo = bincode::deserialize(&value)?;
            replicas.push(info);
        }
        
        Ok(replicas)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replication_tracking() -> Result<()> {
        let db = sled::Config::new().temporary(true).open()?;
        let manager = ReplicationManager::new(db);
        
        manager.update_replica("node1", 100)?;
        
        let info = manager.get_replica("node1")?.unwrap();
        assert_eq!(info.last_sync_index, 100);
        
        manager.update_replica("node1", 150)?;
        let info = manager.get_replica("node1")?.unwrap();
        assert_eq!(info.last_sync_index, 150);
        
        Ok(())
    }
}
