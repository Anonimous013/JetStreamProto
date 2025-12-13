//! Prometheus Metrics Module
//! 
//! Provides comprehensive metrics collection and export for JetStreamProto.

pub mod registry;
pub mod collectors;
pub mod exporter;

pub use registry::MetricsRegistry;
pub use collectors::{ConnectionMetrics, TransportMetrics, MultiHopMetrics};
pub use exporter::MetricsExporter;

use prometheus::{Encoder, TextEncoder};

/// Global metrics registry
static METRICS_REGISTRY: once_cell::sync::Lazy<MetricsRegistry> = 
    once_cell::sync::Lazy::new(|| MetricsRegistry::new());

/// Get the global metrics registry
pub fn global_registry() -> &'static MetricsRegistry {
    &METRICS_REGISTRY
}

/// Export metrics in Prometheus format
pub fn export_metrics() -> Result<String, Box<dyn std::error::Error>> {
    let registry = global_registry().registry();
    let encoder = TextEncoder::new();
    let metric_families = registry.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer)?;
    Ok(String::from_utf8(buffer)?)
}
