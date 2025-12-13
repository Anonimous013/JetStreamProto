//! Metrics Registry
//! 
//! Central registry for all Prometheus metrics.

use prometheus::{Registry, IntCounter, IntGauge, Histogram, HistogramOpts, Opts};

/// Metrics registry for JetStreamProto
pub struct MetricsRegistry {
    registry: Registry,
    
    // Connection metrics
    pub connections_total: IntCounter,
    pub connections_active: IntGauge,
    pub connection_duration: Histogram,
    pub handshake_duration: Histogram,
    
    // Transport metrics
    pub bytes_sent_total: IntCounter,
    pub bytes_received_total: IntCounter,
    pub packets_sent_total: IntCounter,
    pub packets_received_total: IntCounter,
    
    // Error metrics
    pub errors_total: IntCounter,
    pub timeouts_total: IntCounter,
    pub retransmissions_total: IntCounter,
}

impl MetricsRegistry {
    /// Create a new metrics registry
    pub fn new() -> Self {
        let registry = Registry::new();
        
        // Connection metrics
        let connections_total = IntCounter::with_opts(
            Opts::new("jsp_connections_total", "Total number of connections established")
        ).unwrap();
        registry.register(Box::new(connections_total.clone())).unwrap();
        
        let connections_active = IntGauge::with_opts(
            Opts::new("jsp_connections_active", "Number of currently active connections")
        ).unwrap();
        registry.register(Box::new(connections_active.clone())).unwrap();
        
        let connection_duration = Histogram::with_opts(
            HistogramOpts::new("jsp_connection_duration_seconds", "Connection duration in seconds")
                .buckets(vec![0.1, 0.5, 1.0, 5.0, 10.0, 30.0, 60.0, 300.0])
        ).unwrap();
        registry.register(Box::new(connection_duration.clone())).unwrap();
        
        let handshake_duration = Histogram::with_opts(
            HistogramOpts::new("jsp_handshake_duration_seconds", "Handshake duration in seconds")
                .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0])
        ).unwrap();
        registry.register(Box::new(handshake_duration.clone())).unwrap();
        
        // Transport metrics
        let bytes_sent_total = IntCounter::with_opts(
            Opts::new("jsp_bytes_sent_total", "Total bytes sent")
        ).unwrap();
        registry.register(Box::new(bytes_sent_total.clone())).unwrap();
        
        let bytes_received_total = IntCounter::with_opts(
            Opts::new("jsp_bytes_received_total", "Total bytes received")
        ).unwrap();
        registry.register(Box::new(bytes_received_total.clone())).unwrap();
        
        let packets_sent_total = IntCounter::with_opts(
            Opts::new("jsp_packets_sent_total", "Total packets sent")
        ).unwrap();
        registry.register(Box::new(packets_sent_total.clone())).unwrap();
        
        let packets_received_total = IntCounter::with_opts(
            Opts::new("jsp_packets_received_total", "Total packets received")
        ).unwrap();
        registry.register(Box::new(packets_received_total.clone())).unwrap();
        
        // Error metrics
        let errors_total = IntCounter::with_opts(
            Opts::new("jsp_errors_total", "Total number of errors")
        ).unwrap();
        registry.register(Box::new(errors_total.clone())).unwrap();
        
        let timeouts_total = IntCounter::with_opts(
            Opts::new("jsp_timeouts_total", "Total number of timeouts")
        ).unwrap();
        registry.register(Box::new(timeouts_total.clone())).unwrap();
        
        let retransmissions_total = IntCounter::with_opts(
            Opts::new("jsp_retransmissions_total", "Total number of retransmissions")
        ).unwrap();
        registry.register(Box::new(retransmissions_total.clone())).unwrap();
        
        Self {
            registry,
            connections_total,
            connections_active,
            connection_duration,
            handshake_duration,
            bytes_sent_total,
            bytes_received_total,
            packets_sent_total,
            packets_received_total,
            errors_total,
            timeouts_total,
            retransmissions_total,
        }
    }
    
    /// Get the underlying Prometheus registry
    pub fn registry(&self) -> &Registry {
        &self.registry
    }
    
    /// Record a new connection
    pub fn record_connection(&self) {
        self.connections_total.inc();
        self.connections_active.inc();
    }
    
    /// Record a connection close
    pub fn record_connection_close(&self, duration_secs: f64) {
        self.connections_active.dec();
        self.connection_duration.observe(duration_secs);
    }
    
    /// Record handshake duration
    pub fn record_handshake(&self, duration_secs: f64) {
        self.handshake_duration.observe(duration_secs);
    }
    
    /// Record bytes sent
    pub fn record_bytes_sent(&self, bytes: u64) {
        self.bytes_sent_total.inc_by(bytes);
    }
    
    /// Record bytes received
    pub fn record_bytes_received(&self, bytes: u64) {
        self.bytes_received_total.inc_by(bytes);
    }
    
    /// Record packet sent
    pub fn record_packet_sent(&self) {
        self.packets_sent_total.inc();
    }
    
    /// Record packet received
    pub fn record_packet_received(&self) {
        self.packets_received_total.inc();
    }
    
    /// Record an error
    pub fn record_error(&self) {
        self.errors_total.inc();
    }
    
    /// Record a timeout
    pub fn record_timeout(&self) {
        self.timeouts_total.inc();
    }
    
    /// Record a retransmission
    pub fn record_retransmission(&self) {
        self.retransmissions_total.inc();
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}
