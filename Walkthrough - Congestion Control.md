# Walkthrough - Congestion Control (Variant 3)

This document details the implementation of the Congestion Control mechanism for JetStreamProto, focusing on the NewReno algorithm and Bandwidth Estimation.

## 1. Overview

We implemented a modular Congestion Control system with the following components:
- **`CongestionController` Trait**: Defines the interface for congestion control algorithms.
- **`NewReno` Implementation**: A standard TCP-like congestion control algorithm (RFC 6582/5681).
- **`BandwidthEstimator`**: A simple estimator based on delivery rate.
- **Integration**: Integrated into `ReliabilityLayer` and `Connection`.

## 2. Implementation Details

### 2.1 Congestion Controller Trait

Located in `jsp_transport/src/congestion.rs`, this trait allows for pluggable congestion control algorithms.

```rust
pub trait CongestionController: Send + Sync + std::fmt::Debug {
    fn on_packet_sent(&mut self, sent_bytes: usize);
    fn on_packet_acked(&mut self, acked_bytes: usize, rtt: Duration);
    fn on_packet_lost(&mut self, lost_bytes: usize);
    fn congestion_window(&self) -> usize;
    fn can_send(&self, inflight_bytes: usize) -> bool;
    fn state(&self) -> CongestionState;
}
```

### 2.2 NewReno Algorithm

We implemented the NewReno algorithm with the following states:
- **Slow Start**: Exponential growth of cwnd.
- **Congestion Avoidance**: Linear growth of cwnd.
- **Recovery**: Fast recovery after packet loss (simplified).

### 2.3 Integration with ReliabilityLayer

The `ReliabilityLayer` now owns a `Box<dyn CongestionController>`. It tracks `inflight_bytes` and notifies the controller of events:
- `track_sent_packet` -> `on_packet_sent`
- `on_ack` -> `on_packet_acked`
- `get_retransmits` -> `on_packet_lost`

### 2.4 Connection Integration

The `Connection::send_on_stream` method now checks `reliability.can_send()` before sending data. If the congestion window is full, it returns an error (which can be handled by buffering or backpressure in the future).

```rust
        // Check congestion window
        if !self.reliability.can_send() {
             tracing::warn!(...);
            return Err(anyhow::anyhow!("Congestion window full"));
        }
```

## 3. Verification

We added 4 new unit tests in `congestion.rs` and verified integration with existing tests.

### 3.1 Unit Tests
- `test_slow_start`: Verifies cwnd doubles in Slow Start.
- `test_congestion_avoidance_transition`: Verifies transition to Congestion Avoidance when ssthresh is reached.
- `test_packet_loss`: Verifies reaction to packet loss (cwnd reduction).
- `test_bandwidth_estimator`: Verifies basic functionality of bandwidth estimation.

### 3.2 Integration Tests
All existing integration tests passed, ensuring no regressions.

## 4. Next Steps

- **Pacing**: Implement packet pacing to avoid bursts.
- **BBR**: Implement BBR algorithm using the `BandwidthEstimator`.
- **Advanced Recovery**: Implement full Fast Recovery with partial ACKs.
