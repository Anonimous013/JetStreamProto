/// Variable-length integer encoding (varint)
/// Used to efficiently encode integers with variable byte length

/// Encode u64 as varint
/// - 0-127: 1 byte
/// - 128-16383: 2 bytes
/// - etc.
pub fn encode_varint(mut value: u64) -> Vec<u8> {
    let mut bytes = Vec::new();
    
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        
        if value != 0 {
            byte |= 0x80; // Set continuation bit
        }
        
        bytes.push(byte);
        
        if value == 0 {
            break;
        }
    }
    
    bytes
}

/// Decode varint from bytes
/// Returns (value, bytes_consumed)
pub fn decode_varint(bytes: &[u8]) -> Result<(u64, usize), &'static str> {
    let mut value: u64 = 0;
    let mut shift = 0;
    let mut pos = 0;
    
    loop {
        if pos >= bytes.len() {
            return Err("Incomplete varint");
        }
        
        let byte = bytes[pos];
        pos += 1;
        
        value |= ((byte & 0x7F) as u64) << shift;
        
        if byte & 0x80 == 0 {
            // No continuation bit, we're done
            break;
        }
        
        shift += 7;
        
        if shift >= 64 {
            return Err("Varint too large");
        }
    }
    
    Ok((value, pos))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_varint_small() {
        let value = 42u64;
        let encoded = encode_varint(value);
        assert_eq!(encoded.len(), 1);
        assert_eq!(encoded[0], 42);
        
        let (decoded, consumed) = decode_varint(&encoded).unwrap();
        assert_eq!(decoded, value);
        assert_eq!(consumed, 1);
    }

    #[test]
    fn test_varint_medium() {
        let value = 300u64;
        let encoded = encode_varint(value);
        assert_eq!(encoded.len(), 2);
        
        let (decoded, consumed) = decode_varint(&encoded).unwrap();
        assert_eq!(decoded, value);
        assert_eq!(consumed, 2);
    }

    #[test]
    fn test_varint_large() {
        let value = 1_000_000u64;
        let encoded = encode_varint(value);
        
        let (decoded, consumed) = decode_varint(&encoded).unwrap();
        assert_eq!(decoded, value);
        assert_eq!(consumed, encoded.len());
    }

    #[test]
    fn test_varint_max() {
        let value = u64::MAX;
        let encoded = encode_varint(value);
        
        let (decoded, consumed) = decode_varint(&encoded).unwrap();
        assert_eq!(decoded, value);
        assert_eq!(consumed, encoded.len());
    }

    #[test]
    fn test_varint_zero() {
        let value = 0u64;
        let encoded = encode_varint(value);
        assert_eq!(encoded.len(), 1);
        assert_eq!(encoded[0], 0);
        
        let (decoded, consumed) = decode_varint(&encoded).unwrap();
        assert_eq!(decoded, value);
        assert_eq!(consumed, 1);
    }
}
