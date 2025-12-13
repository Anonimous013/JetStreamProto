//! Trace Context Propagation

use opentelemetry::{
    propagation::{Extractor, Injector, TextMapPropagator},
    trace::TraceContextExt,
    Context,
};
use std::collections::HashMap;

/// Trace context for propagating across services
#[derive(Debug, Clone)]
pub struct TraceContext {
    trace_id: String,
    span_id: String,
    trace_flags: u8,
    metadata: HashMap<String, String>,
}

impl TraceContext {
    /// Create a new trace context
    pub fn new() -> Self {
        let context = Context::current();
        let span = context.span();
        let span_context = span.span_context();
        
        Self {
            trace_id: span_context.trace_id().to_string(),
            span_id: span_context.span_id().to_string(),
            trace_flags: span_context.trace_flags().to_u8(),
            metadata: HashMap::new(),
        }
    }

    /// Get trace ID
    pub fn trace_id(&self) -> &str {
        &self.trace_id
    }

    /// Get span ID
    pub fn span_id(&self) -> &str {
        &self.span_id
    }

    /// Inject context into headers
    pub fn inject(&self, headers: &mut HashMap<String, String>) {
        let propagator = opentelemetry_sdk::propagation::TraceContextPropagator::new();
        let context = Context::current();
        propagator.inject_context(&context, &mut HeaderInjector(headers));
    }

    /// Extract context from headers
    pub fn extract(headers: &HashMap<String, String>) -> Self {
        let propagator = opentelemetry_sdk::propagation::TraceContextPropagator::new();
        let context = propagator.extract(&HeaderExtractor(headers));
        
        let span = context.span();
        let span_context = span.span_context();
        
        Self {
            trace_id: span_context.trace_id().to_string(),
            span_id: span_context.span_id().to_string(),
            trace_flags: span_context.trace_flags().to_u8(),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Get metadata
    pub fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }
}

impl Default for TraceContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Header injector for propagation
struct HeaderInjector<'a>(&'a mut HashMap<String, String>);

impl<'a> Injector for HeaderInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        self.0.insert(key.to_string(), value);
    }
}

/// Header extractor for propagation
struct HeaderExtractor<'a>(&'a HashMap<String, String>);

impl<'a> Extractor for HeaderExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(|v| v.as_str())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_context_creation() {
        let context = TraceContext::new();
        assert!(!context.trace_id().is_empty());
    }

    #[test]
    fn test_context_inject_extract() {
        let context = TraceContext::new();
        let mut headers = HashMap::new();
        
        context.inject(&mut headers);
        assert!(!headers.is_empty());
        
        let extracted = TraceContext::extract(&headers);
        assert_eq!(extracted.trace_id(), context.trace_id());
    }
}
