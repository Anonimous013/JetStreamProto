use anyhow::Result;
use colored::Colorize;
use jsp_transport::connection::Connection;
use jsp_transport::config::ConnectionConfig;

pub async fn run(addr: &str, message: &str, count: usize) -> Result<()> {
    println!("{}", "JetStreamProto Send Test".bold().green());
    println!("{}", "=".repeat(50));
    println!("Target: {}", addr.cyan());
    println!("Message: {}", message.yellow());
    println!("Count: {}", count.to_string().yellow());
    println!();

    // Connect
    println!("Connecting...");
    let mut connection = Connection::connect_with_config(addr, ConnectionConfig::default()).await?;
    connection.handshake().await?;
    println!("{}", "✓ Connected".green());
    println!();

    // Open stream
    let stream_id = connection.open_stream(1, jsp_core::types::delivery::DeliveryMode::Reliable)?;
    println!("Stream ID: {}", stream_id.to_string().yellow());
    println!();

    // Send messages
    println!("Sending messages...");
    for i in 0..count {
        let msg = format!("{} #{}", message, i + 1);
        connection.send_on_stream(stream_id, msg.as_bytes()).await?;
        println!("  {} Sent: {}", "✓".green(), msg);
        
        // Small delay between messages
        if count > 1 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    println!();
    println!("{} Sent {} messages successfully", "✓".green(), count.to_string().yellow());

    Ok(())
}
