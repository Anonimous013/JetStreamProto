use hkdf::Hkdf;
use sha2::Sha256;
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce
};
use anyhow::Result;
use std::collections::HashMap;

const MAX_SKIP: usize = 1000; // Maximum number of message keys to skip

/// Double Ratchet implementation for forward secrecy and post-compromise security
/// Based on the Signal Protocol specification
pub struct DoubleRatchet {
    /// Root key for deriving new chain keys
    root_key: [u8; 32],
    
    /// Sending chain key
    sending_chain_key: Option<[u8; 32]>,
    
    /// Receiving chain key
    receiving_chain_key: Option<[u8; 32]>,
    
    /// Sending message number
    sending_message_number: u32,
    
    /// Receiving message number
    receiving_message_number: u32,
    
    /// Previous sending chain length (for header)
    previous_chain_length: u32,
    
    /// Skipped message keys for out-of-order messages
    skipped_message_keys: HashMap<(u32, u32), [u8; 32]>,
    
    /// Our current DH keypair (ephemeral)
    dh_secret: x25519_dalek::StaticSecret,
    dh_public: x25519_dalek::PublicKey,
    
    /// Peer's current DH public key
    peer_dh_public: Option<x25519_dalek::PublicKey>,
}

impl DoubleRatchet {
    /// Initialize as Alice (initiator) with shared secret from handshake
    pub fn new_alice(shared_secret: &[u8; 32], peer_public_key: &[u8; 32]) -> Self {
        let dh_secret = x25519_dalek::StaticSecret::random_from_rng(rand_core::OsRng);
        let dh_public = x25519_dalek::PublicKey::from(&dh_secret);
        
        let peer_dh_public = x25519_dalek::PublicKey::from(*peer_public_key);
        
        Self {
            root_key: *shared_secret,
            sending_chain_key: None,
            receiving_chain_key: None,
            sending_message_number: 0,
            receiving_message_number: 0,
            previous_chain_length: 0,
            skipped_message_keys: HashMap::new(),
            dh_secret,
            dh_public,
            peer_dh_public: Some(peer_dh_public),
        }
    }
    
    /// Initialize as Bob (responder) with shared secret from handshake
    pub fn new_bob(shared_secret: &[u8; 32]) -> Self {
        let dh_secret = x25519_dalek::StaticSecret::random_from_rng(rand_core::OsRng);
        let dh_public = x25519_dalek::PublicKey::from(&dh_secret);
        
        Self {
            root_key: *shared_secret,
            sending_chain_key: None,
            receiving_chain_key: None,
            sending_message_number: 0,
            receiving_message_number: 0,
            previous_chain_length: 0,
            skipped_message_keys: HashMap::new(),
            dh_secret,
            dh_public,
            peer_dh_public: None,
        }
    }
    
    /// Get our current public DH key for sending to peer
    pub fn public_key(&self) -> &[u8; 32] {
        self.dh_public.as_bytes()
    }
    
    /// Encrypt a message
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<EncryptedMessage> {
        // Initialize sending chain if this is the first message
        if self.sending_chain_key.is_none() {
            if let Some(peer_public) = self.peer_dh_public {
                let dh_output = self.dh_secret.diffie_hellman(&peer_public);
                let (new_root_key, sending_chain_key) = self.kdf_rk(&self.root_key, dh_output.as_bytes());
                self.root_key = new_root_key;
                self.sending_chain_key = Some(sending_chain_key);
            } else {
                return Err(anyhow::anyhow!("Cannot encrypt: peer public key not set"));
            }
        }
        
        let message_key = self.get_sending_message_key()?;
        
        let header = MessageHeader {
            dh_public: *self.dh_public.as_bytes(),
            previous_chain_length: self.previous_chain_length,
            message_number: self.sending_message_number,
        };
        
        self.sending_message_number += 1;
        
        // Encrypt with message key
        let ciphertext = self.encrypt_with_key(&message_key, plaintext)?;
        
        Ok(EncryptedMessage {
            header,
            ciphertext,
        })
    }
    
