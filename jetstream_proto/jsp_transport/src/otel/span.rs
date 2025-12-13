//! OpenTelemetry Span

use opentelemetry::trace::{Status, Span as OtelSpan};
use opentelemetry_sdk::trace::Span;
use std::time::Instant;

/// JetStreamProto span wrapper
pub struct JspSpan {
    span: Span,
    start_time: Instant,
}

impl JspSpan {
    /// Create a new span
    pub(crate) fn new(span: Span) -> Self {
        Self {
            span,
            start_time: Instant::now(),
        }
    }

    /// Set span attribute
    pub fn set_attribute(&mut self, key: &str, value: impl Into<opentelemetry::Value>) {
        self.span.set_attribute(opentelemetry::KeyValue::new(key.to_string(), value.into()));
    }

    /// Add event to span
    pub fn add_event(&mut self, name: &str, attributes: Vec<opentelemetry::KeyValue>) {
        self.span.add_event(name, attributes);
    }

    /// Set span status
    pub fn set_status(&mut self, status: Status) {
        self.span.set_status(status);
    }

    /// Record error
    pub fn record_error(&mut self, error: &dyn std::error::Error) {
        self.span.record_error(error);
    }

    /// End the span
    pub fn end(mut self) {
        let duration = self.start_time.elapsed();
        self.set_attribute("duration_ms", duration.as_millis() as i64);
        self.span.end();
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }
}

/// Span builder for creating spans with attributes
pub struct SpanBuilder {
    name: String,
    attributes: Vec<opentelemetry::KeyValue>,
}

impl SpanBuilder {
    /// Create a new span builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            attributes: Vec::new(),
        }
    }

    /// Add attribute
    pub fn with_attribute(mut self, key: &str, value: impl Into<opentelemetry::Value>) -> Self {
        self.attributes.push(opentelemetry::KeyValue::new(key.to_string(), value.into()));
        self
    }

    /// Start the span
    pub fn start(self) -> JspSpan {
        let tracer = super::global_tracer();
        let mut span = tracer.start_span(&self.name);
        
        for attr in self.attributes {
            span.set_attribute(attr.key.as_str(), attr.value);
        }
        
        span
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_builder() {
        let builder = SpanBuilder::new("test_span")
            .with_attribute("key1", "value1")
            .with_attribute("key2", 42);
        
        assert_eq!(builder.name, "test_span");
        assert_eq!(builder.attributes.len(), 2);
    }
}
