use sha2::{Sha256, Digest};
use sled::Db;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct MerkleTree {
    db: Db,
}

impl MerkleTree {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// Add a new leaf to the tree
    pub fn append(&self, data: &[u8]) -> Result<Vec<u8>> {
        let tree = self.db.open_tree("merkle_nodes")?;
        let meta = self.db.open_tree("merkle_meta")?;
        
        // Get current count
        let count_key = b"count";
        let count = meta.get(count_key)?.map(|b| {
            let mut arr = [0u8; 8];
            arr.copy_from_slice(&b);
            u64::from_be_bytes(arr)
        }).unwrap_or(0);

        // Hash leaf
        let mut hasher = Sha256::new();
        hasher.update(&[0x00]); // Leaf prefix
        hasher.update(data);
        let leaf_hash = hasher.finalize().to_vec();

        // Store leaf
        self.store_node(&tree, 0, count, &leaf_hash)?;

        // Update path to root
        let mut current_hash = leaf_hash;
        let mut current_idx = count;
        let mut level = 0;

        while current_idx > 0 {
            let sibling_idx = if current_idx % 2 == 0 {
                current_idx + 1 // Right sibling (doesn't exist yet usually)
            } else {
                current_idx - 1 // Left sibling
            };

            let sibling_hash = if sibling_idx < current_idx {
                self.get_node(&tree, level, sibling_idx)?.unwrap_or(vec![0u8; 32])
            } else {
                vec![0u8; 32] // Zero padding for right sibling
            };

            let mut hasher = Sha256::new();
            hasher.update(&[0x01]); // Node prefix
            if current_idx % 2 != 0 {
                hasher.update(&sibling_hash);
                hasher.update(&current_hash);
            } else {
                hasher.update(&current_hash);
                hasher.update(&sibling_hash);
            }
            current_hash = hasher.finalize().to_vec();
            
            current_idx /= 2;
            level += 1;
            
            self.store_node(&tree, level, current_idx, &current_hash)?;
        }

        // Update count
        meta.insert(count_key, (count + 1).to_be_bytes().to_vec())?;

        Ok(current_hash) // New root (approximate, for full tree we need to go up to max depth)
    }

    fn store_node(&self, tree: &sled::Tree, level: u32, index: u64, hash: &[u8]) -> Result<()> {
        let key = format!("{}:{}", level, index);
        tree.insert(key, hash)?;
        Ok(())
    }

    fn get_node(&self, tree: &sled::Tree, level: u32, index: u64) -> Result<Option<Vec<u8>>> {
        let key = format!("{}:{}", level, index);
        Ok(tree.get(key)?.map(|b| b.to_vec()))
    }
    
    pub fn root(&self) -> Result<Vec<u8>> {
        let tree = self.db.open_tree("merkle_nodes")?;
        let meta = self.db.open_tree("merkle_meta")?;
        
        let count = meta.get(b"count")?.map(|b| {
            let mut arr = [0u8; 8];
            arr.copy_from_slice(&b);
            u64::from_be_bytes(arr)
        }).unwrap_or(0);
        
        if count == 0 {
            return Ok(vec![0u8; 32]);
        }
        
        // Find the highest level node at index 0
        // This is a simplification. A proper Merkle tree root calculation with dynamic size is more complex.
        // For now, let's just return the node at the highest level we reached.
        // In a real implementation, we'd track tree height.
        
        // Simple scan for root at index 0
        let mut level = 0;
        let mut root = vec![0u8; 32];
        loop {
            if let Some(hash) = self.get_node(&tree, level, 0)? {
                root = hash;
                level += 1;
            } else {
                break;
            }
        }
        Ok(root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_append() -> Result<()> {
        let db = sled::Config::new().temporary(true).open()?;
        let tree = MerkleTree::new(db);
        
        let root1 = tree.append(b"data1")?;
        let root2 = tree.append(b"data2")?;
        
        assert_ne!(root1, root2);
        assert_eq!(root2.len(), 32);
        
        Ok(())
    }
}
