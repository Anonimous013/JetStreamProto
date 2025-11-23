use clap::Parser;
use jsp_gateway::proxy::Proxy;
use jsp_gateway::balancer::{LoadBalancer, Strategy};
use std::sync::Arc;
use std::net::SocketAddr;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Bind address
    #[arg(short, long, default_value = "0.0.0.0:5000")]
    bind: String,

    /// Backend servers (comma separated)
    #[arg(short, long, value_delimiter = ',', default_value = "127.0.0.1:8080")]
    backends: Vec<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    
    tracing::info!("Starting JetStreamProto Gateway...");
    tracing::info!("Bind address: {}", args.bind);
    tracing::info!("Backends: {:?}", args.backends);

    // Parse backend addresses
    let backends: Vec<SocketAddr> = args.backends.iter()
        .map(|s| s.parse().expect("Invalid backend address"))
        .collect();

    // Initialize Load Balancer
    let balancer = Arc::new(LoadBalancer::new(backends, Strategy::RoundRobin));

    // Initialize Proxy
    let proxy = Proxy::new(&args.bind, balancer).await?;

    // Run Proxy
    proxy.run().await?;

    Ok(())
}
