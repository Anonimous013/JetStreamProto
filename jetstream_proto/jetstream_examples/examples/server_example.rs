use jsp_transport::server::Server;
use jsp_transport::config::{ServerConfig, ConnectionConfig};
use anyhow::Result;
use std::time::Duration;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize structured logging with JSON format for production
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_target(true)
        .json()
        .init();

    tracing::info!("🚀 JetStreamProto Advanced Server Example");

    // Configure server with custom settings
    let connection_config = ConnectionConfig::builder()
        .session_timeout(Duration::from_secs(30))
        .heartbeat_interval(Duration::from_secs(5))
        .max_streams(100)
        .rate_limit_messages(100)
        .rate_limit_bytes(1_048_576) // 1 MB/s per connection
        .build();

    let server_config = ServerConfig::builder()
        .connection(connection_config)
        .global_rate_limit_messages(Some(10_000))
        .global_rate_limit_bytes(Some(100_000_000)) // 100 MB/s globally
        .cleanup_interval(Duration::from_secs(10))
        .build();

    // Start server
    let mut server = Server::bind_with_config("127.0.0.1:8080", server_config).await?;
    tracing::info!("✅ Server listening on 127.0.0.1:8080");

    // Spawn graceful shutdown handler
    let shutdown_signal = tokio::spawn(async {
        signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        tracing::info!("🛑 Shutdown signal received");
    });

    // Main server loop
    let server_task = tokio::spawn(async move {
        loop {
            match server.accept().await {
                Ok((addr, session)) => {
                    tracing::info!(
                        peer = %addr,
                        session_id = session.session_id,
                        "New client connected"
                    );

                    // Spawn handler for this client
                    tokio::spawn(async move {
                        handle_client(addr, session).await;
                    });
                }
                Err(e) => {
                    if e.to_string().contains("rate limit") {
                        tracing::warn!("Rate limit exceeded: {}", e);
                    } else {
                        tracing::error!("Error accepting connection: {}", e);
                    }
                }
            }
        }
    });

    // Wait for shutdown signal
    shutdown_signal.await?;

    // Graceful shutdown
    tracing::info!("Shutting down server...");
    server_task.abort();
    
    tracing::info!("✨ Server shutdown complete");

    Ok(())
}

async fn handle_client(addr: std::net::SocketAddr, session: jsp_core::session::Session) {
    tracing::info!(
        peer = %addr,
        session_id = session.session_id,
        "Handling client"
    );

    // In a real application, you would:
    // 1. Process incoming messages
    // 2. Handle stream multiplexing
    // 3. Respond to client requests
    // 4. Monitor session timeout
    // 5. Handle graceful disconnection

    // For this example, just log the session info
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    tracing::info!(
        peer = %addr,
        session_age = ?session.age(),
        idle_time = ?session.idle_duration(),
        "Client session info"
    );
}
