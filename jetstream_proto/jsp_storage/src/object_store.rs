use sled::Db;
use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ObjectStore {
    db: Db,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionedObject<T> {
    pub version: u64,
    pub data: T,
    pub timestamp: u64,
}

impl ObjectStore {
    /// Open or create a new ObjectStore at the given path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled::open(path).context("Failed to open sled database")?;
        Ok(Self { db })
    }

    /// Open an in-memory ObjectStore (useful for tests)
    pub fn open_temporary() -> Result<Self> {
        let db = sled::Config::new().temporary(true).open().context("Failed to open temporary sled database")?;
        Ok(Self { db })
    }

    /// Store a value
    pub fn put<K: AsRef<[u8]>, V: Serialize>(&self, key: K, value: &V) -> Result<()> {
        let bytes = bincode::serialize(value).context("Failed to serialize value")?;
        self.db.insert(key, bytes).context("Failed to insert into DB")?;
        Ok(())
    }

    /// Retrieve a value
    pub fn get<K: AsRef<[u8]>, V: for<'a> Deserialize<'a>>(&self, key: K) -> Result<Option<V>> {
        match self.db.get(key).context("Failed to get from DB")? {
            Some(bytes) => {
                let value = bincode::deserialize(&bytes).context("Failed to deserialize value")?;
                Ok(Some(value))
            },
            None => Ok(None),
        }
    }

    /// Delete a value
    pub fn delete<K: AsRef<[u8]>>(&self, key: K) -> Result<()> {
        self.db.remove(key).context("Failed to remove from DB")?;
        Ok(())
    }

    /// Store a versioned object
    /// Key format: "key:version"
    pub fn put_versioned<K: AsRef<[u8]>, V: Serialize>(&self, key: K, value: &V, version: u64) -> Result<()> {
        let key_bytes = key.as_ref();
        let version_key = format!("{}:{}", String::from_utf8_lossy(key_bytes), version);
        
        let object = VersionedObject {
            version,
            data: value,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
        };

        self.put(version_key, &object)?;
        
        // Update latest pointer
        let latest_key = format!("{}:latest", String::from_utf8_lossy(key_bytes));
        self.db.insert(latest_key, &version.to_be_bytes()[..])?;
        
        Ok(())
    }

    /// Get a specific version
    pub fn get_version<K: AsRef<[u8]>, V: for<'a> Deserialize<'a>>(&self, key: K, version: u64) -> Result<Option<VersionedObject<V>>> {
        let key_bytes = key.as_ref();
        let version_key = format!("{}:{}", String::from_utf8_lossy(key_bytes), version);
        self.get(version_key)
    }

    /// Get the latest version
    pub fn get_latest<K: AsRef<[u8]>, V: for<'a> Deserialize<'a>>(&self, key: K) -> Result<Option<VersionedObject<V>>> {
        let key_bytes = key.as_ref();
        let latest_key = format!("{}:latest", String::from_utf8_lossy(key_bytes));
        
        match self.db.get(latest_key)? {
            Some(bytes) => {
                let version_bytes: [u8; 8] = bytes.as_ref().try_into().context("Invalid version bytes")?;
                let version = u64::from_be_bytes(version_bytes);
                self.get_version(key, version)
            },
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_put_get() -> Result<()> {
        let store = ObjectStore::open_temporary()?;
        store.put("key1", &"value1".to_string())?;
        
        let val: Option<String> = store.get("key1")?;
        assert_eq!(val, Some("value1".to_string()));
        
        store.delete("key1")?;
        let val: Option<String> = store.get("key1")?;
        assert!(val.is_none());
        
        Ok(())
    }

    #[test]
    fn test_versioning() -> Result<()> {
        let store = ObjectStore::open_temporary()?;
        
        store.put_versioned("doc", &"v1".to_string(), 1)?;
        store.put_versioned("doc", &"v2".to_string(), 2)?;
        
        let v1: Option<VersionedObject<String>> = store.get_version("doc", 1)?;
        assert_eq!(v1.unwrap().data, "v1");
        
        let v2: Option<VersionedObject<String>> = store.get_version("doc", 2)?;
        assert_eq!(v2.unwrap().data, "v2");
        
        let latest: Option<VersionedObject<String>> = store.get_latest("doc")?;
        assert_eq!(latest.unwrap().data, "v2");
        
        Ok(())
    }
}
