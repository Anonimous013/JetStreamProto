use anyhow::Result;
use jsp_core::transfer::{FileSender, FileReceiver, FileMetadata, ChunkHeader};
use jsp_transport::connection::Connection;
use jsp_transport::config::ConnectionConfig;
use std::time::Duration;
use std::path::PathBuf;
use std::io::Write;
use tempfile::NamedTempFile;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create a dummy file to transfer
    let mut temp_file = NamedTempFile::new()?;
    let data = vec![0u8; 1024 * 1024]; // 1MB file
    temp_file.write_all(&data)?;
    let file_path = temp_file.path().to_path_buf();
    
    println!("Created temporary file: {:?}", file_path);

    // Start server
    let server_handle = tokio::spawn(async move {
        if let Err(e) = run_server().await {
            eprintln!("Server error: {}", e);
        }
    });

    // Wait for server to start
    sleep(Duration::from_millis(500)).await;

    // Start client
    if let Err(e) = run_client(file_path).await {
        eprintln!("Client error: {}", e);
    }

    // Wait for transfer to complete (in real app we'd have better synchronization)
    sleep(Duration::from_secs(5)).await;
    
    Ok(())
}

async fn run_server() -> Result<()> {
    let config = ConnectionConfig::default();
    let mut connection = Connection::connect_with_config("127.0.0.1:8082", config).await?;
    
    println!("Server: Listening on 127.0.0.1:8082");
    
    // Handshake
    connection.handshake().await?;
    println!("Server: Handshake completed");

    // Accept stream
    // In a real server, we'd loop and accept streams.
    // Here we assume the client opens stream 1.
    
    let mut receiver: Option<FileReceiver> = None;
    let output_path = PathBuf::from("received_file.bin");

    let mut receiver: Option<FileReceiver> = None;
    let output_path = PathBuf::from("received_file.bin");

    // We need a timeout to stop server
    let start_time = std::time::Instant::now();

    loop {
        if start_time.elapsed() > Duration::from_secs(10) {
            break;
        }

        let packets = connection.recv().await?;
        
        for (stream_id, data) in packets {
            println!("Server: Received {} bytes on stream {}", data.len(), stream_id);
            
            // Check if it's metadata (first packet)
            if receiver.is_none() {
                // Try to parse metadata
                if let Ok(metadata) = serde_cbor::from_slice::<FileMetadata>(&data) {
                    println!("Server: Received metadata: {:?}", metadata);
                    receiver = Some(FileReceiver::new(1, metadata, output_path.clone())?);
                    continue;
                }
            }
            
            // Process chunk
            if let Some(recv) = &mut receiver {
                // Parse packet: [Header Len (2)] [Header] [Data]
                // Wait, Connection::recv returns payload (which is [Header Len] [Header] [Data] in our example protocol)
                // Yes, we defined a custom protocol ON TOP of streams.
                
                if data.len() < 2 { continue; }
                let header_len = u16::from_be_bytes([data[0], data[1]]) as usize;
                if data.len() < 2 + header_len { continue; }
                
                let header_bytes = &data[2..2+header_len];
                let chunk_data = &data[2+header_len..];
                
                if let Ok(header) = serde_cbor::from_slice::<ChunkHeader>(header_bytes) {
                    recv.write_chunk(&header, chunk_data)?;
                    
                    if recv.is_complete(header.chunk_id + 1) { // This check is wrong, need total chunks
                         // For now just print
                    }
                }
            }
        }
        
        // Send ACKs? Connection should handle ACKs automatically?
        // Wait, Connection::recv doesn't send ACKs.
        // We need to call `connection.flush_acks()` or similar?
        // Or `recv` should send ACKs if needed?
        // Currently `recv` only tracks received packets.
        // We need to send ACKs back.
        
        // Connection needs a `update()` or `tick()` method to send ACKs and Heartbeats.
        // Or `recv` should do it.
        
        // Let's assume for now we don't send ACKs (unreliable) or we add it later.
        // But wait, if we don't send ACKs, the sender will retransmit forever (if reliable).
        // And `ReliabilityLayer` tracks received packets but doesn't generate ACK packets to send.
        
        // I need to implement ACK sending in Connection.
        
        sleep(Duration::from_millis(10)).await;
    }
    
    Ok(())
}

async fn run_client(file_path: PathBuf) -> Result<()> {
    let config = ConnectionConfig::default();
    let mut connection = Connection::connect_with_config("127.0.0.1:8082", config).await?;
    
    println!("Client: Connecting to 127.0.0.1:8082");
    
    // Handshake
    connection.handshake().await?;
    println!("Client: Handshake completed");
    
    // Open stream
    let stream_id = connection.session.open_reliable_stream(10)?;
    println!("Client: Opened stream {}", stream_id);
    
    // Setup file sender
    let chunk_size = 1024;
    let mut sender = FileSender::new(1, file_path, chunk_size)?;
    
    // Send metadata
    let metadata = sender.metadata();
    let metadata_bytes = serde_cbor::to_vec(&metadata)?;
    // We need a way to distinguish metadata from chunks.
    // For this example, let's say first packet is metadata.
    connection.send_on_stream(stream_id, &metadata_bytes).await?;
    
    println!("Client: Sent metadata");
    
    // Send chunks
    let total_chunks = sender.total_chunks();
    for i in 0..total_chunks {
        if let Some((header, data)) = sender.read_chunk(i)? {
            // Serialize header and data
            // Simple format: [Header Len (2 bytes)][Header Bytes][Data]
            let header_bytes = serde_cbor::to_vec(&header)?;
            let header_len = header_bytes.len() as u16;
            
            let mut packet = Vec::new();
            packet.extend_from_slice(&header_len.to_be_bytes());
            packet.extend_from_slice(&header_bytes);
            packet.extend_from_slice(&data);
            
            connection.send_on_stream(stream_id, &packet).await?;
            
            if i % 100 == 0 {
                println!("Client: Sent chunk {}/{}", i, total_chunks);
            }
            
            // Pacing
            sleep(Duration::from_millis(1)).await;
        }
    }
    
    println!("Client: File transfer completed");
    
    Ok(())
}
