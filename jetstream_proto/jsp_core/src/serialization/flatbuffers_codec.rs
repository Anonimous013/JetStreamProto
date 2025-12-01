use flatbuffers::FlatBufferBuilder;
use crate::types::header::Header;
use crate::types::delivery::DeliveryMode;
use crate::types::connection_id::ConnectionId;
use super::generated::messages_generated::jet_stream as fb;
use anyhow::Result;

/// FlatBuffers codec for zero-copy serialization
pub struct FlatBuffersCodec;

impl FlatBuffersCodec {
    /// Serialize Header to FlatBuffers
    pub fn serialize_header(header: &Header) -> Vec<u8> {
        let mut builder = FlatBufferBuilder::new();
        
        // Convert DeliveryMode
        let delivery_mode = match header.delivery_mode {
            DeliveryMode::Reliable => fb::DeliveryMode::Reliable,
            DeliveryMode::PartiallyReliable { .. } => fb::DeliveryMode::PartiallyReliable,
            DeliveryMode::BestEffort => fb::DeliveryMode::BestEffort,
        };
        
        // Extract TTL for PartiallyReliable mode
        let ttl_ms = match header.delivery_mode {
            DeliveryMode::PartiallyReliable { ttl_ms } => ttl_ms,
            _ => 0,
        };
        
        // Create Header
        let fb_header = fb::Header::create(&mut builder, &fb::HeaderArgs {
            stream_id: header.stream_id,
            msg_type: header.msg_type,
            flags: header.flags,
            sequence: header.sequence,
            timestamp: header.timestamp,
            nonce: header.nonce,
            delivery_mode,
            ttl_ms,
            piggybacked_ack: header.piggybacked_ack,
            payload_len: header.payload_len,
            connection_id: header.connection_id.map(|id| id.as_u64()),
        });
        
        builder.finish(fb_header, None);
        builder.finished_data().to_vec()
    }
    
    /// Deserialize Header from FlatBuffers (zero-copy)
    pub fn deserialize_header(data: &[u8]) -> Result<Header> {
        let fb_header = flatbuffers::root::<fb::Header>(data)
            .map_err(|e| anyhow::anyhow!("Failed to parse FlatBuffers header: {}", e))?;
        
        // Convert DeliveryMode
        let delivery_mode = match fb_header.delivery_mode() {
            fb::DeliveryMode::Reliable => DeliveryMode::Reliable,
            fb::DeliveryMode::PartiallyReliable => {
                DeliveryMode::PartiallyReliable { ttl_ms: fb_header.ttl_ms() }
            }
            fb::DeliveryMode::BestEffort => DeliveryMode::BestEffort,
            _ => DeliveryMode::Reliable, // Default fallback
        };
        
        Ok(Header {
            stream_id: fb_header.stream_id(),
            msg_type: fb_header.msg_type(),
            flags: fb_header.flags(),
            sequence: fb_header.sequence(),
            timestamp: fb_header.timestamp(),
            nonce: fb_header.nonce(),
            delivery_mode,
            piggybacked_ack: fb_header.piggybacked_ack(),
            payload_len: fb_header.payload_len(),
            connection_id: fb_header.connection_id().map(ConnectionId::from_u64),
        })
    }

    /// Serialize ClientHello to FlatBuffers
    pub fn serialize_client_hello(hello: &crate::types::handshake::ClientHello) -> Vec<u8> {
        let mut builder = FlatBufferBuilder::new();
        
        // Create vectors
        let random = fb::Bytes32::new(&hello.random);
        let x25519_pub = fb::Bytes32::new(&hello.public_key);
        let cipher_suites = builder.create_vector(&hello.cipher_suites);
        let kyber_pub = builder.create_vector(&hello.kyber_public_key);
        let supported_formats = builder.create_vector(&hello.supported_formats);
        
        // Create ClientHello
        let fb_hello = fb::ClientHello::create(&mut builder, &fb::ClientHelloArgs {
            version: hello.version,
            client_random: Some(&random),
            session_id: hello.session_id,
            supported_ciphers: Some(cipher_suites),
            x25519_public_key: Some(&x25519_pub),
            kyber_public_key: Some(kyber_pub),
            nonce: hello.nonce,
            timestamp: hello.timestamp,
            connection_id: Some(hello.connection_id.as_u64()),
            supported_formats: Some(supported_formats),
        });
        
        builder.finish(fb_hello, None);
        builder.finished_data().to_vec()
    }
    
