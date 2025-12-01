use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce
};
use aes_gcm::Aes256Gcm;
use x25519_dalek::{PublicKey, StaticSecret};
use pqcrypto_kyber::kyber768;
use pqcrypto_traits::kem::{Ciphertext, PublicKey as KyberPublicKey, SharedSecret};
use rand_core::OsRng;
use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CipherSuite {
    ChaCha20Poly1305,
    Aes256Gcm,
}

pub struct CryptoContext {
    local_secret: StaticSecret,
    local_public: PublicKey,
    kyber_secret: kyber768::SecretKey,
    kyber_public: kyber768::PublicKey,
    shared_secret: Option<Key>,
    cipher_suite: CipherSuite,
}

impl Default for CryptoContext {
    fn default() -> Self {
        Self::new()
    }
}

impl CryptoContext {
    pub fn new() -> Self {
        let local_secret = StaticSecret::random_from_rng(OsRng);
        let local_public = PublicKey::from(&local_secret);
        
        // Generate Kyber-768 keypair (upgraded from Kyber-512)
        let (kyber_public, kyber_secret) = kyber768::keypair();
        
        Self {
            local_secret,
            local_public,
            kyber_secret,
            kyber_public,
            shared_secret: None,
            cipher_suite: CipherSuite::ChaCha20Poly1305, // Default
        }
    }

    pub fn set_cipher_suite(&mut self, suite: CipherSuite) {
        self.cipher_suite = suite;
    }

    pub fn x25519_public_key(&self) -> &[u8; 32] {
        self.local_public.as_bytes()
    }
    
    pub fn kyber_public_key(&self) -> &[u8] {
        self.kyber_public.as_bytes()
    }

    pub fn encapsulate_kyber(&self, peer_kyber_pk_bytes: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
        // Ensure peer key is correct length for Kyber-768
        if peer_kyber_pk_bytes.len() != kyber768::public_key_bytes() {
             return Err(anyhow::anyhow!("Invalid Kyber public key length"));
        }
        
        let peer_kyber_pk = kyber768::PublicKey::from_bytes(peer_kyber_pk_bytes)
            .map_err(|_| anyhow::anyhow!("Invalid Kyber public key"))?;
            
        let (shared_secret, ciphertext) = kyber768::encapsulate(&peer_kyber_pk);
            
        Ok((ciphertext.as_bytes().to_vec(), shared_secret.as_bytes().to_vec()))
    }
    
    pub fn decapsulate_kyber(&self, ciphertext_bytes: &[u8]) -> Result<Vec<u8>> {
        if ciphertext_bytes.len() != kyber768::ciphertext_bytes() {
            return Err(anyhow::anyhow!("Invalid Kyber ciphertext length"));
        }
        
        let ciphertext = kyber768::Ciphertext::from_bytes(ciphertext_bytes)
            .map_err(|_| anyhow::anyhow!("Invalid Kyber ciphertext"))?;
        
        let shared_secret = kyber768::decapsulate(&ciphertext, &self.kyber_secret);
             
        Ok(shared_secret.as_bytes().to_vec())
    }

    pub fn derive_shared_secret(&mut self, peer_public_bytes: &[u8; 32], kyber_shared: Option<&[u8]>, client_random: &[u8; 32], server_random: &[u8; 32]) {
        use hkdf::Hkdf;
        use sha2::Sha256;
        
        let peer_public = PublicKey::from(*peer_public_bytes);
        let x25519_shared = self.local_secret.diffie_hellman(&peer_public);
        
        // Combine shared secrets: X25519 || Kyber (if present)
        let mut combined_secret = Vec::with_capacity(32 + kyber_shared.map_or(0, |k| k.len()));
        combined_secret.extend_from_slice(x25519_shared.as_bytes());
        
        if let Some(k_shared) = kyber_shared {
            combined_secret.extend_from_slice(k_shared);
        }
        
        // Use HKDF to derive encryption key from combined shared secret
        let hk = Hkdf::<Sha256>::new(None, &combined_secret);
        
        // Create info by concatenating client and server randoms
        let mut info = Vec::with_capacity(64);
        info.extend_from_slice(client_random);
        info.extend_from_slice(server_random);
        
        let mut okm = [0u8; 32];
        hk.expand(&info, &mut okm).expect("HKDF expand failed");
        
        self.shared_secret = Some(*Key::from_slice(&okm));
    }

    pub fn encrypt(&self, nonce_val: u64, plaintext: &[u8]) -> Result<Vec<u8>> {
        let key_bytes = self.shared_secret.as_ref().ok_or_else(|| anyhow::anyhow!("Handshake not completed"))?;
        
        // Create 12-byte nonce (96 bits) from u64
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[4..].copy_from_slice(&nonce_val.to_be_bytes());
        let nonce = Nonce::from_slice(&nonce_bytes);

        match self.cipher_suite {
            CipherSuite::ChaCha20Poly1305 => {
                let cipher = ChaCha20Poly1305::new(key_bytes);
                cipher.encrypt(nonce, plaintext)
                    .map_err(|_| anyhow::anyhow!("Encryption failed"))
            },
            CipherSuite::Aes256Gcm => {
                let cipher = Aes256Gcm::new(key_bytes);
                cipher.encrypt(nonce, plaintext)
                    .map_err(|_| anyhow::anyhow!("Encryption failed"))
            }
        }
    }

    pub fn decrypt(&self, nonce_val: u64, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let key_bytes = self.shared_secret.as_ref().ok_or_else(|| anyhow::anyhow!("Handshake not completed"))?;
        
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[4..].copy_from_slice(&nonce_val.to_be_bytes());
        let nonce = Nonce::from_slice(&nonce_bytes);

        match self.cipher_suite {
            CipherSuite::ChaCha20Poly1305 => {
                let cipher = ChaCha20Poly1305::new(key_bytes);
                cipher.decrypt(nonce, ciphertext)
                    .map_err(|_| anyhow::anyhow!("Decryption failed"))
            },
            CipherSuite::Aes256Gcm => {
                let cipher = Aes256Gcm::new(key_bytes);
                cipher.decrypt(nonce, ciphertext)
                    .map_err(|_| anyhow::anyhow!("Decryption failed"))
            }
        }
    }

    /// Export session state for 0-RTT resumption
    pub fn export_session_state(&self) -> Result<Vec<u8>> {
        let key = self.shared_secret.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No session state to export"))?;
        
        // In production, this should be encrypted with a server-side key
        // For now, just return the raw key bytes
        Ok(key.as_slice().to_vec())
    }

    /// Import session state for 0-RTT resumption
    pub fn import_session_state(&mut self, state: &[u8]) -> Result<()> {
        if state.len() != 32 {
            return Err(anyhow::anyhow!("Invalid session state length"));
        }
        
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(state);
        self.shared_secret = Some(*Key::from_slice(&key_bytes));
        
        Ok(())
    }
}
