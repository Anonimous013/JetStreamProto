use serde::{Serialize, Deserialize};

/// Data type detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    /// Text data (UTF-8, ASCII)
    Text,
    /// Binary data
    Binary,
    /// Media data (images, video, audio)
    Media,
    /// Already compressed data
    Compressed,
}

/// Compression algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    /// No compression
    None,
    /// Zstd (balanced, good for most data)
    Zstd,
    /// LZ4 (fast, low CPU usage)
    Lz4,
    /// Brotli (best compression for text)
    Brotli,
}

/// Compression selector
#[derive(Debug, Clone)]
pub struct CompressionSelector {
    /// Available bandwidth (bytes/sec)
    bandwidth: u64,
    /// Available CPU (0.0 - 1.0)
    cpu_available: f32,
    /// Minimum size to compress
    min_compress_size: usize,
}

impl CompressionSelector {
    /// Create new compression selector
    pub fn new(bandwidth: u64, cpu_available: f32) -> Self {
        Self {
            bandwidth,
            cpu_available,
            min_compress_size: 128, // Don't compress small packets
        }
    }

    /// Select compression algorithm based on data and conditions
    pub fn select_algorithm(&self, data: &[u8]) -> CompressionAlgorithm {
        // Don't compress small data
        if !self.should_compress(data.len()) {
            return CompressionAlgorithm::None;
        }

        let data_type = self.detect_data_type(data);

        match data_type {
            DataType::Text => {
                if self.cpu_available > 0.5 {
                    // High CPU available, use best compression
                    CompressionAlgorithm::Brotli
                } else {
                    // Low CPU, use balanced
                    CompressionAlgorithm::Zstd
                }
            }
            DataType::Binary => {
                if self.bandwidth < 1_000_000 {
                    // Low bandwidth, prioritize compression
                    CompressionAlgorithm::Zstd
                } else {
                    // High bandwidth, prioritize speed
                    CompressionAlgorithm::Lz4
                }
            }
            DataType::Media | DataType::Compressed => {
                // Already compressed, don't compress again
                CompressionAlgorithm::None
            }
        }
    }

    /// Detect data type
    pub fn detect_data_type(&self, data: &[u8]) -> DataType {
        if data.is_empty() {
            return DataType::Binary;
        }

        // Check for common compressed/media formats
        if self.is_compressed_format(data) {
            return DataType::Compressed;
        }

        if self.is_media_format(data) {
            return DataType::Media;
        }

        // Check if data is mostly ASCII/UTF-8
        if self.is_text_data(data) {
            return DataType::Text;
        }

        DataType::Binary
    }

    /// Check if data should be compressed
    pub fn should_compress(&self, size: usize) -> bool {
        size >= self.min_compress_size
    }

    /// Check if data is compressed format
    fn is_compressed_format(&self, data: &[u8]) -> bool {
        if data.len() < 4 {
            return false;
        }

        // Check magic numbers for compressed formats
        matches!(
            &data[0..2],
            b"\x1f\x8b" | // gzip
            b"PK" | // zip
            b"BZ" | // bzip2
            b"\x28\xb5" // zstd
        ) || matches!(&data[0..4], b"\x89PNG" | b"RIFF") // PNG, WEBP
    }

    /// Check if data is media format
    fn is_media_format(&self, data: &[u8]) -> bool {
        if data.len() < 4 {
            return false;
        }

        // Check magic numbers for media formats
        matches!(
            &data[0..3],
            b"\xff\xd8\xff" | // JPEG
            b"GIF" // GIF
        ) || matches!(
            &data[0..4],
            b"\x00\x00\x00\x18" | // MP4
            b"\x00\x00\x00\x1c" | // MP4
            b"ftyp" | // MP4
            b"RIFF" // WAV, AVI, WEBP
        )
    }

    /// Check if data is text
    fn is_text_data(&self, data: &[u8]) -> bool {
        let sample_size = data.len().min(1024);
        let sample = &data[0..sample_size];

        let mut text_chars = 0;
        for &byte in sample {
            if byte.is_ascii_graphic() || byte.is_ascii_whitespace() {
                text_chars += 1;
            }
        }

        // If more than 80% is text, consider it text
        (text_chars as f32 / sample_size as f32) > 0.8
    }

    /// Update bandwidth estimate
    pub fn update_bandwidth(&mut self, bandwidth: u64) {
        self.bandwidth = bandwidth;
    }

