use std::time::Duration;
use crate::ddos_protection::DdosConfig;

/// Connection configuration
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// Session timeout duration
    pub session_timeout: Duration,
    /// Heartbeat interval
    pub heartbeat_interval: Duration,
    /// Number of missed heartbeats before timeout
    pub heartbeat_timeout_count: u32,
    /// Maximum concurrent streams per connection
    pub max_streams: u32,
    /// Per-connection message rate limit (messages/second)
    pub rate_limit_messages: u32,
    /// Per-connection byte rate limit (bytes/second)
    pub rate_limit_bytes: u64,
    /// Optional local address to bind to (e.g. "0.0.0.0:0" or "127.0.0.1:8081")
    pub bind_addr: Option<String>,
    /// Memory pool capacity (number of buffers to keep)
    pub pool_capacity: usize,
    /// Maximum packet size for pooled buffers
    pub pool_max_packet_size: usize,
    /// Maximum number of ACKs to batch before sending
    pub ack_batch_size: usize,
    /// Maximum time to wait before sending batched ACKs (in milliseconds)
    pub ack_batch_timeout_ms: u64,
    /// Maximum time to wait for message coalescing (in milliseconds, 0 = disabled)
    pub coalescing_window_ms: u64,
    /// STUN servers for NAT discovery (e.g., ["stun.l.google.com:19302"])
    pub stun_servers: Vec<String>,
    /// STUN request timeout
    pub stun_timeout: Duration,
    /// STUN cache TTL (how long to cache discovered address)
    pub stun_cache_ttl: Duration,
    /// Enable header compression (default: true)
    pub enable_header_compression: bool,
    /// Multi-hop tunnel configuration (optional)
    pub multihop_config: Option<crate::multihop::MultiHopConfig>,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            session_timeout: Duration::from_secs(30),
            heartbeat_interval: Duration::from_secs(5),
            heartbeat_timeout_count: 3,
            max_streams: 100,
            rate_limit_messages: 100,
            rate_limit_bytes: 1_048_576, // 1 MB/s
            bind_addr: None,
            pool_capacity: 100, // Keep up to 100 buffers
            pool_max_packet_size: 65536, // 64 KB max packet size
            ack_batch_size: 10, // Batch up to 10 ACKs
            ack_batch_timeout_ms: 10, // Wait max 10ms
            coalescing_window_ms: 0, // Disabled by default
            stun_servers: vec![], // No STUN servers by default
            stun_timeout: Duration::from_secs(5),
            stun_cache_ttl: Duration::from_secs(300), // 5 minutes
            enable_header_compression: true,
            multihop_config: None, // Multi-hop disabled by default
        }
    }
}

impl ConnectionConfig {
    pub fn builder() -> ConnectionConfigBuilder {
        ConnectionConfigBuilder::default()
    }
}

/// Builder for ConnectionConfig
#[derive(Debug, Default)]
pub struct ConnectionConfigBuilder {
    session_timeout: Option<Duration>,
    heartbeat_interval: Option<Duration>,
    heartbeat_timeout_count: Option<u32>,
    max_streams: Option<u32>,
    rate_limit_messages: Option<u32>,
    rate_limit_bytes: Option<u64>,
    bind_addr: Option<String>,
    pool_capacity: Option<usize>,
    pool_max_packet_size: Option<usize>,
    ack_batch_size: Option<usize>,
    ack_batch_timeout_ms: Option<u64>,
    coalescing_window_ms: Option<u64>,
    stun_servers: Option<Vec<String>>,
    stun_timeout: Option<Duration>,
    stun_cache_ttl: Option<Duration>,
    enable_header_compression: Option<bool>,
    multihop_config: Option<Option<crate::multihop::MultiHopConfig>>,
}

impl ConnectionConfigBuilder {
    pub fn session_timeout(mut self, timeout: Duration) -> Self {
        self.session_timeout = Some(timeout);
        self
    }

    pub fn heartbeat_interval(mut self, interval: Duration) -> Self {
        self.heartbeat_interval = Some(interval);
        self
    }

    pub fn heartbeat_timeout_count(mut self, count: u32) -> Self {
        self.heartbeat_timeout_count = Some(count);
        self
    }

    pub fn max_streams(mut self, max: u32) -> Self {
        self.max_streams = Some(max);
        self
    }

    pub fn rate_limit_messages(mut self, limit: u32) -> Self {
        self.rate_limit_messages = Some(limit);
        self
    }

    pub fn rate_limit_bytes(mut self, limit: u64) -> Self {
        self.rate_limit_bytes = Some(limit);
        self
    }

    pub fn bind_addr(mut self, addr: String) -> Self {
        self.bind_addr = Some(addr);
        self
    }

    pub fn pool_capacity(mut self, capacity: usize) -> Self {
        self.pool_capacity = Some(capacity);
        self
    }

    pub fn pool_max_packet_size(mut self, size: usize) -> Self {
        self.pool_max_packet_size = Some(size);
        self
    }

    pub fn ack_batch_size(mut self, size: usize) -> Self {
        self.ack_batch_size = Some(size);
        self
    }

