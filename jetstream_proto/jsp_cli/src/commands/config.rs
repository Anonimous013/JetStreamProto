use anyhow::Result;
use colored::Colorize;
use serde::{Serialize, Deserialize};



#[derive(Debug, Serialize, Deserialize)]
struct Config {
    server_address: String,
    session_timeout_secs: u64,
    heartbeat_interval_secs: u64,
    max_streams: usize,
    rate_limit_messages: usize,
    enable_compression: bool,
    enable_encryption: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_address: "127.0.0.1:8080".to_string(),
            session_timeout_secs: 300,
            heartbeat_interval_secs: 30,
            max_streams: 100,
            rate_limit_messages: 1000,
            enable_compression: true,
            enable_encryption: true,
        }
    }
}

pub fn generate(output: &str) -> Result<()> {
    println!("{}", "Generating default configuration...".bold());
    
    let config = Config::default();
    let json = serde_json::to_string_pretty(&config)?;
    
    std::fs::write(output, json)?;
    
    println!("{} {}", "✓ Configuration saved to:".green(), output.cyan());
    Ok(())
}

pub fn validate(file: &str) -> Result<()> {
    println!("{} {}", "Validating configuration:".bold(), file.cyan());
    
    let content = std::fs::read_to_string(file)?;
    let config: Config = serde_json::from_str(&content)?;
    
    // Validate fields
    let mut valid = true;
    
    if config.session_timeout_secs == 0 {
        println!("{} Session timeout must be > 0", "✗".red());
        valid = false;
    }
    
    if config.heartbeat_interval_secs == 0 {
        println!("{} Heartbeat interval must be > 0", "✗".red());
        valid = false;
    }
    
    if config.max_streams == 0 {
        println!("{} Max streams must be > 0", "✗".red());
        valid = false;
    }
    
    if valid {
        println!("{} Configuration is valid", "✓".green());
    } else {
        anyhow::bail!("Configuration validation failed");
    }
    
    Ok(())
}

pub fn show() -> Result<()> {
    println!("{}", "Current Configuration".bold().green());
    println!("{}", "=".repeat(50));
    
    let config = Config::default();
    
    println!("Server Address: {}", config.server_address.cyan());
    println!("Session Timeout: {} seconds", config.session_timeout_secs.to_string().yellow());
    println!("Heartbeat Interval: {} seconds", config.heartbeat_interval_secs.to_string().yellow());
    println!("Max Streams: {}", config.max_streams.to_string().yellow());
    println!("Rate Limit: {} messages/sec", config.rate_limit_messages.to_string().yellow());
    println!("Compression: {}", if config.enable_compression { "Enabled".green() } else { "Disabled".red() });
    println!("Encryption: {}", if config.enable_encryption { "Enabled".green() } else { "Disabled".red() });
    
    Ok(())
}
