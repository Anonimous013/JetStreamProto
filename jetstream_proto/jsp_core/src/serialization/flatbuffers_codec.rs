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
}
