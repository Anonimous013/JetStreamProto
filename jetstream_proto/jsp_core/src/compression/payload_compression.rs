use anyhow::Result;
use std::io::Write;

/// Compression algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgorithm {
    /// LZ4 (Fast, good for streaming)
    Lz4,
    /// Brotli (High compression, good for text)
    Brotli,
    /// Zstd (Balanced, good for binary)
    Zstd,
}

/// Payload compressor with multiple algorithms
pub struct PayloadCompressor {
    /// Minimum size threshold for compression (bytes)
    min_size: usize,
    /// Compression enabled flag
    enabled: bool,
    /// Default algorithm
    algorithm: CompressionAlgorithm,
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
            algorithm: CompressionAlgorithm::Lz4,
        }
    }
    
    /// Create with default settings (512 bytes minimum)
    pub fn default() -> Self {
        Self::new(512)
    }
    
    /// Set compression algorithm
    pub fn set_algorithm(&mut self, algo: CompressionAlgorithm) {
        self.algorithm = algo;
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
    
    /// Compress payload using adaptive selection
    /// 
    /// Automatically selects the best algorithm based on data characteristics
    pub fn compress_adaptive(&self, data: &[u8]) -> Result<(Option<Vec<u8>>, CompressionAlgorithm)> {
        if !self.should_compress(data) {
            return Ok((None, CompressionAlgorithm::Lz4));
        }
        
        // Heuristic: Check for binary data (high entropy or null bytes)
        // If data looks like text -> Brotli
        // If data looks like binary -> Zstd
        // If data is very large and speed matters -> LZ4 (not implemented here, assuming best compression ratio goal)
        
        let is_text = self.is_likely_text(data);
        let algo = if is_text {
            CompressionAlgorithm::Brotli
        } else {
            CompressionAlgorithm::Zstd
        };
        
        let compressed = match algo {
            CompressionAlgorithm::Lz4 => lz4_flex::compress_prepend_size(data),
            CompressionAlgorithm::Brotli => {
                let mut writer = brotli::CompressorWriter::new(Vec::new(), 4096, 6, 20);
                writer.write_all(data)?;
                writer.into_inner()
            },
            CompressionAlgorithm::Zstd => {
                zstd::stream::encode_all(std::io::Cursor::new(data), 3)?
            }
        };
        
        if compressed.len() < data.len() {
             tracing::trace!(
                original = data.len(),
                compressed = compressed.len(),
                ratio = format!("{:.1}%", (compressed.len() as f64 / data.len() as f64) * 100.0),
                algo = ?algo,
                "Adaptive compression successful"
            );
            Ok((Some(compressed), algo))
        } else {
             tracing::trace!(
                original = data.len(),
                compressed = compressed.len(),
                algo = ?algo,
                "Adaptive compression not beneficial"
            );
            Ok((None, algo))
        }
    }

    /// Simple heuristic to check if data is likely text
    fn is_likely_text(&self, data: &[u8]) -> bool {
        // Check first 256 bytes for null bytes or non-printable chars
        let check_len = std::cmp::min(data.len(), 256);
        for &byte in &data[..check_len] {
            if byte == 0 {
                return false; // Null byte usually means binary
            }
            // Check for control characters (except whitespace)
            if byte < 32 && byte != 9 && byte != 10 && byte != 13 {
                return false;
            }
        }
        true
    }

    /// Compress payload using selected algorithm
    /// 
    /// Returns compressed data if compression is beneficial, otherwise returns None
    pub fn compress(&self, data: &[u8]) -> Result<Option<Vec<u8>>> {
        if !self.should_compress(data) {
            return Ok(None);
        }
        
        let compressed = match self.algorithm {
            CompressionAlgorithm::Lz4 => lz4_flex::compress_prepend_size(data),
            CompressionAlgorithm::Brotli => {
                let mut writer = brotli::CompressorWriter::new(Vec::new(), 4096, 6, 20); // Quality 6, LGWin 20
                writer.write_all(data)?;
                writer.into_inner()
            },
            CompressionAlgorithm::Zstd => {
                zstd::stream::encode_all(std::io::Cursor::new(data), 3)? // Level 3
            }
        };
        
        // Only use compression if it actually reduces size
        if compressed.len() < data.len() {
            tracing::trace!(
                original = data.len(),
                compressed = compressed.len(),
                ratio = format!("{:.1}%", (compressed.len() as f64 / data.len() as f64) * 100.0),
                algo = ?self.algorithm,
                "Payload compressed"
            );
            Ok(Some(compressed))
        } else {
            tracing::trace!(
                original = data.len(),
                compressed = compressed.len(),
                algo = ?self.algorithm,
                "Compression not beneficial, using original"
            );
            Ok(None)
        }
    }
    
    /// Decompress payload
    pub fn decompress(&self, data: &[u8], algo: CompressionAlgorithm) -> Result<Vec<u8>> {
        let decompressed = match algo {
            CompressionAlgorithm::Lz4 => {
                lz4_flex::decompress_size_prepended(data)
                    .map_err(|e| anyhow::anyhow!("LZ4 decompression failed: {}", e))?
            },
            CompressionAlgorithm::Brotli => {
                let mut reader = brotli::Decompressor::new(data, 4096);
                let mut buffer = Vec::new();
                std::io::Read::read_to_end(&mut reader, &mut buffer)?;
                buffer
            },
            CompressionAlgorithm::Zstd => {
                zstd::stream::decode_all(std::io::Cursor::new(data))?
            }
        };
        
        tracing::trace!(
            compressed = data.len(),
            decompressed = decompressed.len(),
            algo = ?algo,
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
    fn test_compress_large_payload_lz4() {
        let compressor = PayloadCompressor::new(512);
        let data = vec![0u8; 1024]; // Above threshold
        
        let result = compressor.compress(&data).unwrap();
        assert!(result.is_some());
        
        let compressed = result.unwrap();
        assert!(compressed.len() < data.len());
        
        let decompressed = compressor.decompress(&compressed, CompressionAlgorithm::Lz4).unwrap();
        assert_eq!(data, decompressed);
    }
    
    #[test]
    fn test_compress_large_payload_brotli() {
        let mut compressor = PayloadCompressor::new(512);
        compressor.set_algorithm(CompressionAlgorithm::Brotli);
        let data = b"Hello world ".repeat(100);
        
        let result = compressor.compress(&data).unwrap();
        assert!(result.is_some());
        
        let compressed = result.unwrap();
        let decompressed = compressor.decompress(&compressed, CompressionAlgorithm::Brotli).unwrap();
        assert_eq!(data, decompressed);
    }
    
    #[test]
    fn test_compress_large_payload_zstd() {
        let mut compressor = PayloadCompressor::new(512);
        compressor.set_algorithm(CompressionAlgorithm::Zstd);
        let data = vec![0u8; 1024];
        
        let result = compressor.compress(&data).unwrap();
        assert!(result.is_some());
        
        let compressed = result.unwrap();
        let decompressed = compressor.decompress(&compressed, CompressionAlgorithm::Zstd).unwrap();
        assert_eq!(data, decompressed);
    }

    #[test]
    fn test_adaptive_compression_text() {
        let compressor = PayloadCompressor::new(512);
        let data = b"This is a text payload that should be compressed with Brotli because it is text.".repeat(50);
        
        let (compressed, algo) = compressor.compress_adaptive(&data).unwrap();
        assert!(compressed.is_some());
        assert_eq!(algo, CompressionAlgorithm::Brotli);
        
        let decompressed = compressor.decompress(&compressed.unwrap(), algo).unwrap();
        assert_eq!(data.to_vec(), decompressed);
    }

    #[test]
    fn test_adaptive_compression_binary() {
        let compressor = PayloadCompressor::new(512);
        let mut data = vec![0u8; 1024];
        // Fill with some random-ish data but keep it compressible
        for i in 0..1024 {
            data[i] = (i % 256) as u8;
        }
        // Ensure it has null bytes to be detected as binary
        data[0] = 0;
        
        let (compressed, algo) = compressor.compress_adaptive(&data).unwrap();
        assert!(compressed.is_some());
        assert_eq!(algo, CompressionAlgorithm::Zstd);
        
        let decompressed = compressor.decompress(&compressed.unwrap(), algo).unwrap();
        assert_eq!(data, decompressed);
    }
}
