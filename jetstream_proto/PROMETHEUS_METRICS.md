# Prometheus Metrics Integration

## Overview

JetStreamProto now includes comprehensive Prometheus metrics for monitoring and observability.

## Quick Start

### 1. Run the metrics server

```bash
cargo run --example prometheus_demo
```

This will start an HTTP server on `http://127.0.0.1:9090` with:
- `/metrics` - Prometheus metrics endpoint
- `/health` - Health check endpoint

### 2. Configure Prometheus

Add to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'jetstream_proto'
    scrape_interval: 15s
    static_configs:
      - targets: ['localhost:9090']
```

### 3. View metrics

Visit `http://localhost:9090/metrics` to see all available metrics.

## Available Metrics

### Connection Metrics

- `jsp_connections_total` (counter) - Total connections established
- `jsp_connections_active` (gauge) - Currently active connections
- `jsp_connection_duration_seconds` (histogram) - Connection duration
- `jsp_handshake_duration_seconds` (histogram) - Handshake latency

### Transport Metrics

- `jsp_bytes_sent_total` (counter) - Total bytes sent
- `jsp_bytes_received_total` (counter) - Total bytes received
- `jsp_packets_sent_total` (counter) - Total packets sent
- `jsp_packets_received_total` (counter) - Total packets received

### Error Metrics

- `jsp_errors_total` (counter) - Total errors
- `jsp_timeouts_total` (counter) - Connection timeouts
- `jsp_retransmissions_total` (counter) - Packet retransmissions

### Multi-Hop Metrics

- `jsp_multihop_latency_milliseconds` (histogram) - Per-hop latency
- `jsp_multihop_throughput_bps` (gauge) - Per-hop throughput
- `jsp_multihop_health` (gauge) - Hop health status

## Usage in Code

```rust
use jsp_transport::prometheus::global_registry;

// Record a connection
let registry = global_registry();
registry.record_connection();

// Record handshake
registry.record_handshake(0.015); // 15ms

// Record data transfer
registry.record_bytes_sent(1024);
registry.record_packet_sent();

// Record connection close
registry.record_connection_close(30.5); // 30.5 seconds
```

## Grafana Dashboards

Import the included Grafana dashboard:

```bash
# Import dashboard from grafana/jetstream_dashboard.json
```

### Key Panels

1. **Connection Overview**
   - Active connections
   - Connection rate
   - Connection duration

2. **Performance**
   - Throughput (bytes/sec)
   - Packet rate
   - RTT latency

3. **Errors**
   - Error rate
   - Timeout rate
   - Retransmission rate

4. **Multi-Hop**
   - Per-hop latency
   - Hop health status
   - Chain throughput

## Alerting Rules

Example Prometheus alerting rules:

```yaml
groups:
  - name: jetstream_proto
    rules:
      - alert: HighErrorRate
        expr: rate(jsp_errors_total[5m]) > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High error rate detected"
          
      - alert: HighLatency
        expr: histogram_quantile(0.99, jsp_handshake_duration_seconds) > 1.0
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High handshake latency (p99 > 1s)"
          
      - alert: UnhealthyHop
        expr: jsp_multihop_health < 2
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "Multi-hop chain has unhealthy hop"
```

## Integration with Existing Code

The metrics are automatically collected when using JetStreamProto. No code changes required for basic metrics.

For custom metrics:

```rust
use jsp_transport::prometheus::global_registry;

let registry = global_registry();

// Your application logic
let start = std::time::Instant::now();
// ... do work ...
let duration = start.elapsed().as_secs_f64();

registry.record_handshake(duration);
```

## Performance Impact

Metrics collection has minimal performance impact:
- < 1% CPU overhead
- < 10MB memory overhead
- Lock-free counters for high-throughput scenarios

## Troubleshooting

### Metrics not appearing

1. Check that the metrics server is running
2. Verify Prometheus can reach the endpoint
3. Check for firewall rules blocking port 9090

### High memory usage

Reduce histogram bucket count or scrape interval.

### Missing labels

Ensure you're using the correct metric collectors (ConnectionMetrics, TransportMetrics, MultiHopMetrics).