    /// Update CPU availability
    pub fn update_cpu(&mut self, cpu_available: f32) {
        self.cpu_available = cpu_available.clamp(0.0, 1.0);
    }

    /// Set minimum compression size
    pub fn set_min_compress_size(&mut self, size: usize) {
        self.min_compress_size = size;
    }
}

impl Default for CompressionSelector {
    fn default() -> Self {
        Self::new(10_000_000, 0.5) // 10 Mbps, 50% CPU
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_detection() {
        let selector = CompressionSelector::default();
        let text_data = b"Hello, this is some text data!";
        assert_eq!(selector.detect_data_type(text_data), DataType::Text);
    }

    #[test]
    fn test_binary_detection() {
        let selector = CompressionSelector::default();
        let binary_data = &[0u8, 1, 2, 3, 255, 254, 253];
        assert_eq!(selector.detect_data_type(binary_data), DataType::Binary);
    }

    #[test]
    fn test_compressed_detection() {
        let selector = CompressionSelector::default();
        let gzip_data = b"\x1f\x8b\x08\x00\x00\x00\x00\x00";
        assert_eq!(selector.detect_data_type(gzip_data), DataType::Compressed);
    }

    #[test]
    fn test_media_detection() {
        let selector = CompressionSelector::default();
        let jpeg_data = b"\xff\xd8\xff\xe0\x00\x10JFIF";
        assert_eq!(selector.detect_data_type(jpeg_data), DataType::Media);
    }

    #[test]
    fn test_algorithm_selection_text_high_cpu() {
        let selector = CompressionSelector::new(10_000_000, 0.8);
        // Make text long enough to compress (> 128 bytes)
        let text_data = b"This is some text that should be compressed with Brotli. \
                          Adding more text to make it longer than the minimum compression size. \
                          This ensures the test actually tests compression algorithm selection.";
        assert_eq!(selector.select_algorithm(text_data), CompressionAlgorithm::Brotli);
    }

    #[test]
    fn test_algorithm_selection_text_low_cpu() {
        let selector = CompressionSelector::new(10_000_000, 0.2);
        // Make text long enough to compress (> 128 bytes)
        let text_data = b"This is some text that should be compressed with Zstd. \
                          Adding more text to make it longer than the minimum compression size. \
                          This ensures the test actually tests compression algorithm selection.";
        assert_eq!(selector.select_algorithm(text_data), CompressionAlgorithm::Zstd);
    }

    #[test]
    fn test_algorithm_selection_binary_low_bandwidth() {
        let selector = CompressionSelector::new(500_000, 0.5);
        let binary_data = &[0u8; 200];
        assert_eq!(selector.select_algorithm(binary_data), CompressionAlgorithm::Zstd);
    }

    #[test]
    fn test_algorithm_selection_binary_high_bandwidth() {
        let selector = CompressionSelector::new(100_000_000, 0.5);
        let binary_data = &[0u8; 200];
        assert_eq!(selector.select_algorithm(binary_data), CompressionAlgorithm::Lz4);
    }

    #[test]
    fn test_no_compression_for_small_data() {
        let selector = CompressionSelector::default();
        let small_data = b"small";
        assert_eq!(selector.select_algorithm(small_data), CompressionAlgorithm::None);
    }

    #[test]
    fn test_no_compression_for_media() {
        let selector = CompressionSelector::default();
        let jpeg_data = b"\xff\xd8\xff\xe0\x00\x10JFIF\x00\x01\x01\x00\x00\x01\x00\x01\x00\x00";
        assert_eq!(selector.select_algorithm(jpeg_data), CompressionAlgorithm::None);
    }

    #[test]
    fn test_should_compress() {
        let selector = CompressionSelector::default();
        assert!(!selector.should_compress(64));
        assert!(selector.should_compress(256));
    }

    #[test]
    fn test_update_bandwidth() {
        let mut selector = CompressionSelector::default();
        selector.update_bandwidth(1_000_000);
        assert_eq!(selector.bandwidth, 1_000_000);
    }

    #[test]
    fn test_update_cpu() {
        let mut selector = CompressionSelector::default();
        selector.update_cpu(0.9);
        assert_eq!(selector.cpu_available, 0.9);
        
        // Test clamping
        selector.update_cpu(1.5);
        assert_eq!(selector.cpu_available, 1.0);
    }
}
