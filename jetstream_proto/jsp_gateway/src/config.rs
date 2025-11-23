use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GatewayConfig {
    pub bind_addr: String,
    pub backends: Vec<String>,
    #[serde(default)]
    pub strategy: String,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:5000".to_string(),
            backends: vec!["127.0.0.1:8080".to_string()],
            strategy: "round-robin".to_string(),
        }
    }
}