    /// Deserialize ClientHello from FlatBuffers
    pub fn deserialize_client_hello(data: &[u8]) -> Result<crate::types::handshake::ClientHello> {
        let fb_hello = flatbuffers::root::<fb::ClientHello>(data)
            .map_err(|e| anyhow::anyhow!("Failed to parse FlatBuffers ClientHello: {}", e))?;
            
        Ok(crate::types::handshake::ClientHello {
            version: fb_hello.version(),
            random: fb_hello.client_random().map(|r| r.0).unwrap_or([0; 32]),
            session_id: fb_hello.session_id(),
            cipher_suites: fb_hello.supported_ciphers().map(|v| v.iter().collect()).unwrap_or_default(),
            public_key: fb_hello.x25519_public_key().map(|k| k.0).unwrap_or([0; 32]),
            kyber_public_key: fb_hello.kyber_public_key().map(|v| v.iter().collect()).unwrap_or_default(),
            nonce: fb_hello.nonce(),
            timestamp: fb_hello.timestamp(),
            connection_id: ConnectionId::from_u64(fb_hello.connection_id().unwrap_or(0)),
            supported_formats: fb_hello.supported_formats().map(|v| v.iter().collect()).unwrap_or_default(),
        })
    }

    /// Serialize ServerHello to FlatBuffers
    pub fn serialize_server_hello(hello: &crate::types::handshake::ServerHello) -> Vec<u8> {
        let mut builder = FlatBufferBuilder::new();
        
        // Create vectors
        let random = fb::Bytes32::new(&hello.random);
        let x25519_pub = fb::Bytes32::new(&hello.public_key);
        let kyber_ciphertext = builder.create_vector(&hello.kyber_ciphertext);
        
        // Create ServerHello
        let fb_hello = fb::ServerHello::create(&mut builder, &fb::ServerHelloArgs {
            version: hello.version,
            server_random: Some(&random),
            session_id: hello.session_id,
            selected_cipher: hello.cipher_suite,
            x25519_public_key: Some(&x25519_pub),
            kyber_ciphertext: Some(kyber_ciphertext),
            connection_id: Some(hello.connection_id.as_u64()),
            selected_format: hello.selected_format,
        });
        
        builder.finish(fb_hello, None);
        builder.finished_data().to_vec()
    }
    
