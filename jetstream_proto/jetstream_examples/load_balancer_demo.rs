//! Load Balancer Example
//! 
//! Demonstrates advanced load balancing with JetStreamProto.

use jsp_transport::load_balancer::{
    LoadBalancer, Backend, RoundRobin, LeastConnections, WeightedRoundRobin,
    ConsistentHash, HealthChecker,
};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,jsp_transport=debug")
        .init();

    println!("🚀 JetStreamProto Load Balancer Demo");
    println!("====================================\n");

    // Create backends
    let backends = vec![
        Backend::new("backend1", "127.0.0.1:8001", 1),
        Backend::new("backend2", "127.0.0.1:8002", 2), // Higher weight
        Backend::new("backend3", "127.0.0.1:8003", 1),
    ];

    println!("📋 Backends:");
    for backend in &backends {
        println!("  - {} @ {} (weight: {})", backend.id, backend.address, backend.weight);
    }
    println!();

    // Test Round Robin
    println!("🔄 Round Robin Algorithm:");
    let lb_rr = LoadBalancer::new(
        backends.clone(),
        Arc::new(RoundRobin::new()),
        HealthChecker::default(),
    );

    for i in 1..=6 {
        if let Some(backend) = lb_rr.select_backend(None) {
            println!("  Request {}: {} @ {}", i, backend.id, backend.address);
        }
    }
    println!();

    // Test Least Connections
    println!("📊 Least Connections Algorithm:");
    let lb_lc = LoadBalancer::new(
        backends.clone(),
        Arc::new(LeastConnections::new()),
        HealthChecker::default(),
    );

    // Simulate some connections
    if let Some(b) = lb_lc.select_backend(None) {
        b.inc_connections();
        b.inc_connections();
        println!("  {} has 2 active connections", b.id);
    }

    for i in 1..=4 {
        if let Some(backend) = lb_lc.select_backend(None) {
            println!("  Request {}: {} (connections: {})", i, backend.id, backend.connections());
        }
    }
    println!();

    // Test Weighted Round Robin
    println!("⚖️  Weighted Round Robin Algorithm:");
    let lb_wrr = LoadBalancer::new(
        backends.clone(),
        Arc::new(WeightedRoundRobin::new()),
        HealthChecker::default(),
    );

    for i in 1..=8 {
        if let Some(backend) = lb_wrr.select_backend(None) {
            println!("  Request {}: {} (weight: {})", i, backend.id, backend.weight);
        }
    }
    println!();

    // Test Consistent Hashing
    println!("🔑 Consistent Hashing Algorithm:");
    let lb_ch = LoadBalancer::new(
        backends.clone(),
        Arc::new(ConsistentHash::new(100)),
        HealthChecker::default(),
    );

    let users = ["user123", "user456", "user789", "user123"]; // user123 appears twice
    for user in &users {
        if let Some(backend) = lb_ch.select_backend(Some(user)) {
            println!("  {} -> {} @ {}", user, backend.id, backend.address);
        }
    }
    println!();

    // Test Health Checking
    println!("💚 Health Checking:");
    let lb_health = LoadBalancer::new(
        backends.clone(),
        Arc::new(RoundRobin::new()),
        HealthChecker::new(Duration::from_secs(5), Duration::from_secs(2)),
    );

    println!("  Total backends: {}", lb_health.backend_count());
    println!("  Healthy backends: {}", lb_health.healthy_backend_count());
    
    // Start health checks (runs in background)
    lb_health.start_health_checks().await;
    println!("  Health checks started (interval: 5s)\n");

    // Test Dynamic Backend Management
    println!("🔧 Dynamic Backend Management:");
    let lb_dynamic = LoadBalancer::new(
        vec![Backend::new("initial", "127.0.0.1:8000", 1)],
        Arc::new(RoundRobin::new()),
        HealthChecker::default(),
    );

    println!("  Initial backends: {}", lb_dynamic.backend_count());
    
    lb_dynamic.add_backend(Backend::new("added1", "127.0.0.1:8004", 1));
    lb_dynamic.add_backend(Backend::new("added2", "127.0.0.1:8005", 1));
    println!("  After adding 2: {}", lb_dynamic.backend_count());
    
    lb_dynamic.remove_backend("initial");
    println!("  After removing 1: {}", lb_dynamic.backend_count());
    println!();

    println!("💡 Features:");
    println!("  ✅ Round Robin - Simple rotation");
    println!("  ✅ Least Connections - Load-based selection");
    println!("  ✅ Weighted Round Robin - Capacity-aware");
    println!("  ✅ Consistent Hashing - Session affinity");
    println!("  ✅ Health Checking - Active monitoring");
    println!("  ✅ Dynamic Management - Add/remove backends");
    println!();

    println!("✅ Load balancer demo completed!");

    Ok(())
}
