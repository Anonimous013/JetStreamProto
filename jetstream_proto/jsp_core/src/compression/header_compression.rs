use crate::types::header::Header;
use crate::types::delivery::DeliveryMode;
use crate::types::connection_id::ConnectionId;
use super::varint::{encode_varint, decode_varint};

/// Header compression flags
const FLAG_STREAM_ID_CHANGED: u8 = 0x01;
const FLAG_MSG_TYPE_PRESENT: u8 = 0x02;
const FLAG_SEQUENCE_DELTA: u8 = 0x04;
const FLAG_TIMESTAMP_DELTA: u8 = 0x08;
const FLAG_NONCE_DELTA: u8 = 0x10;
const FLAG_HAS_PIGGYBACKED_ACK: u8 = 0x20;
const FLAG_HAS_PAYLOAD_LEN: u8 = 0x40;

/// Header compressor with stateful delta encoding
pub struct HeaderCompressor {
    last_header: Option<Header>,
}

impl HeaderCompressor {
    pub fn new() -> Self {
        Self { last_header: None }
    }
    
    /// Compress a header using delta encoding
    pub fn compress(&mut self, header: &Header) -> Vec<u8> {
        let mut compressed = Vec::new();
        let mut flags: u8 = 0;
        
        // Determine what changed from last header
        let stream_id_changed = self.last_header.as_ref()
            .map(|last| last.stream_id != header.stream_id)
            .unwrap_or(true);
        
        let msg_type_changed = self.last_header.as_ref()
            .map(|last| last.msg_type != header.msg_type)
            .unwrap_or(true);
        
        // Set flags
        if stream_id_changed {
            flags |= FLAG_STREAM_ID_CHANGED;
        }
        if msg_type_changed {
            flags |= FLAG_MSG_TYPE_PRESENT;
        }
        if self.last_header.is_some() {
            flags |= FLAG_SEQUENCE_DELTA;
            flags |= FLAG_TIMESTAMP_DELTA;
            flags |= FLAG_NONCE_DELTA;
        }
        if header.piggybacked_ack.is_some() {
            flags |= FLAG_HAS_PIGGYBACKED_ACK;
        }
        if header.payload_len.is_some() {
            flags |= FLAG_HAS_PAYLOAD_LEN;
        }
        
        compressed.push(flags);
        
        // Encode changed fields
        if stream_id_changed {
            compressed.extend(encode_varint(header.stream_id as u64));
        }
        
        if msg_type_changed {
            compressed.push(header.msg_type);
        }
        
        // Encode deltas or full values
        if let Some(last) = &self.last_header {
            // Delta encoding
            let seq_delta = header.sequence.wrapping_sub(last.sequence);
            compressed.extend(encode_varint(seq_delta));
            
            let ts_delta = header.timestamp.wrapping_sub(last.timestamp);
            compressed.extend(encode_varint(ts_delta));
            
            let nonce_delta = header.nonce.wrapping_sub(last.nonce);
            compressed.extend(encode_varint(nonce_delta));
        } else {
            // First packet, send full values
            compressed.extend(encode_varint(header.sequence));
            compressed.extend(encode_varint(header.timestamp));
            compressed.extend(encode_varint(header.nonce));
        }
        
        // Flags and delivery mode
        compressed.push(header.flags);
        
        // Encode delivery mode
        match header.delivery_mode {
            DeliveryMode::Reliable => compressed.push(0),
            DeliveryMode::PartiallyReliable { ttl_ms } => {
                compressed.push(1);
                compressed.extend(encode_varint(ttl_ms as u64));
            },
            DeliveryMode::BestEffort => compressed.push(2),
        }
        
        // Optional fields
        if let Some(ack) = header.piggybacked_ack {
            compressed.extend(encode_varint(ack));
        }
        
        if let Some(len) = header.payload_len {
            compressed.extend(encode_varint(len as u64));
        }
        
        // Connection ID (if present, send once then assume same)
        if self.last_header.is_none() {
            if let Some(cid) = &header.connection_id {
                compressed.extend(encode_varint(cid.as_u64()));
            }
        }
        
        // Update last header
        self.last_header = Some(*header);
        
        compressed
    }
    
    /// Decompress a header
    pub fn decompress(&mut self, compressed: &[u8]) -> Result<Header, &'static str> {
        if compressed.is_empty() {
            return Err("Empty compressed data");
        }
        
        let flags = compressed[0];
        let mut pos = 1;
        
        // Decode stream_id
        let stream_id = if flags & FLAG_STREAM_ID_CHANGED != 0 {
            let (val, consumed) = decode_varint(&compressed[pos..])?;
            pos += consumed;
            val as u32
        } else {
            self.last_header.as_ref().map(|h| h.stream_id).unwrap_or(0)
        };
        
        // Decode msg_type
        let msg_type = if flags & FLAG_MSG_TYPE_PRESENT != 0 {
            let val = compressed[pos];
            pos += 1;
            val
        } else {
            self.last_header.as_ref().map(|h| h.msg_type).unwrap_or(0)
        };
        