    /// Deserialize ServerHello from FlatBuffers
    pub fn deserialize_server_hello(data: &[u8]) -> Result<crate::types::handshake::ServerHello> {
        let fb_hello = flatbuffers::root::<fb::ServerHello>(data)
            .map_err(|e| anyhow::anyhow!("Failed to parse FlatBuffers ServerHello: {}", e))?;
            
        Ok(crate::types::handshake::ServerHello {
            version: fb_hello.version(),
            random: fb_hello.server_random().map(|r| r.0).unwrap_or([0; 32]),
            session_id: fb_hello.session_id(),
            cipher_suite: fb_hello.selected_cipher(),
            public_key: fb_hello.x25519_public_key().map(|k| k.0).unwrap_or([0; 32]),
            kyber_ciphertext: fb_hello.kyber_ciphertext().map(|v| v.iter().collect()).unwrap_or_default(),
            connection_id: ConnectionId::from_u64(fb_hello.connection_id().unwrap_or(0)),
            selected_format: fb_hello.selected_format(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_roundtrip() {
        let header = Header {
            stream_id: 42,
            msg_type: 0x01,
            flags: 0,
            sequence: 1000,
            timestamp: 123456789,
            nonce: 999,
            delivery_mode: DeliveryMode::Reliable,
            piggybacked_ack: Some(500),
            payload_len: Some(1024),
            connection_id: Some(ConnectionId::from_u64(12345)),
        };
        
        let serialized = FlatBuffersCodec::serialize_header(&header);
        let deserialized = FlatBuffersCodec::deserialize_header(&serialized).unwrap();
        
        assert_eq!(header.stream_id, deserialized.stream_id);
        assert_eq!(header.msg_type, deserialized.msg_type);
        assert_eq!(header.sequence, deserialized.sequence);
        assert_eq!(header.timestamp, deserialized.timestamp);
        assert_eq!(header.payload_len, deserialized.payload_len);
    }
    
    #[test]
    fn test_partially_reliable_mode() {
        let header = Header {
            stream_id: 1,
            msg_type: 0x02,
            flags: 0,
            sequence: 100,
            timestamp: 1000,
            nonce: 50,
            delivery_mode: DeliveryMode::PartiallyReliable { ttl_ms: 5000 },
            piggybacked_ack: None,
            payload_len: Some(512),
            connection_id: None,
        };
        
        let serialized = FlatBuffersCodec::serialize_header(&header);
        let deserialized = FlatBuffersCodec::deserialize_header(&serialized).unwrap();
        
        match deserialized.delivery_mode {
            DeliveryMode::PartiallyReliable { ttl_ms } => {
                assert_eq!(ttl_ms, 5000);
            }
            _ => panic!("Expected PartiallyReliable mode"),
        }
    }

    #[test]
    fn test_client_hello_roundtrip() {
        let hello = crate::types::handshake::ClientHello {
            version: 1,
            random: [1u8; 32],
            session_id: 12345,
            cipher_suites: vec![1, 2, 3],
            public_key: [2u8; 32],
            kyber_public_key: vec![3u8; 100],
            nonce: 999,
            timestamp: 88888,
            connection_id: ConnectionId::from_u64(54321),
            supported_formats: vec![0, 1],
        };
        
        let serialized = FlatBuffersCodec::serialize_client_hello(&hello);
        let deserialized = FlatBuffersCodec::deserialize_client_hello(&serialized).unwrap();
        
        assert_eq!(hello.version, deserialized.version);
        assert_eq!(hello.random, deserialized.random);
        assert_eq!(hello.session_id, deserialized.session_id);
        assert_eq!(hello.cipher_suites, deserialized.cipher_suites);
        assert_eq!(hello.public_key, deserialized.public_key);
        assert_eq!(hello.kyber_public_key, deserialized.kyber_public_key);
        assert_eq!(hello.nonce, deserialized.nonce);
        assert_eq!(hello.timestamp, deserialized.timestamp);
        assert_eq!(hello.connection_id, deserialized.connection_id);
        assert_eq!(hello.supported_formats, deserialized.supported_formats);
    }

    #[test]
    fn test_server_hello_roundtrip() {
        let hello = crate::types::handshake::ServerHello {
            version: 1,
            random: [4u8; 32],
            session_id: 12345,
            cipher_suite: 2,
            public_key: [5u8; 32],
            kyber_ciphertext: vec![6u8; 100],
            connection_id: ConnectionId::from_u64(98765),
            selected_format: 1,
        };
        
        let serialized = FlatBuffersCodec::serialize_server_hello(&hello);
        let deserialized = FlatBuffersCodec::deserialize_server_hello(&serialized).unwrap();
        
        assert_eq!(hello.version, deserialized.version);
        assert_eq!(hello.random, deserialized.random);
        assert_eq!(hello.session_id, deserialized.session_id);
        assert_eq!(hello.cipher_suite, deserialized.cipher_suite);
        assert_eq!(hello.public_key, deserialized.public_key);
        assert_eq!(hello.kyber_ciphertext, deserialized.kyber_ciphertext);
        assert_eq!(hello.connection_id, deserialized.connection_id);
        assert_eq!(hello.selected_format, deserialized.selected_format);
    }
}
