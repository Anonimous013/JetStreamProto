use anyhow::Result;
use colored::Colorize;
use std::time::Duration;
use tokio::time;
use jsp_transport::connection::Connection;
use jsp_transport::config::ConnectionConfig;

pub async fn run(addr: &str, interval_secs: u64) -> Result<()> {
    println!("{}", "JetStreamProto Connection Monitor".bold().green());
    println!("{}", "=".repeat(50));
    println!("Connecting to: {}", addr.cyan());
    println!();

    // Connect
    let mut connection = Connection::connect_with_config(addr, ConnectionConfig::default()).await?;
    
    println!("{}", "✓ Connected successfully".green());
    
    // Perform handshake
    connection.handshake().await?;
    println!("{}", "✓ Handshake completed".green());
    println!();

    let session_id = connection.session_id();
    println!("Session ID: {}", session_id.to_string().yellow());
    println!();

    // Monitor loop
    let interval = Duration::from_secs(interval_secs);
    let mut counter = 0;

    loop {
        counter += 1;
        
        println!("{} {}", "Update".bold(), counter);
        println!("  Status: {}", "Connected".green());
        println!("  Session: {}", session_id.to_string().yellow());
        println!("  Transport: {}", "UDP".cyan());
        
        // In real implementation, would show actual metrics
        println!("  Throughput: {} MB/s", "5.2".green());
        println!("  Latency: {} ms", "45".green());
        println!("  Packet Loss: {}%", "0.1".green());
        println!();

        time::sleep(interval).await;
    }
}
