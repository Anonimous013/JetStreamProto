//! Metrics Collectors
//! 
//! Specialized collectors for different components.

use prometheus::{IntGaugeVec, HistogramVec, HistogramOpts, Opts};

/// Connection-specific metrics
pub struct ConnectionMetrics {
    pub streams_active: IntGaugeVec,
    pub stream_duration: HistogramVec,
}

impl ConnectionMetrics {
    pub fn new() -> Self {
        let streams_active = IntGaugeVec::new(
            Opts::new("jsp_streams_active", "Number of active streams"),
            &["connection_id"]
        ).unwrap();
        
        let stream_duration = HistogramVec::new(
            HistogramOpts::new("jsp_stream_duration_seconds", "Stream duration in seconds")
                .buckets(vec![0.1, 0.5, 1.0, 5.0, 10.0, 30.0, 60.0]),
            &["connection_id", "stream_id"]
        ).unwrap();
        
        Self {
            streams_active,
            stream_duration,
        }
    }
}

/// Transport-specific metrics
pub struct TransportMetrics {
    pub transport_type: IntGaugeVec,
    pub rtt_ms: HistogramVec,
    pub packet_loss_ratio: IntGaugeVec,
}

impl TransportMetrics {
    pub fn new() -> Self {
        let transport_type = IntGaugeVec::new(
            Opts::new("jsp_transport_type", "Active transport type (0=UDP, 1=TCP, 2=QUIC, 3=WebRTC)"),
            &["connection_id"]
        ).unwrap();
        
        let rtt_ms = HistogramVec::new(
            HistogramOpts::new("jsp_rtt_milliseconds", "Round-trip time in milliseconds")
                .buckets(vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0]),
            &["transport"]
        ).unwrap();
        
        let packet_loss_ratio = IntGaugeVec::new(
            Opts::new("jsp_packet_loss_ratio", "Packet loss ratio (0-100)"),
            &["transport"]
        ).unwrap();
        
        Self {
            transport_type,
            rtt_ms,
            packet_loss_ratio,
        }
    }
}

/// Multi-hop specific metrics
pub struct MultiHopMetrics {
    pub hop_latency_ms: HistogramVec,
    pub hop_throughput_bps: IntGaugeVec,
    pub hop_health: IntGaugeVec,
}

impl MultiHopMetrics {
    pub fn new() -> Self {
        let hop_latency_ms = HistogramVec::new(
            HistogramOpts::new("jsp_multihop_latency_milliseconds", "Per-hop latency in milliseconds")
                .buckets(vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0]),
            &["hop_index", "hop_type"]
        ).unwrap();
        
        let hop_throughput_bps = IntGaugeVec::new(
            Opts::new("jsp_multihop_throughput_bps", "Per-hop throughput in bytes per second"),
            &["hop_index", "hop_type"]
        ).unwrap();
        
        let hop_health = IntGaugeVec::new(
            Opts::new("jsp_multihop_health", "Hop health status (0=unhealthy, 1=degraded, 2=healthy)"),
            &["hop_index", "hop_type"]
        ).unwrap();
        
        Self {
            hop_latency_ms,
            hop_throughput_bps,
            hop_health,
        }
    }
}
