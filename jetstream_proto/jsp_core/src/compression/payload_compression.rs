use anyhow::Result;

/// LZ4 payload compressor with adaptive compression
pub struct PayloadCompressor {
    /// Minimum size threshold for compression (bytes)
    min_size: usize,
    /// Compression enabled flag
    enabled: bool,
}

impl PayloadCompressor {
    /// Create a new payload compressor
    /// 
    /// # Arguments
    /// * `min_size` - Minimum payload size to trigger compression (default: 512 bytes)
    pub fn new(min_size: usize) -> Self {
        Self {
            min_size,
            enabled: true,
        }
    }
    
    /// Create with default settings (512 bytes minimum)
    pub fn default() -> Self {
        Self::new(512)
    }
    
    /// Enable compression
    pub fn enable(&mut self) {
        self.enabled = true;
    }
    
    /// Disable compression
    pub fn disable(&mut self) {
        self.enabled = false;
    }
    
    /// Check if payload should be compressed
    fn should_compress(&self, data: &[u8]) -> bool {
        self.enabled && data.len() >= self.min_size
    }
    
    /// Compress payload using LZ4
    /// 
    /// Returns compressed data if compression is beneficial, otherwise returns None
    pub fn compress(&self, data: &[u8]) -> Result<Option<Vec<u8>>> {
        if !self.should_compress(data) {
            return Ok(None);
        }
        
        let compressed = lz4_flex::compress_prepend_size(data);
        
        // Only use compression if it actually reduces size
        if compressed.len() < data.len() {
            tracing::trace!(
                original = data.len(),
                compressed = compressed.len(),
                ratio = format!("{:.1}%", (compressed.len() as f64 / data.len() as f64) * 100.0),
                "Payload compressed"
            );
            Ok(Some(compressed))
        } else {
            tracing::trace!(
                original = data.len(),
                compressed = compressed.len(),
                "Compression not beneficial, using original"
            );
            Ok(None)
        }
    }
    
    /// Decompress LZ4 compressed payload
    pub fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        let decompressed = lz4_flex::decompress_size_prepended(data)
            .map_err(|e| anyhow::anyhow!("LZ4 decompression failed: {}", e))?;
        
        tracing::trace!(
            compressed = data.len(),
            decompressed = decompressed.len(),
            "Payload decompressed"
        );
        
        Ok(decompressed)
    }
    
    /// Get compression statistics for a payload
    pub fn compression_ratio(&self, original: &[u8], compressed: &[u8]) -> f64 {
        (compressed.len() as f64 / original.len() as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_compress_small_payload() {
        let compressor = PayloadCompressor::new(512);
        let data = vec![0u8; 256]; // Below threshold
        
        let result = compressor.compress(&data).unwrap();
        assert!(result.is_none(), "Small payloads should not be compressed");
    }
    
    #[test]
    fn test_compress_large_payload() {
        let compressor = PayloadCompressor::new(512);
        let data = vec![0u8; 1024]; // Above threshold, highly compressible
        
        let result = compressor.compress(&data).unwrap();
        assert!(result.is_some(), "Large compressible payloads should be compressed");
        
        let compressed = result.unwrap();
        assert!(compressed.len() < data.len(), "Compressed size should be smaller");
    }
    
    #[test]
    fn test_compress_decompress_roundtrip() {
        let compressor = PayloadCompressor::new(512);
        let original = b"Hello, World! This is a test payload that should be compressed.".repeat(20);
        
        let compressed = compressor.compress(&original).unwrap().unwrap();
        let decompressed = compressor.decompress(&compressed).unwrap();
        
        assert_eq!(original, decompressed, "Roundtrip should preserve data");
    }
    
    #[test]
    fn test_random_data_compression() {
        let compressor = PayloadCompressor::new(512);
        let random_data: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();
        
        let result = compressor.compress(&random_data).unwrap();
        // Random data might not compress well, so result could be None
        if let Some(compressed) = result {
            let decompressed = compressor.decompress(&compressed).unwrap();
            assert_eq!(random_data, decompressed);
        }
    }
    
    #[test]
    fn test_disabled_compression() {
        let mut compressor = PayloadCompressor::new(512);
        compressor.disable();
        
        let data = vec![0u8; 1024];
        let result = compressor.compress(&data).unwrap();
        
        assert!(result.is_none(), "Disabled compressor should not compress");
    }
}
