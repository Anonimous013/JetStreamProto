use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::sync::Arc;
use anyhow::Result;

/// Generate self-signed certificate for QUIC
pub fn generate_self_signed_cert() -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])?;
    let key_der = cert.key_pair.serialize_der();
    let cert_der = cert.cert.der().clone();
    
    Ok((
        vec![CertificateDer::from(cert_der)],
        PrivateKeyDer::try_from(key_der).map_err(|e| anyhow::anyhow!(e))?,
    ))
}

/// Create rustls ServerConfig for QUIC
pub fn server_config() -> Result<rustls::ServerConfig> {
    let (certs, key) = generate_self_signed_cert()?;
    
    let mut config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;
    
    config.alpn_protocols = vec![b"jetstream".to_vec()];
    config.max_early_data_size = 0xffffffff; // Enable 0-RTT
    
    Ok(config)
}

/// Create rustls ClientConfig for QUIC
pub fn client_config() -> rustls::ClientConfig {
    let mut config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(SkipServerVerification::new())
        .with_no_client_auth();
    
    config.alpn_protocols = vec![b"jetstream".to_vec()];
    config.enable_early_data = true; // Enable 0-RTT
    
    config
}

// Skip certificate verification for development
#[derive(Debug)]
struct SkipServerVerification;

impl SkipServerVerification {
    fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }
    
    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    
    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn init() {
        INIT.call_once(|| {
            let _ = rustls::crypto::ring::default_provider().install_default();
        });
    }

    #[test]
    fn test_generate_cert() {
        init();
        let result = generate_self_signed_cert();
        assert!(result.is_ok());
        let (certs, key) = result.unwrap();
        assert!(!certs.is_empty());
        // PrivateKeyDer is an enum, we just check it was generated
        let _ = key;
    }

    #[test]
    fn test_server_config() {
        init();
        let config = server_config();
        assert!(config.is_ok());
        let config = config.unwrap();
        assert!(config.alpn_protocols.contains(&b"jetstream".to_vec()));
    }

    #[test]
    fn test_client_config() {
        init();
        let config = client_config();
        assert!(config.alpn_protocols.contains(&b"jetstream".to_vec()));
        assert!(config.enable_early_data);
    }
}
