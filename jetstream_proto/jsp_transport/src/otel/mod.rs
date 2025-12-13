//! OpenTelemetry Distributed Tracing Module
//! 
//! Simplified tracing implementation for JetStreamProto.

use std::collections::HashMap;
use std::time::Instant;

/// Trace span
pub struct Span {
    name: String,
    start_time: Instant,
    attributes: HashMap<String, String>,
    events: Vec<(String, HashMap<String, String>)>,
}

impl Span {
    /// Create a new span
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            start_time: Instant::now(),
            attributes: HashMap::new(),
            events: Vec::new(),
        }
    }

    /// Set attribute
    pub fn set_attribute(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.attributes.insert(key.into(), value.into());
    }

    /// Add event
    pub fn add_event(&mut self, name: impl Into<String>, attrs: HashMap<String, String>) {
        self.events.push((name.into(), attrs));
    }

    /// End span and return duration
    pub fn end(self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    /// Get span name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get attributes
    pub fn attributes(&self) -> &HashMap<String, String> {
        &self.attributes
    }
}

/// Simple tracer
pub struct Tracer {
    service_name: String,
}

impl Tracer {
    /// Create a new tracer
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
        }
    }

    /// Start a new span
    pub fn start_span(&self, name: impl Into<String>) -> Span {
        Span::new(name)
    }

    /// Get service name
    pub fn service_name(&self) -> &str {
        &self.service_name
    }
}

/// Global tracer
static GLOBAL_TRACER: std::sync::OnceLock<Tracer> = std::sync::OnceLock::new();

/// Initialize global tracer
pub fn init_tracer(service_name: impl Into<String>) -> Result<(), Box<dyn std::error::Error>> {
    GLOBAL_TRACER.set(Tracer::new(service_name))
        .map_err(|_| "Tracer already initialized")?;
    Ok(())
}

/// Get global tracer
pub fn global_tracer() -> &'static Tracer {
    GLOBAL_TRACER.get().expect("Tracer not initialized")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_creation() {
        let mut span = Span::new("test_span");
        span.set_attribute("key", "value");
        assert_eq!(span.name(), "test_span");
        assert_eq!(span.attributes().get("key").unwrap(), "value");
    }

    #[test]
    fn test_tracer() {
        let tracer = Tracer::new("test_service");
        let span = tracer.start_span("test");
        assert_eq!(span.name(), "test");
    }
}
