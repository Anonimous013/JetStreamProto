use crate::crypto::{CryptoContext, CipherSuite};

#[test]
fn test_chacha20_encryption() {
    let mut client = CryptoContext::new();
    let mut server = CryptoContext::new();
    
    // Setup shared secret
    let client_random = [0u8; 32];
    let server_random = [0u8; 32];
    
    let (ciphertext, shared) = client.encapsulate_kyber(server.kyber_public_key()).unwrap();
    let server_shared = server.decapsulate_kyber(&ciphertext).unwrap();
    
    assert_eq!(shared, server_shared);
    
    client.derive_shared_secret(server.x25519_public_key(), Some(&shared), &client_random, &server_random);
    server.derive_shared_secret(client.x25519_public_key(), Some(&server_shared), &client_random, &server_random);
    
    // Test encryption
    let plaintext = b"Hello, World!";
    let nonce = 12345;
    
    let encrypted = client.encrypt(nonce, plaintext).unwrap();
    let decrypted = server.decrypt(nonce, &encrypted).unwrap();
    
    assert_eq!(plaintext.to_vec(), decrypted);
}

#[test]
fn test_aes_gcm_encryption() {
    let mut client = CryptoContext::new();
    let mut server = CryptoContext::new();
    
    client.set_cipher_suite(CipherSuite::Aes256Gcm);
    server.set_cipher_suite(CipherSuite::Aes256Gcm);
    
    // Setup shared secret
    let client_random = [0u8; 32];
    let server_random = [0u8; 32];
    
    let (ciphertext, shared) = client.encapsulate_kyber(server.kyber_public_key()).unwrap();
    let server_shared = server.decapsulate_kyber(&ciphertext).unwrap();
    
    client.derive_shared_secret(server.x25519_public_key(), Some(&shared), &client_random, &server_random);
    server.derive_shared_secret(client.x25519_public_key(), Some(&server_shared), &client_random, &server_random);
    
    // Test encryption
    let plaintext = b"Hello, AES-GCM!";
    let nonce = 67890;
    
    let encrypted = client.encrypt(nonce, plaintext).unwrap();
    let decrypted = server.decrypt(nonce, &encrypted).unwrap();
    
    assert_eq!(plaintext.to_vec(), decrypted);
}
