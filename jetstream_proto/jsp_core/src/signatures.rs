use pqcrypto_dilithium::dilithium5;
use pqcrypto_traits::sign::{PublicKey as SignPublicKey, SecretKey as SignSecretKey, SignedMessage};
use anyhow::Result;

/// Dilithium5 signature context for post-quantum digital signatures
pub struct SignatureContext {
    secret_key: dilithium5::SecretKey,
    public_key: dilithium5::PublicKey,
}

impl Default for SignatureContext {
    fn default() -> Self {
        Self::new()
    }
}

impl SignatureContext {
    /// Create a new signature context with a fresh keypair
    pub fn new() -> Self {
        let (public_key, secret_key) = dilithium5::keypair();
        Self {
            secret_key,
            public_key,
        }
    }

    /// Get the public key bytes for sharing
    pub fn public_key_bytes(&self) -> &[u8] {
        self.public_key.as_bytes()
    }

    /// Sign a message using Dilithium5
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        let signed_msg = dilithium5::sign(message, &self.secret_key);
        signed_msg.as_bytes().to_vec()
    }

    /// Verify a signed message using a peer's public key
    pub fn verify(signed_message: &[u8], peer_public_key_bytes: &[u8]) -> Result<Vec<u8>> {
        if peer_public_key_bytes.len() != dilithium5::public_key_bytes() {
            return Err(anyhow::anyhow!("Invalid Dilithium public key length"));
        }

        let peer_public_key = dilithium5::PublicKey::from_bytes(peer_public_key_bytes)
            .map_err(|_| anyhow::anyhow!("Invalid Dilithium public key"))?;

        if signed_message.len() < dilithium5::signature_bytes() {
            return Err(anyhow::anyhow!("Signed message too short"));
        }

        let signed_msg = dilithium5::SignedMessage::from_bytes(signed_message)
            .map_err(|_| anyhow::anyhow!("Invalid signed message format"))?;

        let original_message = dilithium5::open(&signed_msg, &peer_public_key)
            .map_err(|_| anyhow::anyhow!("Signature verification failed"))?;

        Ok(original_message)
    }

    /// Create from existing keypair bytes (secret_key + public_key concatenated)
    pub fn from_keypair(secret_key_bytes: &[u8], public_key_bytes: &[u8]) -> Result<Self> {
        if secret_key_bytes.len() != dilithium5::secret_key_bytes() {
            return Err(anyhow::anyhow!("Invalid secret key length"));
        }
        
        if public_key_bytes.len() != dilithium5::public_key_bytes() {
            return Err(anyhow::anyhow!("Invalid public key length"));
        }

        let secret_key = dilithium5::SecretKey::from_bytes(secret_key_bytes)
            .map_err(|_| anyhow::anyhow!("Invalid secret key"))?;

        let public_key = dilithium5::PublicKey::from_bytes(public_key_bytes)
            .map_err(|_| anyhow::anyhow!("Invalid public key"))?;

        Ok(Self {
            secret_key,
            public_key,
        })
    }

    /// Export both secret and public keys for storage (should be encrypted!)
    pub fn export_keypair(&self) -> (Vec<u8>, Vec<u8>) {
        (self.secret_key.as_bytes().to_vec(), self.public_key.as_bytes().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_and_verify() {
        let ctx = SignatureContext::new();
        let message = b"Hello, JetStreamProto!";

        let signed = ctx.sign(message);
        let verified = SignatureContext::verify(&signed, ctx.public_key_bytes())
            .expect("Verification should succeed");

        assert_eq!(verified, message);
    }

    #[test]
    fn test_verify_invalid_signature() {
        let ctx1 = SignatureContext::new();
        let ctx2 = SignatureContext::new();
        let message = b"Test message";

        let signed = ctx1.sign(message);
        
        // Try to verify with wrong public key
        let result = SignatureContext::verify(&signed, ctx2.public_key_bytes());
        assert!(result.is_err());
    }

    #[test]
    fn test_export_import_key() {
        let ctx1 = SignatureContext::new();
        let (secret_bytes, public_bytes) = ctx1.export_keypair();

        let ctx2 = SignatureContext::from_keypair(&secret_bytes, &public_bytes)
            .expect("Should import key successfully");

        let message = b"Test persistence";
        let signed = ctx1.sign(message);
        let verified = SignatureContext::verify(&signed, ctx2.public_key_bytes())
            .expect("Should verify with imported key");

        assert_eq!(verified, message);
    }
}
