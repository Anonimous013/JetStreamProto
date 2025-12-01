use serde::{Serialize, Deserialize};

/// Hardware capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareCapabilities {
    pub has_aes_ni: bool,
    pub has_avx2: bool,
    pub has_avx512: bool,
    pub is_mobile: bool,
}

impl HardwareCapabilities {
    /// Detect hardware capabilities
    pub fn detect() -> Self {
        // In a real implementation, this would use CPUID or similar
        // For now, we'll use conservative defaults
        Self {
            has_aes_ni: cfg!(target_feature = "aes"),
            has_avx2: cfg!(target_feature = "avx2"),
            has_avx512: cfg!(target_feature = "avx512f"),
            is_mobile: cfg!(target_os = "android") || cfg!(target_os = "ios"),
        }
    }

    /// Check if hardware acceleration is available
    pub fn has_hardware_acceleration(&self) -> bool {
        self.has_aes_ni || self.has_avx2
    }
}

/// Cipher suite selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CipherSuite {
    /// ChaCha20-Poly1305 (software-optimized)
    ChaCha20Poly1305,
    /// AES-256-GCM (hardware-optimized)
    Aes256Gcm,
}

/// KDF algorithm selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KdfAlgorithm {
    /// HKDF-SHA256
    HkdfSha256,
    /// HKDF-SHA512
    HkdfSha512,
}

/// Crypto selector
#[derive(Debug, Clone)]
pub struct CryptoSelector {
    hardware_caps: HardwareCapabilities,
}

impl CryptoSelector {
    /// Create new crypto selector
    pub fn new() -> Self {
        Self {
            hardware_caps: HardwareCapabilities::detect(),
        }
    }

    /// Create with specific capabilities (for testing)
    pub fn with_capabilities(hardware_caps: HardwareCapabilities) -> Self {
        Self { hardware_caps }
    }

    /// Select optimal cipher suite
    pub fn select_cipher(&self) -> CipherSuite {
        if self.hardware_caps.has_aes_ni {
            // AES-NI available, use AES-GCM for better performance
            CipherSuite::Aes256Gcm
        } else {
            // No hardware acceleration, use ChaCha20 (faster in software)
            CipherSuite::ChaCha20Poly1305
        }
    }

    /// Select KDF algorithm
    pub fn select_kdf(&self) -> KdfAlgorithm {
        if self.hardware_caps.has_avx2 || self.hardware_caps.has_avx512 {
            // AVX available, SHA-512 can be faster
            KdfAlgorithm::HkdfSha512
        } else {
            // Use SHA-256 for better compatibility
            KdfAlgorithm::HkdfSha256
        }
    }

    /// Get hardware capabilities
    pub fn capabilities(&self) -> &HardwareCapabilities {
        &self.hardware_caps
    }

    /// Check if mobile device
    pub fn is_mobile(&self) -> bool {
        self.hardware_caps.is_mobile
    }

    /// Get recommended cipher for mobile
    pub fn mobile_cipher(&self) -> CipherSuite {
        // ChaCha20 is generally better on mobile (lower power consumption)
        CipherSuite::ChaCha20Poly1305
    }
}

impl Default for CryptoSelector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_detection() {
        let caps = HardwareCapabilities::detect();
        // Just verify it doesn't crash
        assert!(caps.has_aes_ni || !caps.has_aes_ni);
    }

    #[test]
    fn test_cipher_selection_with_aes_ni() {
        let caps = HardwareCapabilities {
            has_aes_ni: true,
            has_avx2: false,
            has_avx512: false,
            is_mobile: false,
        };
        
        let selector = CryptoSelector::with_capabilities(caps);
        assert_eq!(selector.select_cipher(), CipherSuite::Aes256Gcm);
    }

    #[test]
    fn test_cipher_selection_without_aes_ni() {
        let caps = HardwareCapabilities {
            has_aes_ni: false,
            has_avx2: false,
            has_avx512: false,
            is_mobile: false,
        };
        
        let selector = CryptoSelector::with_capabilities(caps);
        assert_eq!(selector.select_cipher(), CipherSuite::ChaCha20Poly1305);
    }

    #[test]
    fn test_kdf_selection_with_avx() {
        let caps = HardwareCapabilities {
            has_aes_ni: false,
            has_avx2: true,
            has_avx512: false,
            is_mobile: false,
        };
        
        let selector = CryptoSelector::with_capabilities(caps);
        assert_eq!(selector.select_kdf(), KdfAlgorithm::HkdfSha512);
    }

    #[test]
    fn test_kdf_selection_without_avx() {
        let caps = HardwareCapabilities {
            has_aes_ni: false,
            has_avx2: false,
            has_avx512: false,
            is_mobile: false,
        };
        
        let selector = CryptoSelector::with_capabilities(caps);
        assert_eq!(selector.select_kdf(), KdfAlgorithm::HkdfSha256);
    }

    #[test]
    fn test_mobile_cipher() {
        let caps = HardwareCapabilities {
            has_aes_ni: true,
            has_avx2: false,
            has_avx512: false,
            is_mobile: true,
        };
        
        let selector = CryptoSelector::with_capabilities(caps);
        assert!(selector.is_mobile());
        assert_eq!(selector.mobile_cipher(), CipherSuite::ChaCha20Poly1305);
    }

    #[test]
    fn test_hardware_acceleration_check() {
        let caps1 = HardwareCapabilities {
            has_aes_ni: true,
            has_avx2: false,
            has_avx512: false,
            is_mobile: false,
        };
        assert!(caps1.has_hardware_acceleration());

        let caps2 = HardwareCapabilities {
            has_aes_ni: false,
            has_avx2: false,
            has_avx512: false,
            is_mobile: false,
        };
        assert!(!caps2.has_hardware_acceleration());
    }
}
