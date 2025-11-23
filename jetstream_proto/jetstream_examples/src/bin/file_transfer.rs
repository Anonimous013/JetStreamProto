use jsp_transport::connection::Connection;
use jsp_transport::config::ConnectionConfig;
use jsp_core::types::control::CloseReason;
use std::time::{Duration, Instant};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const CHUNK_SIZE: usize = 64 * 1024; // 64KB chunks

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("📁 JetStreamProto File Transfer Demo");
    println!("{}", "=".repeat(50));
    
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 3 {
        println!("\nUsage:");
        println!("  Send:    {} send <file_path> [server_addr]", args[0]);
        println!("  Receive: {} receive <output_path> [bind_addr]", args[0]);
        return Ok(());
    }
    
    let mode = &args[1];
    
    match mode.as_str() {
        "send" => {
            let file_path = &args[2];
            let server_addr = args.get(3).map(|s| s.as_str()).unwrap_or("127.0.0.1:8080");
            send_file(file_path, server_addr).await?;
        }
        "receive" => {
            let output_path = &args[2];
            let bind_addr = args.get(3).map(|s| s.as_str()).unwrap_or("0.0.0.0:8080");
            receive_file(output_path, bind_addr).await?;
        }
        _ => {
            println!("❌ Invalid mode. Use 'send' or 'receive'");
        }
    }
    
    Ok(())
}

async fn send_file(file_path: &str, server_addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📤 Sending file: {}", file_path);
    println!("🌐 Server: {}", server_addr);
    
    // Open file
    let mut file = File::open(file_path).await?;
    let metadata = file.metadata().await?;
    let file_size = metadata.len();
    
    println!("📊 File size: {} bytes ({:.2} MB)", file_size, file_size as f64 / 1_048_576.0);
    
    // Connect to server
    let config = ConnectionConfig {
        heartbeat_interval: Duration::from_secs(15),
        ..Default::default()
    };
    
    println!("\n🔌 Connecting...");
    let mut conn = Connection::connect_with_config(server_addr, config).await?;
    println!("✅ Connected!");
    
    // Send file metadata
    let filename = Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    
    let metadata_msg = format!("FILE:{}:{}", filename, file_size);
    conn.send_on_stream(1, metadata_msg.as_bytes()).await?;
    
    println!("\n📦 Transferring...");
    let start = Instant::now();
    let mut total_sent = 0u64;
    let mut chunk_buffer = vec![0u8; CHUNK_SIZE];
    
    loop {
        let n = file.read(&mut chunk_buffer).await?;
        if n == 0 {
            break; // EOF
        }
        
        // Send chunk
        conn.send_on_stream(2, &chunk_buffer[..n]).await?;
        total_sent += n as u64;
        
        // Progress
        let progress = (total_sent as f64 / file_size as f64) * 100.0;
        print!("\r📊 Progress: {:.1}% ({} / {} bytes)", progress, total_sent, file_size);
        std::io::Write::flush(&mut std::io::stdout())?;
    }
    
    println!();
    
    // Send completion marker
    conn.send_on_stream(3, b"DONE").await?;
    
    let elapsed = start.elapsed();
    let throughput = (total_sent as f64 / elapsed.as_secs_f64()) / 1_048_576.0; // MB/s
    
    println!("\n✅ Transfer complete!");
    println!("⏱️  Time: {:.2}s", elapsed.as_secs_f64());
    println!("🚀 Throughput: {:.2} MB/s", throughput);
    
    // Close connection
    conn.close(CloseReason::Normal, Some("Transfer complete".to_string())).await?;
    
    Ok(())
}

async fn receive_file(output_path: &str, bind_addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📥 Waiting for file...");
    println!("🌐 Listening on: {}", bind_addr);
    
    // Start server
    let config = ConnectionConfig {
        heartbeat_interval: Duration::from_secs(15),
        ..Default::default()
    };
    
    let mut server = Connection::listen_with_config(bind_addr, config).await?;
    println!("✅ Server ready");
    
    let mut file: Option<File> = None;
    let mut file_size = 0u64;
    let mut total_received = 0u64;
    let start = Instant::now();
    
    println!("\n⏳ Waiting for connection...\n");
    
    loop {
        let packets = server.recv().await?;
        
        for (stream_id, data) in packets {
            match stream_id {
                1 => {
                    // Metadata
                    let msg = String::from_utf8_lossy(&data);
                    if msg.starts_with("FILE:") {
                        let parts: Vec<&str> = msg.splitn(3, ':').collect();
                        if parts.len() == 3 {
                            let filename = parts[1];
                            file_size = parts[2].parse().unwrap_or(0);
                            
                            println!("📁 Receiving: {}", filename);
                            println!("📊 Size: {} bytes ({:.2} MB)", file_size, file_size as f64 / 1_048_576.0);
                            println!();
                            
                            // Create output file
                            file = Some(File::create(output_path).await?);
                        }
                    }
                }
                2 => {
                    // Data chunk
                    if let Some(ref mut f) = file {
                        f.write_all(&data).await?;
                        total_received += data.len() as u64;
                        
                        // Progress
                        if file_size > 0 {
                            let progress = (total_received as f64 / file_size as f64) * 100.0;
                            print!("\r📊 Progress: {:.1}% ({} / {} bytes)", progress, total_received, file_size);
                            std::io::Write::flush(&mut std::io::stdout())?;
                        }
                    }
                }
                3 => {
                    // Completion
                    if String::from_utf8_lossy(&data) == "DONE" {
                        println!("\n");
                        
                        // Flush and close file
                        if let Some(mut f) = file.take() {
                            f.flush().await?;
                        }
                        
                        let elapsed = start.elapsed();
                        let throughput = (total_received as f64 / elapsed.as_secs_f64()) / 1_048_576.0;
                        
                        println!("✅ Transfer complete!");
                        println!("📁 Saved to: {}", output_path);
                        println!("⏱️  Time: {:.2}s", elapsed.as_secs_f64());
                        println!("🚀 Throughput: {:.2} MB/s", throughput);
                        
                        return Ok(());
                    }
                }
                _ => {}
            }
        }
    }
}
