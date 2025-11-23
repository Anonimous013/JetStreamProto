/// Initialize logging for development (human-readable format)
pub fn init_dev_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("debug"))
        )
        .with_target(false)
        .with_thread_ids(true)
        .with_line_number(true)
        .init();
}

/// Initialize logging for production (JSON format)
pub fn init_prod_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .json()
        .init();
}

/// Initialize logging with custom filter
pub fn init_logging_with_filter(filter: &str) {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(filter))
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .init();
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_logging_initialization() {
        // This test just ensures the functions compile
        // Actual initialization is tested in integration tests
        assert!(true);
    }
}
