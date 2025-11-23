use anyhow::{Result, anyhow};
use reed_solomon_erasure::galois_8::ReedSolomon;

/// Forward Error Correction using Reed-Solomon encoding
/// 
/// Uses 10 data shards + 2 parity shards (10/2 configuration)
/// This allows recovery from up to 2 lost packets
pub struct FecEncoder {
    /// Number of data shards
    data_shards: usize,
    /// Number of parity shards
    parity_shards: usize,
    /// Reed-Solomon encoder
    encoder: ReedSolomon,
}

impl FecEncoder {
    /// Create a new FEC encoder with 10/2 configuration
    /// 
    /// - 10 data shards
    /// - 2 parity shards
    /// - Can recover from up to 2 lost packets
    pub fn new() -> Result<Self> {
        Self::with_config(10, 2)
    }
    
    /// Create a new FEC encoder with custom configuration
    /// 
    /// # Arguments
    /// * `data_shards` - Number of data shards
    /// * `parity_shards` - Number of parity shards
    pub fn with_config(data_shards: usize, parity_shards: usize) -> Result<Self> {
        let encoder = ReedSolomon::new(data_shards, parity_shards)
            .map_err(|e| anyhow!("Failed to create Reed-Solomon encoder: {:?}", e))?;
        
        Ok(Self {
            data_shards,
            parity_shards,
            encoder,
        })
    }
    
    /// Encode data into shards with parity
    /// 
    /// # Arguments
    /// * `data` - Input data to encode
    /// 
    /// # Returns
    /// Vector of shards (data shards + parity shards)
    pub fn encode(&self, data: &[u8]) -> Result<Vec<Vec<u8>>> {
        // Calculate shard size
        let shard_size = (data.len() + self.data_shards - 1) / self.data_shards;
        
        // Create data shards
        let mut shards: Vec<Vec<u8>> = Vec::with_capacity(self.data_shards + self.parity_shards);
        
        // Split data into shards
        for i in 0..self.data_shards {
            let start = i * shard_size;
            let end = std::cmp::min(start + shard_size, data.len());
            
            let mut shard = vec![0u8; shard_size];
            if start < data.len() {
                let copy_len = end - start;
                shard[..copy_len].copy_from_slice(&data[start..end]);
            }
            shards.push(shard);
        }
        
        // Add empty parity shards
        for _ in 0..self.parity_shards {
            shards.push(vec![0u8; shard_size]);
        }
        
        // Encode to generate parity shards
        self.encoder.encode(&mut shards)
            .map_err(|e| anyhow!("FEC encoding failed: {:?}", e))?;
        
        tracing::trace!(
            data_size = data.len(),
            shard_count = shards.len(),
            shard_size = shard_size,
            "FEC encoded data"
        );
        
        Ok(shards)
    }
    
    /// Decode shards back to original data
    /// 
    /// # Arguments
    /// * `shards` - Vector of shards (some may be None if lost)
    /// * `original_size` - Original data size
    /// 
    /// # Returns
    /// Recovered original data
    pub fn decode(&self, shards: &mut [Option<Vec<u8>>], original_size: usize) -> Result<Vec<u8>> {
        // Reconstruct missing shards
        self.encoder.reconstruct(shards)
            .map_err(|e| anyhow!("FEC reconstruction failed: {:?}", e))?;
        
        // Extract data shards and concatenate
        let mut data = Vec::with_capacity(original_size);
        
        for i in 0..self.data_shards {
            if let Some(ref shard) = shards[i] {
                data.extend_from_slice(shard);
            } else {
                return Err(anyhow!("Missing data shard after reconstruction"));
            }
        }
        
        // Truncate to original size
        data.truncate(original_size);
        
        tracing::trace!(
            recovered_size = data.len(),
            "FEC decoded data"
        );
        
        Ok(data)
    }
    
    /// Get the number of data shards
    pub fn data_shards(&self) -> usize {
        self.data_shards
    }
    
    /// Get the number of parity shards
    pub fn parity_shards(&self) -> usize {
        self.parity_shards
    }
    
    /// Get total number of shards
    pub fn total_shards(&self) -> usize {
        self.data_shards + self.parity_shards
    }
}

impl Default for FecEncoder {
    fn default() -> Self {
        Self::new().expect("Failed to create default FEC encoder")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fec_encode_decode() {
        let encoder = FecEncoder::new().unwrap();
        let data = b"Hello, World! This is a test of FEC encoding.".repeat(10);
        
        // Encode
        let shards = encoder.encode(&data).unwrap();
        assert_eq!(shards.len(), 12); // 10 data + 2 parity
        
        // Decode without loss
        let mut shard_options: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        let recovered = encoder.decode(&mut shard_options, data.len()).unwrap();
        
        assert_eq!(data, recovered.as_slice());
    }
    
    #[test]
    fn test_fec_with_one_lost_shard() {
        let encoder = FecEncoder::new().unwrap();
        let data = b"Testing FEC with packet loss".repeat(20);
        
        // Encode
        let shards = encoder.encode(&data).unwrap();
        
        // Simulate losing one shard
        let mut shard_options: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        shard_options[3] = None; // Lose shard 3
        
        // Decode
        let recovered = encoder.decode(&mut shard_options, data.len()).unwrap();
        
        assert_eq!(data, recovered.as_slice());
    }
    
    #[test]
    fn test_fec_with_two_lost_shards() {
        let encoder = FecEncoder::new().unwrap();
        let data = b"Testing FEC with multiple packet losses".repeat(15);
        
        // Encode
        let shards = encoder.encode(&data).unwrap();
        
        // Simulate losing two shards
        let mut shard_options: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        shard_options[2] = None; // Lose shard 2
        shard_options[7] = None; // Lose shard 7
        
        // Decode
        let recovered = encoder.decode(&mut shard_options, data.len()).unwrap();
        
        assert_eq!(data, recovered.as_slice());
    }
    
    #[test]
    fn test_fec_custom_config() {
        let encoder = FecEncoder::with_config(8, 4).unwrap(); // 8 data + 4 parity
        let data = b"Custom FEC configuration test".repeat(10);
        
        // Encode
        let shards = encoder.encode(&data).unwrap();
        assert_eq!(shards.len(), 12); // 8 data + 4 parity
        
        // Simulate losing 4 shards (maximum recoverable)
        let mut shard_options: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        shard_options[1] = None;
        shard_options[3] = None;
        shard_options[5] = None;
        shard_options[9] = None;
        
        // Decode
        let recovered = encoder.decode(&mut shard_options, data.len()).unwrap();
        
        assert_eq!(data, recovered.as_slice());
    }
    
    #[test]
    fn test_fec_small_data() {
        let encoder = FecEncoder::new().unwrap();
        let data = b"Small";
        
        // Encode
        let shards = encoder.encode(data).unwrap();
        
        // Decode
        let mut shard_options: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        let recovered = encoder.decode(&mut shard_options, data.len()).unwrap();
        
        assert_eq!(data, recovered.as_slice());
    }
}
