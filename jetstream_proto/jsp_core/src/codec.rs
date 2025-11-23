use crate::types::{frame::Frame, header::Header};
use crate::serialization::FlatBuffersCodec;
use anyhow::Result;
use std::io::Cursor;

/// Serialization format for protocol messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerializationFormat {
    /// CBOR (Concise Binary Object Representation) - default for compatibility
    Cbor,
    /// FlatBuffers - zero-copy, high-performance serialization
    FlatBuffers,
}

impl Default for SerializationFormat {
    fn default() -> Self {
        // Default to CBOR for backward compatibility
        SerializationFormat::Cbor
    }
}

impl SerializationFormat {
    /// Convert to wire format byte
    pub fn to_byte(self) -> u8 {
        match self {
            SerializationFormat::Cbor => 0,
            SerializationFormat::FlatBuffers => 1,
        }
    }
    
    /// Parse from wire format byte
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(SerializationFormat::Cbor),
            1 => Some(SerializationFormat::FlatBuffers),
            _ => None,
        }
    }
}

/// Unified codec supporting multiple serialization formats
pub struct Codec {
    format: SerializationFormat,
}

impl Codec {
    /// Create a new codec with the specified format
    pub fn new(format: SerializationFormat) -> Self {
        Self { format }
    }
    
    /// Create a codec with CBOR format (default)
    pub fn cbor() -> Self {
        Self::new(SerializationFormat::Cbor)
    }
    
    /// Create a codec with FlatBuffers format
    pub fn flatbuffers() -> Self {
        Self::new(SerializationFormat::FlatBuffers)
    }
    
    /// Get the current serialization format
    pub fn format(&self) -> SerializationFormat {
        self.format
    }
    
    /// Set the serialization format
    pub fn set_format(&mut self, format: SerializationFormat) {
        self.format = format;
    }
    
    /// Encode a header using the configured format
    pub fn encode_header(&self, header: &Header) -> Result<Vec<u8>> {
        match self.format {
            SerializationFormat::Cbor => CborCodec::encode_header(header),
            SerializationFormat::FlatBuffers => Ok(FlatBuffersCodec::serialize_header(header)),
        }
    }
    
    /// Decode a header using the configured format
    pub fn decode_header(&self, data: &[u8]) -> Result<Header> {
        match self.format {
            SerializationFormat::Cbor => CborCodec::decode_header(data),
            SerializationFormat::FlatBuffers => FlatBuffersCodec::deserialize_header(data),
        }
    }
    
    /// Encode a frame using the configured format
    pub fn encode_frame(&self, frame: &Frame) -> Result<Vec<u8>> {
        match self.format {
            SerializationFormat::Cbor => CborCodec::encode_frame(frame),
            SerializationFormat::FlatBuffers => {
                // TODO: Implement FlatBuffers frame encoding
                CborCodec::encode_frame(frame)
            }
        }
    }
    
    /// Decode a frame using the configured format
    pub fn decode_frame(&self, data: &[u8]) -> Result<Frame> {
        match self.format {
            SerializationFormat::Cbor => CborCodec::decode_frame(data),
            SerializationFormat::FlatBuffers => {
                // TODO: Implement FlatBuffers frame decoding
                CborCodec::decode_frame(data)
            }
        }
    }
}

impl Default for Codec {
    fn default() -> Self {
        Self::cbor()
    }
}

pub trait ProtocolCodec {
    fn encode_header(header: &Header) -> Result<Vec<u8>>;
    fn decode_header(data: &[u8]) -> Result<Header>;
    fn encode_frame(frame: &Frame) -> Result<Vec<u8>>;
    fn decode_frame(data: &[u8]) -> Result<Frame>;
}

pub struct CborCodec;

impl ProtocolCodec for CborCodec {
    fn encode_header(header: &Header) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        serde_cbor::to_writer(&mut data, header)?;
        Ok(data)
    }

    fn decode_header(data: &[u8]) -> Result<Header> {
        let reader = Cursor::new(data);
        let header = serde_cbor::from_reader(reader)?;
        Ok(header)
    }

    fn encode_frame(frame: &Frame) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        serde_cbor::to_writer(&mut data, frame)?;
        Ok(data)
    }

    fn decode_frame(data: &[u8]) -> Result<Frame> {
        let reader = Cursor::new(data);
        let frame = serde_cbor::from_reader(reader)?;
        Ok(frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::delivery::DeliveryMode;
    use crate::types::connection_id::ConnectionId;
    
    #[test]
    fn test_serialization_format_conversion() {
        assert_eq!(SerializationFormat::Cbor.to_byte(), 0);
        assert_eq!(SerializationFormat::FlatBuffers.to_byte(), 1);
        
        assert_eq!(SerializationFormat::from_byte(0), Some(SerializationFormat::Cbor));
        assert_eq!(SerializationFormat::from_byte(1), Some(SerializationFormat::FlatBuffers));
        assert_eq!(SerializationFormat::from_byte(99), None);
    }
    
    #[test]
    fn test_codec_cbor_header() {
        let codec = Codec::cbor();
        
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
        
        let encoded = codec.encode_header(&header).unwrap();
        let decoded = codec.decode_header(&encoded).unwrap();
        
        assert_eq!(header.stream_id, decoded.stream_id);
        assert_eq!(header.sequence, decoded.sequence);
    }
    
    #[test]
    fn test_codec_flatbuffers_header() {
        let codec = Codec::flatbuffers();
        
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
        
        let encoded = codec.encode_header(&header).unwrap();
        let decoded = codec.decode_header(&encoded).unwrap();
        
        assert_eq!(header.stream_id, decoded.stream_id);
        assert_eq!(header.sequence, decoded.sequence);
    }
    
    #[test]
    fn test_codec_format_switching() {
        let mut codec = Codec::cbor();
        assert_eq!(codec.format(), SerializationFormat::Cbor);
        
        codec.set_format(SerializationFormat::FlatBuffers);
        assert_eq!(codec.format(), SerializationFormat::FlatBuffers);
    }
}
