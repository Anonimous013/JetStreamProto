use jsp_transport::connection::Connection;
use jsp_transport::config::ConnectionConfig;
use jsp_core::types::control::CloseReason;
use std::io::{self, Write};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("💬 JetStreamProto Chat Client");
    println!("=" .repeat(50));
    
    // Get username
    print!("\n👤 Enter your username: ");
    io::stdout().flush()?;
    let mut username = String::new();
    io::stdin().read_line(&mut username)?;
    let username = username.trim().to_string();
    
    // Get server address
    print!("🌐 Server address (default: 127.0.0.1:8080): ");
    io::stdout().flush()?;
    let mut server_addr = String::new();
    io::stdin().read_line(&mut server_addr)?;
    let server_addr = server_addr.trim();
    let server_addr = if server_addr.is_empty() {
        "127.0.0.1:8080"
    } else {
        server_addr
    };
    
    // Connect to server
    println!("\n🔌 Connecting to {}...", server_addr);
    let config = ConnectionConfig {
        heartbeat_interval: Duration::from_secs(10),
        enable_compression: true,
        ..Default::default()
    };
    
    let mut conn = Connection::connect_with_config(server_addr, config).await?;
    println!("✅ Connected!");
    
    // Send JOIN message
    let join_msg = format!("JOIN:{}", username);
    conn.send_on_stream(1, join_msg.as_bytes()).await?;
    
    println!("\n💬 Chat started! Type your messages (or 'quit' to exit)\n");
    
    // Spawn receiver task
    let conn_recv = Arc::new(tokio::sync::Mutex::new(conn));
    let conn_send = conn_recv.clone();
    
    let username_clone = username.clone();
    let recv_task = tokio::spawn(async move {
        loop {
            let mut conn = conn_recv.lock().await;
            match conn.recv().await {
                Ok(packets) => {
                    for (_, data) in packets {
                        let msg = String::from_utf8_lossy(&data);
                        
                        // Parse message
                        let parts: Vec<&str> = msg.splitn(2, ':').collect();
                        if parts.len() == 2 {
                            match parts[0] {
                                "MSG" => {
                                    println!("💬 {}", parts[1]);
                                }
                                "SYSTEM" => {
                                    println!("📢 {}", parts[1]);
                                }
                                _ => {}
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("\n❌ Connection error: {}", e);
                    break;
                }
            }
        }
    });
    
    // Input loop
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();
    
    loop {
        line.clear();
        
        // Read line from stdin
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                let input = line.trim();
                
                if input.is_empty() {
                    continue;
                }
                
                if input == "quit" || input == "exit" {
                    println!("\n👋 Leaving chat...");
                    
                    // Send LEAVE message
                    let leave_msg = format!("LEAVE:{}", username);
                    let mut conn = conn_send.lock().await;
                    let _ = conn.send_on_stream(1, leave_msg.as_bytes()).await;
                    let _ = conn.close(CloseReason::Normal, Some("User quit".to_string())).await;
                    break;
                }
                
                // Send message
                let chat_msg = format!("MSG:{}: {}", username, input);
                let mut conn = conn_send.lock().await;
                if let Err(e) = conn.send_on_stream(1, chat_msg.as_bytes()).await {
                    eprintln!("❌ Failed to send: {}", e);
                    break;
                }
            }
            Err(e) => {
                eprintln!("❌ Input error: {}", e);
                break;
            }
        }
    }
    
    // Wait for receiver task
    recv_task.abort();
    
    println!("✅ Disconnected");
    Ok(())
}
