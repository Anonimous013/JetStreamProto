//! HTTP/3 Frames

use bytes::{Buf, BufMut, Bytes, BytesMut};

/// HTTP/3 frame types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    Data = 0x00,
    Headers = 0x01,
    CancelPush = 0x03,
    Settings = 0x04,
    PushPromise = 0x05,
    Goaway = 0x07,
    MaxPushId = 0x0D,
}

impl FrameType {
    pub fn from_u64(value: u64) -> Option<Self> {
        match value {
            0x00 => Some(Self::Data),
            0x01 => Some(Self::Headers),
            0x03 => Some(Self::CancelPush),
            0x04 => Some(Self::Settings),
            0x05 => Some(Self::PushPromise),
            0x07 => Some(Self::Goaway),
            0x0D => Some(Self::MaxPushId),
            _ => None,
        }
    }
}

/// HTTP/3 frame
#[derive(Debug, Clone)]
pub struct Frame {
    pub frame_type: FrameType,
    pub payload: Bytes,
}

impl Frame {
    /// Create a new frame
    pub fn new(frame_type: FrameType, payload: Bytes) -> Self {
        Self { frame_type, payload }
    }

    /// Create a DATA frame
    pub fn data(data: Bytes) -> Self {
        Self::new(FrameType::Data, data)
    }

    /// Create a HEADERS frame
    pub fn headers(headers: Bytes) -> Self {
        Self::new(FrameType::Headers, headers)
    }

    /// Create a SETTINGS frame
    pub fn settings() -> Self {
        Self::new(FrameType::Settings, Bytes::new())
    }

    /// Encode frame to bytes
    pub fn encode(&self) -> Bytes {
        let mut buf = BytesMut::new();
        
        // Frame type (varint)
        buf.put_u8(self.frame_type as u8);
        
        // Length (varint)
        Self::encode_varint(&mut buf, self.payload.len() as u64);
        
        // Payload
        buf.put(self.payload.clone());
        
        buf.freeze()
    }

    /// Decode frame from bytes
    pub fn decode(mut data: Bytes) -> super::Result<Self> {
        if data.remaining() < 2 {
            return Err(super::Http3Error::InvalidFrame("Too short".to_string()));
        }

        // Frame type
        let frame_type_val = Self::decode_varint(&mut data)?;
        let frame_type = FrameType::from_u64(frame_type_val)
            .ok_or_else(|| super::Http3Error::InvalidFrame(format!("Unknown frame type: {}", frame_type_val)))?;

        // Length
        let length = Self::decode_varint(&mut data)? as usize;

        if data.remaining() < length {
            return Err(super::Http3Error::InvalidFrame("Incomplete payload".to_string()));
        }

        // Payload
        let payload = data.split_to(length);

        Ok(Self { frame_type, payload })
    }

    /// Encode variable-length integer
    fn encode_varint(buf: &mut BytesMut, value: u64) {
        if value < 64 {
            buf.put_u8(value as u8);
        } else if value < 16384 {
            buf.put_u16((value as u16) | 0x4000);
        } else if value < 1073741824 {
            buf.put_u32((value as u32) | 0x80000000);
        } else {
            buf.put_u64(value | 0xC000000000000000);
        }
    }

    /// Decode variable-length integer
    fn decode_varint(data: &mut Bytes) -> super::Result<u64> {
        if data.remaining() < 1 {
            return Err(super::Http3Error::InvalidFrame("No data for varint".to_string()));
        }

        let first = data.get_u8();
        let prefix = first >> 6;

        match prefix {
            0 => Ok(first as u64),
            1 => {
                if data.remaining() < 1 {
                    return Err(super::Http3Error::InvalidFrame("Incomplete varint".to_string()));
                }
                Ok((((first & 0x3F) as u64) << 8) | data.get_u8() as u64)
            }
            2 => {
                if data.remaining() < 3 {
                    return Err(super::Http3Error::InvalidFrame("Incomplete varint".to_string()));
                }
                let mut val = ((first & 0x3F) as u64) << 24;
                val |= (data.get_u8() as u64) << 16;
                val |= (data.get_u8() as u64) << 8;
                val |= data.get_u8() as u64;
                Ok(val)
            }
            3 => {
                if data.remaining() < 7 {
                    return Err(super::Http3Error::InvalidFrame("Incomplete varint".to_string()));
                }
                let mut val = ((first & 0x3F) as u64) << 56;
                for _ in 0..7 {
                    val = (val << 8) | data.get_u8() as u64;
                }
                Ok(val)
            }
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_encode_decode() {
        let original = Frame::data(Bytes::from("Hello HTTP/3"));
        let encoded = original.encode();
        let decoded = Frame::decode(encoded).unwrap();

        assert_eq!(decoded.frame_type, FrameType::Data);
        assert_eq!(decoded.payload, Bytes::from("Hello HTTP/3"));
    }

    #[test]
    fn test_varint_encoding() {
        let mut buf = BytesMut::new();
        Frame::encode_varint(&mut buf, 42);
        assert_eq!(buf.len(), 1);

        let mut buf = BytesMut::new();
        Frame::encode_varint(&mut buf, 1000);
        assert!(buf.len() >= 2);
    }
}
