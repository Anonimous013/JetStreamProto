use jsp_core::compression::header_compression::HeaderCompressor;
use jsp_core::types::header::{Header, FRAME_TYPE_DATA};
use jsp_core::types::delivery::DeliveryMode;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let mut compressor = HeaderCompressor::new();
    let mut total_original_size = 0;
    let mut total_compressed_size = 0;
    let start_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;

    println!("Generating 100 headers...");

    for i in 0..100 {
        let header = Header::new(
            1,
            FRAME_TYPE_DATA,
            0,
            i as u64,
            start_time + i as u64 * 10,
            0,
            DeliveryMode::Reliable,
            None,
            Some(100),
        );
        
        let original = serde_cbor::to_vec(&header).unwrap();
        let compressed = compressor.compress(&header);
        
        total_original_size += original.len();
        total_compressed_size += compressed.len();
    }
    
    println!("Total Original Size: {} bytes", total_original_size);
    println!("Total Compressed Size: {} bytes", total_compressed_size);
    println!("Compression Ratio: {:.2}", total_original_size as f64 / total_compressed_size as f64);
    println!("Space Saving: {:.2}%", 100.0 * (1.0 - total_compressed_size as f64 / total_original_size as f64));
}