    pub fn ack_batch_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.ack_batch_timeout_ms = Some(timeout_ms);
        self
    }

    pub fn coalescing_window_ms(mut self, window_ms: u64) -> Self {
        self.coalescing_window_ms = Some(window_ms);
        self
    }

    pub fn stun_servers(mut self, servers: Vec<String>) -> Self {
        self.stun_servers = Some(servers);
        self
    }

    pub fn stun_timeout(mut self, timeout: Duration) -> Self {
        self.stun_timeout = Some(timeout);
        self
    }

    pub fn stun_cache_ttl(mut self, ttl: Duration) -> Self {
        self.stun_cache_ttl = Some(ttl);
        self
    }

    pub fn enable_header_compression(mut self, enable: bool) -> Self {
        self.enable_header_compression = Some(enable);
        self
    }

    pub fn multihop_config(mut self, config: Option<crate::multihop::MultiHopConfig>) -> Self {
        self.multihop_config = Some(config);
        self
    }

    pub fn build(self) -> ConnectionConfig {
        let default = ConnectionConfig::default();
        ConnectionConfig {
            session_timeout: self.session_timeout.unwrap_or(default.session_timeout),
            heartbeat_interval: self.heartbeat_interval.unwrap_or(default.heartbeat_interval),
            heartbeat_timeout_count: self.heartbeat_timeout_count.unwrap_or(default.heartbeat_timeout_count),
            max_streams: self.max_streams.unwrap_or(default.max_streams),
            rate_limit_messages: self.rate_limit_messages.unwrap_or(default.rate_limit_messages),
            rate_limit_bytes: self.rate_limit_bytes.unwrap_or(default.rate_limit_bytes),
            bind_addr: self.bind_addr.or(default.bind_addr),
            pool_capacity: self.pool_capacity.unwrap_or(default.pool_capacity),
            pool_max_packet_size: self.pool_max_packet_size.unwrap_or(default.pool_max_packet_size),
            ack_batch_size: self.ack_batch_size.unwrap_or(default.ack_batch_size),
            ack_batch_timeout_ms: self.ack_batch_timeout_ms.unwrap_or(default.ack_batch_timeout_ms),
            coalescing_window_ms: self.coalescing_window_ms.unwrap_or(default.coalescing_window_ms),
            stun_servers: self.stun_servers.unwrap_or(default.stun_servers),
            stun_timeout: self.stun_timeout.unwrap_or(default.stun_timeout),
            stun_cache_ttl: self.stun_cache_ttl.unwrap_or(default.stun_cache_ttl),
            enable_header_compression: self.enable_header_compression.unwrap_or(default.enable_header_compression),
            multihop_config: self.multihop_config.unwrap_or(default.multihop_config),
        }
    }
}

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Connection configuration (applies to all connections)
    pub connection: ConnectionConfig,
    /// Global server-wide message rate limit
    pub global_rate_limit_messages: Option<u32>,
    /// Global server-wide byte rate limit
    pub global_rate_limit_bytes: Option<u64>,
    /// DDoS protection configuration
    pub ddos_config: DdosConfig,
    /// Session cleanup interval
    pub cleanup_interval: Duration,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            connection: ConnectionConfig::default(),
            global_rate_limit_messages: Some(10_000),
            global_rate_limit_bytes: Some(100_000_000), // 100 MB/s
            ddos_config: DdosConfig::default(),
            cleanup_interval: Duration::from_secs(10),
        }
    }
}

impl ServerConfig {
    pub fn builder() -> ServerConfigBuilder {
        ServerConfigBuilder::default()
    }
}

/// Builder for ServerConfig
#[derive(Debug, Default)]
pub struct ServerConfigBuilder {
    connection: Option<ConnectionConfig>,
    global_rate_limit_messages: Option<Option<u32>>,
    global_rate_limit_bytes: Option<Option<u64>>,
    ddos_config: Option<DdosConfig>,
    cleanup_interval: Option<Duration>,
}

impl ServerConfigBuilder {
    pub fn connection(mut self, config: ConnectionConfig) -> Self {
        self.connection = Some(config);
        self
    }

    pub fn global_rate_limit_messages(mut self, limit: Option<u32>) -> Self {
        self.global_rate_limit_messages = Some(limit);
        self
    }

    pub fn global_rate_limit_bytes(mut self, limit: Option<u64>) -> Self {
        self.global_rate_limit_bytes = Some(limit);
        self
    }

    pub fn ddos_config(mut self, config: DdosConfig) -> Self {
        self.ddos_config = Some(config);
        self
    }

    pub fn cleanup_interval(mut self, interval: Duration) -> Self {
        self.cleanup_interval = Some(interval);
        self
    }

    pub fn build(self) -> ServerConfig {
        let default = ServerConfig::default();
        ServerConfig {
            connection: self.connection.unwrap_or(default.connection),
            global_rate_limit_messages: self.global_rate_limit_messages.unwrap_or(default.global_rate_limit_messages),
            global_rate_limit_bytes: self.global_rate_limit_bytes.unwrap_or(default.global_rate_limit_bytes),
            ddos_config: self.ddos_config.unwrap_or(default.ddos_config),
            cleanup_interval: self.cleanup_interval.unwrap_or(default.cleanup_interval),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_connection_config() {
        let config = ConnectionConfig::default();
        assert_eq!(config.session_timeout, Duration::from_secs(30));
        assert_eq!(config.heartbeat_interval, Duration::from_secs(5));
        assert_eq!(config.max_streams, 100);
    }

    #[test]
    fn test_connection_config_builder() {
        let config = ConnectionConfig::builder()
            .session_timeout(Duration::from_secs(60))
            .max_streams(50)
            .build();
        
        assert_eq!(config.session_timeout, Duration::from_secs(60));
        assert_eq!(config.max_streams, 50);
        // Other fields should use defaults
        assert_eq!(config.heartbeat_interval, Duration::from_secs(5));
    }

    #[test]
    fn test_server_config_builder() {
        let config = ServerConfig::builder()
            .global_rate_limit_messages(Some(5000))
            .cleanup_interval(Duration::from_secs(5))
            .build();
        
        assert_eq!(config.global_rate_limit_messages, Some(5000));
        assert_eq!(config.cleanup_interval, Duration::from_secs(5));
    }
}