    /// Decrypt a message
    pub fn decrypt(&mut self, encrypted: &EncryptedMessage) -> Result<Vec<u8>> {
        // Check if we have a skipped message key
        let key_id = (encrypted.header.previous_chain_length, encrypted.header.message_number);
        if let Some(message_key) = self.skipped_message_keys.remove(&key_id) {
            return self.decrypt_with_key(&message_key, &encrypted.ciphertext);
        }
        
        // Check if we need to perform DH ratchet
        let peer_public = x25519_dalek::PublicKey::from(encrypted.header.dh_public);
        if self.peer_dh_public.as_ref() != Some(&peer_public) {
            self.skip_message_keys(encrypted.header.previous_chain_length)?;
            self.dh_ratchet_step(&peer_public);
        }
        
        // Skip message keys if needed
        self.skip_message_keys(encrypted.header.message_number)?;
        
        let message_key = self.get_receiving_message_key()?;
        self.receiving_message_number += 1;
        
        self.decrypt_with_key(&message_key, &encrypted.ciphertext)
    }
    
    /// Perform DH ratchet step
    fn dh_ratchet_step(&mut self, peer_public: &x25519_dalek::PublicKey) {
        self.peer_dh_public = Some(*peer_public);
        
        // Derive receiving chain key
        let dh_output = self.dh_secret.diffie_hellman(peer_public);
        let (new_root_key, receiving_chain_key) = self.kdf_rk(&self.root_key, dh_output.as_bytes());
        self.root_key = new_root_key;
        self.receiving_chain_key = Some(receiving_chain_key);
        self.receiving_message_number = 0;
        
        // Generate new DH keypair
        self.previous_chain_length = self.sending_message_number;
        self.dh_secret = x25519_dalek::StaticSecret::random_from_rng(rand_core::OsRng);
        self.dh_public = x25519_dalek::PublicKey::from(&self.dh_secret);
        
        // Derive sending chain key
        let dh_output = self.dh_secret.diffie_hellman(peer_public);
        let (new_root_key, sending_chain_key) = self.kdf_rk(&self.root_key, dh_output.as_bytes());
        self.root_key = new_root_key;
        self.sending_chain_key = Some(sending_chain_key);
        self.sending_message_number = 0;
    }
    
    /// KDF for root key
    fn kdf_rk(&self, root_key: &[u8; 32], dh_output: &[u8]) -> ([u8; 32], [u8; 32]) {
        let hk = Hkdf::<Sha256>::new(Some(root_key), dh_output);
        let mut output = [0u8; 64];
        hk.expand(b"JetStreamProto-DoubleRatchet", &mut output)
            .expect("HKDF expand failed");
        
        let mut new_root_key = [0u8; 32];
        let mut chain_key = [0u8; 32];
        new_root_key.copy_from_slice(&output[..32]);
        chain_key.copy_from_slice(&output[32..]);
        
        (new_root_key, chain_key)
    }
    
    /// KDF for chain key
    fn kdf_ck(&self, chain_key: &[u8; 32]) -> ([u8; 32], [u8; 32]) {
        let hk = Hkdf::<Sha256>::new(None, chain_key);
        let mut output = [0u8; 64];
        hk.expand(b"JetStreamProto-ChainKey", &mut output)
            .expect("HKDF expand failed");
        
        let mut new_chain_key = [0u8; 32];
        let mut message_key = [0u8; 32];
        new_chain_key.copy_from_slice(&output[..32]);
        message_key.copy_from_slice(&output[32..]);
        
        (new_chain_key, message_key)
    }
    
    /// Get next sending message key
    fn get_sending_message_key(&mut self) -> Result<[u8; 32]> {
        let chain_key = self.sending_chain_key
            .ok_or_else(|| anyhow::anyhow!("Sending chain not initialized"))?;
        
        let (new_chain_key, message_key) = self.kdf_ck(&chain_key);
        self.sending_chain_key = Some(new_chain_key);
        
        Ok(message_key)
    }
    
    /// Get next receiving message key
    fn get_receiving_message_key(&mut self) -> Result<[u8; 32]> {
        let chain_key = self.receiving_chain_key
            .ok_or_else(|| anyhow::anyhow!("Receiving chain not initialized"))?;
        
        let (new_chain_key, message_key) = self.kdf_ck(&chain_key);
        self.receiving_chain_key = Some(new_chain_key);
        
        Ok(message_key)
    }
    