        // Decode sequence, timestamp, nonce
        let (sequence, timestamp, nonce) = if let Some(last) = &self.last_header {
            let (seq_delta, consumed) = decode_varint(&compressed[pos..])?;
            pos += consumed;
            let sequence = last.sequence.wrapping_add(seq_delta);
            
            let (ts_delta, consumed) = decode_varint(&compressed[pos..])?;
            pos += consumed;
            let timestamp = last.timestamp.wrapping_add(ts_delta);
            
            let (nonce_delta, consumed) = decode_varint(&compressed[pos..])?;
            pos += consumed;
            let nonce = last.nonce.wrapping_add(nonce_delta);
            
            (sequence, timestamp, nonce)
        } else {
            let (sequence, consumed) = decode_varint(&compressed[pos..])?;
            pos += consumed;
            
            let (timestamp, consumed) = decode_varint(&compressed[pos..])?;
            pos += consumed;
            
            let (nonce, consumed) = decode_varint(&compressed[pos..])?;
            pos += consumed;
            
            (sequence, timestamp, nonce)
        };
        
        // Decode flags and delivery_mode
        let header_flags = compressed[pos];
        pos += 1;
        
        let delivery_mode_byte = compressed[pos];
        pos += 1;
        
        let delivery_mode = match delivery_mode_byte {
            0 => DeliveryMode::Reliable,
            1 => {
                let (ttl_ms, consumed) = decode_varint(&compressed[pos..])?;
                pos += consumed;
                DeliveryMode::PartiallyReliable { ttl_ms: ttl_ms as u32 }
            },
            2 => DeliveryMode::BestEffort,
            _ => DeliveryMode::Reliable,
        };
        
        // Decode optional fields
        let piggybacked_ack = if flags & FLAG_HAS_PIGGYBACKED_ACK != 0 {
            let (val, consumed) = decode_varint(&compressed[pos..])?;
            pos += consumed;
            Some(val)
        } else {
            None
        };
        
        let payload_len = if flags & FLAG_HAS_PAYLOAD_LEN != 0 {
            let (val, consumed) = decode_varint(&compressed[pos..])?;
            pos += consumed;
            Some(val as u32)
        } else {
            None
        };
        
        // Connection ID (only in first packet)
        let connection_id = if self.last_header.is_none() && pos < compressed.len() {
            let (val, _consumed) = decode_varint(&compressed[pos..])?;
            Some(ConnectionId::from_u64(val))
        } else {
            self.last_header.as_ref().and_then(|h| h.connection_id)
        };
        
        let header = Header {
            stream_id,
            msg_type,
            flags: header_flags,
            sequence,
            timestamp,
            nonce,
            delivery_mode,
            piggybacked_ack,
            payload_len,
            connection_id,
        };
        
        self.last_header = Some(header);
        Ok(header)
    }
    
    /// Reset compression state
    pub fn reset(&mut self) {
        self.last_header = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_decompress_first_packet() {
        let mut compressor = HeaderCompressor::new();
        
        let header = Header {
            stream_id: 0,
            msg_type: 0x00,
            flags: 0,
            sequence: 1,
            timestamp: 1000,
            nonce: 100,
            delivery_mode: DeliveryMode::Reliable,
            piggybacked_ack: None,
            payload_len: Some(1024),
            connection_id: Some(ConnectionId::from_u64(12345)),
        };
        
        let compressed = compressor.compress(&header);
        
        let mut decompressor = HeaderCompressor::new();
        let decompressed = decompressor.decompress(&compressed).unwrap();
        
        assert_eq!(decompressed.stream_id, header.stream_id);
        assert_eq!(decompressed.sequence, header.sequence);
        assert_eq!(decompressed.payload_len, header.payload_len);
    }

    #[test]
    fn test_delta_encoding() {
        let mut compressor = HeaderCompressor::new();
        
        let header1 = Header {
            stream_id: 0,
            msg_type: 0x00,
            flags: 0,
            sequence: 1,
            timestamp: 1000,
            nonce: 100,
            delivery_mode: DeliveryMode::Reliable,
            piggybacked_ack: None,
            payload_len: Some(1024),
            connection_id: Some(ConnectionId::from_u64(12345)),
        };
        
        let compressed1 = compressor.compress(&header1);
        let size1 = compressed1.len();
        
        // Second packet with incremental values
        let header2 = Header {
            stream_id: 0,
            msg_type: 0x00,
            flags: 0,
            sequence: 2,
            timestamp: 1010,
            nonce: 101,
            delivery_mode: DeliveryMode::Reliable,
            piggybacked_ack: None,
            payload_len: Some(1024),
            connection_id: Some(ConnectionId::from_u64(12345)),
        };
        
        let compressed2 = compressor.compress(&header2);
        let size2 = compressed2.len();
        
        // Second packet should be smaller due to delta encoding
        assert!(size2 < size1, "Delta encoded packet should be smaller");
    }
}
