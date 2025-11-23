use jsp_transport::connection::Connection;
use jsp_transport::config::ConnectionConfig;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::Duration;

type ClientId = u64;
type Clients = Arc<RwLock<HashMap<ClientId, Arc<tokio::sync::Mutex<Connection>>>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("🚀 JetStreamProto Chat Server");
    println!("{}", "=".repeat(50));
    
    // Server configuration
    let config = ConnectionConfig {
        heartbeat_interval: Duration::from_secs(10),
        heartbeat_timeout_count: 3,
        ..Default::default()
    };
    
    let bind_addr = "0.0.0.0:8080";
    println!("\n📡 Starting server on {}", bind_addr);
    
    // Create server connection
    let mut server = Connection::listen_with_config(bind_addr, config).await?;
    println!("✅ Server listening");
    
    // Client storage
    let clients: Clients = Arc::new(RwLock::new(HashMap::new()));
    
    // Metrics reporting task
    let metrics_clients = clients.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            let count = metrics_clients.read().await.len();
            println!("\n📊 Active clients: {}", count);
        }
    });
    
    println!("\n💬 Waiting for messages...\n");
    
    // Main message loop
    loop {
        match server.recv().await {
            Ok(packets) => {
                for (stream_id, data) in packets {
                    handle_message(stream_id, data.to_vec(), &clients).await;
                }
            }
            Err(e) => {
                eprintln!("❌ Receive error: {}", e);
            }
        }
    }
}

async fn handle_message(stream_id: u32, data: Vec<u8>, clients: &Clients) {
    // Parse message
    let msg = String::from_utf8_lossy(&data);
    
    // Simple protocol: "COMMAND:DATA"
    let parts: Vec<&str> = msg.splitn(2, ':').collect();
    if parts.len() < 2 {
        println!("⚠️  Invalid message format");
        return;
    }
    
    let command = parts[0];
    let payload = parts[1];
    
    match command {
        "JOIN" => {
            let username = payload;
            println!("👤 {} joined (stream {})", username, stream_id);
            
            // Broadcast join message
            let broadcast_msg = format!("SYSTEM:{} joined the chat", username);
            broadcast(clients, &broadcast_msg, None).await;
        }
        
        "MSG" => {
            println!("💬 Stream {}: {}", stream_id, payload);
            
            // Broadcast message to all clients
            let broadcast_msg = format!("MSG:{}", payload);
            broadcast(clients, &broadcast_msg, Some(stream_id)).await;
        }
        
        "LEAVE" => {
            let username = payload;
            println!("👋 {} left (stream {})", username, stream_id);
            
            // Broadcast leave message
            let broadcast_msg = format!("SYSTEM:{} left the chat", username);
            broadcast(clients, &broadcast_msg, Some(stream_id)).await;
        }
        
        _ => {
            println!("⚠️  Unknown command: {}", command);
        }
    }
}

async fn broadcast(clients: &Clients, message: &str, exclude_stream: Option<u32>) {
    let clients_lock = clients.read().await;
    
    for (client_id, client) in clients_lock.iter() {
        // Skip excluded stream
        if let Some(exclude) = exclude_stream {
            if *client_id as u32 == exclude {
                continue;
            }
        }
        
        // Send message
        let mut conn = client.lock().await;
        if let Err(e) = conn.send_on_stream(1, message.as_bytes()).await {
            eprintln!("❌ Failed to send to client {}: {}", client_id, e);
        }
    }
}
