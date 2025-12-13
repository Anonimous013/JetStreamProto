mod crd;
mod controller;

use kube::Client;
use tracing::Level;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    tracing::info!("Starting JetStreamProto Kubernetes Operator");

    let client = Client::try_default().await?;
    
    controller::run(client).await;

    Ok(())
}
