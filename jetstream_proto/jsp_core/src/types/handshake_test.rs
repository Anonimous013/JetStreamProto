#[cfg(test)]
mod tests {
    // use super::*;
    use crate::types::handshake::{ClientHello, ServerHello};
    use crate::types::connection_id::ConnectionId;

    #[test]
    fn test_client_hello_serialization() {
        let hello = ClientHello {
            version: 1,
            random: [1u8; 32],
            session_id: 12345,
            cipher_suites: vec![0x1301, 0x1302],
            public_key: [3u8; 32],
            kyber_public_key: vec![5u8; 800],
            nonce: 999,
            timestamp: 1234567890,
            connection_id: ConnectionId::generate(),
            supported_formats: vec![0, 1], // CBOR and FlatBuffers
        };

        let serialized = serde_cbor::to_vec(&hello).unwrap();
        let deserialized: ClientHello = serde_cbor::from_slice(&serialized).unwrap();

        assert_eq!(deserialized.version, hello.version);
        assert_eq!(deserialized.nonce, hello.nonce);
        assert_eq!(deserialized.timestamp, hello.timestamp);
    }

    #[test]
    fn test_server_hello_serialization() {
        let hello = ServerHello {
            version: 1,
            random: [2u8; 32],
            session_id: 67890,
            cipher_suite: 0x1301,
            public_key: [4u8; 32],
            kyber_ciphertext: vec![6u8; 768],
            connection_id: ConnectionId::generate(),
            selected_format: 0, // CBOR selected
        };

        let serialized = serde_cbor::to_vec(&hello).unwrap();
        let deserialized: ServerHello = serde_cbor::from_slice(&serialized).unwrap();

        assert_eq!(deserialized.version, hello.version);
        assert_eq!(deserialized.session_id, hello.session_id);
    }
}