    /// Skip message keys for out-of-order delivery
    fn skip_message_keys(&mut self, until: u32) -> Result<()> {
        if self.receiving_message_number + (MAX_SKIP as u32) < until {
            return Err(anyhow::anyhow!("Too many skipped messages"));
        }
        
        if let Some(mut chain_key) = self.receiving_chain_key {
            while self.receiving_message_number < until {
                let (new_chain_key, message_key) = self.kdf_ck(&chain_key);
                chain_key = new_chain_key; // Update chain_key for next iteration
                self.receiving_chain_key = Some(chain_key);
                
                let key_id = (self.previous_chain_length, self.receiving_message_number);
                self.skipped_message_keys.insert(key_id, message_key);
                
                self.receiving_message_number += 1;
            }
        }
        
        Ok(())
    }
    
    /// Encrypt with a specific message key
    fn encrypt_with_key(&self, key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>> {
        let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
        let nonce = Nonce::from_slice(&[0u8; 12]); // In production, use proper nonce
        
        cipher.encrypt(nonce, plaintext)
            .map_err(|_| anyhow::anyhow!("Encryption failed"))
    }
    
    /// Decrypt with a specific message key
    fn decrypt_with_key(&self, key: &[u8; 32], ciphertext: &[u8]) -> Result<Vec<u8>> {
        let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
        let nonce = Nonce::from_slice(&[0u8; 12]); // In production, use proper nonce
        
        cipher.decrypt(nonce, ciphertext)
            .map_err(|_| anyhow::anyhow!("Decryption failed"))
    }
}

/// Message header for Double Ratchet
#[derive(Debug, Clone)]
pub struct MessageHeader {
    pub dh_public: [u8; 32],
    pub previous_chain_length: u32,
    pub message_number: u32,
}

/// Encrypted message with header
#[derive(Debug, Clone)]
pub struct EncryptedMessage {
    pub header: MessageHeader,
    pub ciphertext: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_double_ratchet_basic() {
        let shared_secret = [42u8; 32];
        
        // Bob creates his ratchet first and shares his public key
        let mut bob = DoubleRatchet::new_bob(&shared_secret);
        let bob_public_key = *bob.public_key();
        
        // Alice creates her ratchet with Bob's public key
        let mut alice = DoubleRatchet::new_alice(&shared_secret, &bob_public_key);
        
        // Alice sends first message
        let plaintext1 = b"Hello from Alice!";
        let encrypted1 = alice.encrypt(plaintext1).unwrap();
        
        // Bob receives and decrypts
        let decrypted1 = bob.decrypt(&encrypted1).unwrap();
        assert_eq!(decrypted1, plaintext1);
        
        // Bob sends response
        let plaintext2 = b"Hello from Bob!";
        let encrypted2 = bob.encrypt(plaintext2).unwrap();
        
        // Alice receives and decrypts
        let decrypted2 = alice.decrypt(&encrypted2).unwrap();
        assert_eq!(decrypted2, plaintext2);
    }

    #[test]
    fn test_out_of_order_messages() {
        let shared_secret = [42u8; 32];
        
        // Bob creates his ratchet first and shares his public key
        let mut bob = DoubleRatchet::new_bob(&shared_secret);
        let bob_public_key = *bob.public_key();
        
        // Alice creates her ratchet with Bob's public key
        let mut alice = DoubleRatchet::new_alice(&shared_secret, &bob_public_key);
        
        // Alice sends multiple messages in same chain
        let msg1 = alice.encrypt(b"Message 1").unwrap();
        let msg2 = alice.encrypt(b"Message 2").unwrap();
        let msg3 = alice.encrypt(b"Message 3").unwrap();
        
        // Bob receives out of order: 3, 1, 2
        let dec3 = bob.decrypt(&msg3).unwrap();
        assert_eq!(dec3, b"Message 3");
        
        let dec1 = bob.decrypt(&msg1).unwrap();
        assert_eq!(dec1, b"Message 1");
        
        let dec2 = bob.decrypt(&msg2).unwrap();
        assert_eq!(dec2, b"Message 2");
    }
}
