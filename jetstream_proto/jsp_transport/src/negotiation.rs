use std::collections::HashSet;
use serde::{Serialize, Deserialize};

/// Protocol version
pub const CURRENT_VERSION: u8 = 1;
pub const MIN_SUPPORTED_VERSION: u8 = 1;
pub const MAX_SUPPORTED_VERSION: u8 = 1;

/// Feature flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Feature {
    /// QUIC transport support
    QuicTransport,
    /// TCP transport support
    TcpTransport,
    /// UDP transport support (always supported)
    UdpTransport,
    /// ChaCha20-Poly1305 encryption
    ChaCha20Poly1305,
    /// AES-256-GCM encryption
    Aes256Gcm,
    /// Zstd compression
    ZstdCompression,
    /// LZ4 compression
    Lz4Compression,
    /// Brotli compression
    BrotliCompression,
    /// 0-RTT connection resumption
    ZeroRttResumption,
    /// Connection migration
    ConnectionMigration,
    /// Multiplexing
    Multiplexing,
    /// Proof-of-Work challenge
    ProofOfWork,
}

/// Feature set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureSet {
    features: HashSet<Feature>,
}

impl FeatureSet {
    pub fn new() -> Self {
        Self {
            features: HashSet::new(),
        }
    }

    pub fn with_defaults() -> Self {
        let mut features = HashSet::new();
        features.insert(Feature::UdpTransport);
        features.insert(Feature::ChaCha20Poly1305);
        features.insert(Feature::ZstdCompression);
        features.insert(Feature::Multiplexing);
        
        Self { features }
    }

    pub fn add(&mut self, feature: Feature) {
        self.features.insert(feature);
    }

    pub fn remove(&mut self, feature: Feature) {
        self.features.remove(&feature);
    }

    pub fn has(&self, feature: Feature) -> bool {
        self.features.contains(&feature)
    }

    pub fn intersect(&self, other: &FeatureSet) -> FeatureSet {
        let features = self.features.intersection(&other.features).copied().collect();
        FeatureSet { features }
    }

    pub fn len(&self) -> usize {
        self.features.len()
    }

    pub fn is_empty(&self) -> bool {
        self.features.is_empty()
    }
}

impl Default for FeatureSet {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Negotiation result
#[derive(Debug, Clone)]
pub struct NegotiationResult {
    pub version: u8,
    pub features: FeatureSet,
}

/// Protocol negotiator
#[derive(Debug, Clone)]
pub struct ProtocolNegotiator {
    supported_versions: Vec<u8>,
    supported_features: FeatureSet,
}

impl ProtocolNegotiator {
    pub fn new() -> Self {
        let mut supported_features = FeatureSet::with_defaults();
        
        // Add all supported features
        supported_features.add(Feature::TcpTransport);
        supported_features.add(Feature::QuicTransport);
        supported_features.add(Feature::Aes256Gcm);
        supported_features.add(Feature::Lz4Compression);
        supported_features.add(Feature::BrotliCompression);
        supported_features.add(Feature::ZeroRttResumption);
        supported_features.add(Feature::ConnectionMigration);
        supported_features.add(Feature::ProofOfWork);

        Self {
            supported_versions: vec![MIN_SUPPORTED_VERSION, CURRENT_VERSION],
            supported_features,
        }
    }

    /// Negotiate protocol version and features
    pub fn negotiate(&self, client_versions: &[u8], client_features: &FeatureSet) -> Option<NegotiationResult> {
        // Select version
        let version = self.select_version(client_versions)?;
        
        // Select features (intersection of supported features)
        let features = self.supported_features.intersect(client_features);
        
        Some(NegotiationResult { version, features })
    }

    /// Select highest mutually supported version
    pub fn select_version(&self, client_versions: &[u8]) -> Option<u8> {
        let mut common_versions: Vec<u8> = self.supported_versions
            .iter()
            .filter(|v| client_versions.contains(v))
            .copied()
            .collect();
        
        common_versions.sort_by(|a, b| b.cmp(a)); // Sort descending
        common_versions.first().copied()
    }

    /// Get supported versions
    pub fn supported_versions(&self) -> &[u8] {
        &self.supported_versions
    }

    /// Get supported features
    pub fn supported_features(&self) -> &FeatureSet {
        &self.supported_features
    }

    /// Check if version is supported
    pub fn supports_version(&self, version: u8) -> bool {
        self.supported_versions.contains(&version)
    }

    /// Check if feature is supported
    pub fn supports_feature(&self, feature: Feature) -> bool {
        self.supported_features.has(feature)
    }
}

impl Default for ProtocolNegotiator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_set() {
        let mut features = FeatureSet::new();
        assert!(features.is_empty());

        features.add(Feature::QuicTransport);
        assert!(features.has(Feature::QuicTransport));
        assert_eq!(features.len(), 1);

        features.remove(Feature::QuicTransport);
        assert!(!features.has(Feature::QuicTransport));
        assert!(features.is_empty());
    }

    #[test]
    fn test_feature_intersection() {
        let mut features1 = FeatureSet::new();
        features1.add(Feature::QuicTransport);
        features1.add(Feature::ChaCha20Poly1305);

        let mut features2 = FeatureSet::new();
        features2.add(Feature::ChaCha20Poly1305);
        features2.add(Feature::Aes256Gcm);

        let intersection = features1.intersect(&features2);
        assert!(intersection.has(Feature::ChaCha20Poly1305));
        assert!(!intersection.has(Feature::QuicTransport));
        assert!(!intersection.has(Feature::Aes256Gcm));
        assert_eq!(intersection.len(), 1);
    }

    #[test]
    fn test_version_selection() {
        let negotiator = ProtocolNegotiator::new();
        
        // Client supports version 1
        let version = negotiator.select_version(&[1]);
        assert_eq!(version, Some(1));

        // Client supports unsupported version
        let version = negotiator.select_version(&[99]);
        assert_eq!(version, None);

        // Client supports multiple versions
        let version = negotiator.select_version(&[1, 2]);
        assert_eq!(version, Some(1)); // Should select highest common
    }

    #[test]
    fn test_negotiation() {
        let negotiator = ProtocolNegotiator::new();
        
        let mut client_features = FeatureSet::new();
        client_features.add(Feature::QuicTransport);
        client_features.add(Feature::ChaCha20Poly1305);

        let result = negotiator.negotiate(&[1], &client_features);
        assert!(result.is_some());

        let result = result.unwrap();
        assert_eq!(result.version, 1);
        assert!(result.features.has(Feature::QuicTransport));
        assert!(result.features.has(Feature::ChaCha20Poly1305));
    }

    #[test]
    fn test_default_features() {
        let features = FeatureSet::with_defaults();
        assert!(features.has(Feature::UdpTransport));
        assert!(features.has(Feature::ChaCha20Poly1305));
        assert!(features.has(Feature::ZstdCompression));
        assert!(features.has(Feature::Multiplexing));
    }
}
