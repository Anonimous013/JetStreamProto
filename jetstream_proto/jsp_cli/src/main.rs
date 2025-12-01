use clap::{Parser, Subcommand};
use anyhow::Result;

mod commands;

#[derive(Parser)]
#[command(name = "jsp-cli")]
#[command(about = "JetStreamProto CLI Tools", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Monitor connection status and metrics
    Monitor {
        /// Server address
        #[arg(short, long, default_value = "127.0.0.1:8080")]
        addr: String,
        
        /// Update interval in seconds
        #[arg(short, long, default_value = "1")]
        interval: u64,
    },
    
    /// Profile connection performance
    Profile {
        /// Server address
        #[arg(short, long, default_value = "127.0.0.1:8080")]
        addr: String,
        
        /// Duration in seconds
        #[arg(short, long, default_value = "60")]
        duration: u64,
        
        /// Output file (JSON)
        #[arg(short, long)]
        output: Option<String>,
    },
    
    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    
    /// Send test data
    Send {
        /// Server address
        #[arg(short, long, default_value = "127.0.0.1:8080")]
        addr: String,
        
        /// Message to send
        #[arg(short, long, default_value = "Hello, JetStream!")]
        message: String,
        
        /// Number of messages
        #[arg(short, long, default_value = "1")]
        count: usize,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Generate default configuration
    Generate {
        /// Output file
        #[arg(short, long, default_value = "config.json")]
        output: String,
    },
    
    /// Validate configuration file
    Validate {
        /// Config file to validate
        #[arg(short, long)]
        file: String,
    },
    
    /// Show current configuration
    Show,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Monitor { addr, interval } => {
            commands::monitor::run(&addr, interval).await?;
        }
        Commands::Profile { addr, duration, output } => {
            commands::profile::run(&addr, duration, output.as_deref()).await?;
        }
        Commands::Config { action } => {
            match action {
                ConfigAction::Generate { output } => {
                    commands::config::generate(&output)?;
                }
                ConfigAction::Validate { file } => {
                    commands::config::validate(&file)?;
                }
                ConfigAction::Show => {
                    commands::config::show()?;
                }
            }
        }
        Commands::Send { addr, message, count } => {
            commands::send::run(&addr, &message, count).await?;
        }
    }

    Ok(())
}
