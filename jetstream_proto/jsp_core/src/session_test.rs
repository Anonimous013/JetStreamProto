#[cfg(test)]
mod tests {
    use crate::session::Session;
    use crate::types::handshake::ClientHello;
    
    #[test]
    fn test_key_exchange() {
        // Simulate client
        let mut client_session = Session::new();
        let client_hello_bytes = client_session.generate_client_hello().unwrap();
        
        // Simulate server
        let mut server_session = Session::new();
        
        // Server processes ClientHello (this stores client_random)
        let client_hello = server_session.process_client_hello(&client_hello_bytes).unwrap();
        
        // Server creates ServerHello with Kyber
        let (server_hello_bytes, kyber_shared) = server_session.generate_server_hello(
            99999,
            0x1303, // ChaCha20-Poly1305
            &client_hello.kyber_public_key,
            &client_hello.supported_formats  // Pass client's supported formats
        ).unwrap();
        
        // Server derives keys
        server_session.derive_keys_from_client_hello(
            &client_hello.public_key,
            Some(&kyber_shared)
        );
        
        // Client processes ServerHello
        client_session.process_server_hello(&server_hello_bytes).unwrap();
        
        // Test encryption/decryption
        let plaintext = b"Hello, JetStreamProto!";
        let nonce = 1u64;
        
        let ciphertext = client_session.crypto.encrypt(nonce, plaintext).unwrap();
        let decrypted = server_session.crypto.decrypt(nonce, &ciphertext).unwrap();
        
        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_key_exchange_aes_gcm() {
        // Simulate client
        let mut client_session = Session::new();
        let client_hello_bytes = client_session.generate_client_hello().unwrap();
        
        // Simulate server
        let mut server_session = Session::new();
        
        // Server processes ClientHello
        let client_hello = server_session.process_client_hello(&client_hello_bytes).unwrap();
        
        // Server creates ServerHello with AES-GCM
        let (server_hello_bytes, kyber_shared) = server_session.generate_server_hello(
            99999,
            0x1302, // AES-256-GCM
            &client_hello.kyber_public_key,
            &client_hello.supported_formats  // Pass client's supported formats
        ).unwrap();
        
        // Server derives keys
        server_session.derive_keys_from_client_hello(
            &client_hello.public_key,
            Some(&kyber_shared)
        );
        
        // Client processes ServerHello
        client_session.process_server_hello(&server_hello_bytes).unwrap();
        
        // Test encryption/decryption
        let plaintext = b"Hello, AES-GCM!";
        let nonce = 1u64;
        
        let ciphertext = client_session.crypto.encrypt(nonce, plaintext).unwrap();
        let decrypted = server_session.crypto.decrypt(nonce, &ciphertext).unwrap();
        
        assert_eq!(plaintext.to_vec(), decrypted);
    }
}
