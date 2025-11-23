# JetStreamProto API Reference

Complete API documentation for JetStreamProto v0.4.0

## Table of Contents

- [Connection](#connection)
- [Configuration](#configuration)
- [Session](#session)
- [Transport](#transport)
- [Metrics](#metrics)
- [Types](#types)

---

## Connection

The main interface for client-server communication.

### `Connection`

```rust
pub struct Connection {
    // Internal fields omitted
}
```

#### Methods

##### `connect_with_config`
```rust
pub async fn connect_with_config(
    addr: &str,
    config: ConnectionConfig
) -> Result<Self>
```

Connect to a server with custom configuration.

**Parameters:**
- `addr`: Server address (e.g., "127.0.0.1:8080")
- `config`: Connection configuration

**Returns:** `Result<Connection>`

**Example:**
```rust
let config = ConnectionConfig::default();
let conn = Connection::connect_with_config("127.0.0.1:8080", config).await?;
```

##### `listen_with_config`
```rust
pub async fn listen_with_config(
    addr: &str,
    config: ConnectionConfig
) -> Result<Self>
```

Start listening for incoming connections.

**Parameters:**
- `addr`: Bind address (e.g., "0.0.0.0:8080")
- `config`: Connection configuration

**Returns:** `Result<Connection>`

##### `send_on_stream`
```rust
pub async fn send_on_stream(
    &mut self,
    stream_id: u32,
    data: &[u8]
) -> Result<()>
```

Send data on a specific stream.

**Parameters:**
- `stream_id`: Stream identifier
- `data`: Data to send

**Example:**
```rust
conn.send_on_stream(1, b"Hello, World!").await?;
```

##### `recv`
```rust
pub async fn recv(&mut self) -> Result<Vec<(u32, Bytes)>>
```

Receive available packets.

**Returns:** Vector of `(stream_id, data)` tuples

**Example:**
```rust
let packets = conn.recv().await?;
for (stream_id, data) in packets {
    println!("Stream {}: {:?}", stream_id, data);
}
```

##### `close`
```rust
pub async fn close(
    &mut self,
    reason: CloseReason,
    message: Option<String>
) -> Result<()>
```

Close the connection gracefully.

**Parameters:**
- `reason`: Reason for closing
- `message`: Optional message

---

## Configuration

### `ConnectionConfig`

```rust
pub struct ConnectionConfig {
    pub heartbeat_interval: Duration,
    pub heartbeat_timeout_count: u32,
    pub max_packet_size: usize,
    pub enable_compression: bool,
    pub enable_fec: bool,
    pub qos_enabled: bool,
    // ... more fields
}
```

#### Default Values

```rust
ConnectionConfig {
    heartbeat_interval: Duration::from_secs(5),
    heartbeat_timeout_count: 3,
    max_packet_size: 1400,
    enable_compression: true,
    enable_fec: true,
    qos_enabled: true,
}
```

#### Methods

##### `default`
```rust
pub fn default() -> Self
```

Create configuration with default values.

##### `with_heartbeat`
```rust
pub fn with_heartbeat(mut self, interval: Duration) -> Self
```

Set heartbeat interval (builder pattern).

**Example:**
```rust
let config = ConnectionConfig::default()
    .with_heartbeat(Duration::from_secs(10));
```

---

## Session

Session management and cryptographic state.

### `Session`

```rust
pub struct Session {
    // Internal fields omitted
}
```

#### Methods

##### `new`
```rust
pub fn new(connection_id: u64, is_server: bool) -> Self
```

Create a new session.

##### `handshake`
```rust
pub async fn handshake(&mut self) -> Result<()>
```

Perform cryptographic handshake.

---

## Transport

Multi-transport abstraction layer.

### `Transport`

```rust
pub enum Transport {
    Udp(UdpSocket),
    Tcp(TcpTransport),
    Quic(QuicTransport),
}
```

#### Methods

##### `send`
```rust
pub async fn send(&mut self, data: &[u8]) -> Result<usize>
```

Send data via the transport.

##### `recv`
```rust
pub async fn recv(&mut self, buf: &mut [u8]) -> Result<usize>
```

Receive data from the transport.

---

## Metrics

Performance monitoring and statistics.

### `Metrics`

```rust
pub struct Metrics {
    // Atomic counters
}
```

#### Methods

##### `snapshot`
```rust
pub fn snapshot(&self) -> MetricsSnapshot
```

Get current metrics snapshot.

**Returns:** `MetricsSnapshot` with all counters

##### `reset`
```rust
pub fn reset(&self)
```

Reset all counters to zero.

### `MetricsSnapshot`

```rust
pub struct MetricsSnapshot {
    pub packets_sent: u64,
    pub packets_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub rtt_ms: u64,
    pub packet_loss_rate: f64,
    // ... more fields
}
```

**Example:**
```rust
let snapshot = conn.metrics().snapshot();
println!("RTT: {}ms", snapshot.rtt_ms);
println!("Loss: {:.2}%", snapshot.packet_loss_rate * 100.0);
```

---

## Types

### `Priority`

```rust
pub enum Priority {
    System = 0,   // Highest priority
    Media = 1,
    Chat = 2,
    Bulk = 3,     // Lowest priority
}
```

### `CloseReason`

```rust
pub enum CloseReason {
    Normal,
    Error,
    Timeout,
    Shutdown,
}
```

### `NetworkType`

```rust
pub enum NetworkType {
    Unknown,
    Wifi,
    Cellular,
    Ethernet,
}
```

### `AppState`

```rust
pub enum AppState {
    Foreground,
    Background,
}
```

---

## Mobile Optimizations

### Adaptive Compression

```rust
pub fn set_network_type(&self, net_type: NetworkType)
```

Update network type for adaptive optimizations.

### Battery-Aware Heartbeats

```rust
pub async fn set_app_state(&self, state: AppState)
```

Change application state (affects heartbeat interval).

**Example:**
```rust
// App goes to background
conn.set_app_state(AppState::Background).await;

// Heartbeat interval increases to 30s to save battery
```

---

## Error Handling

All async methods return `Result<T, Error>` where `Error` implements `std::error::Error`.

### Common Errors

- `ConnectionClosed`: Connection was closed
- `Timeout`: Operation timed out
- `InvalidPacket`: Malformed packet received
- `EncryptionError`: Cryptographic operation failed

**Example:**
```rust
match conn.send_on_stream(1, data).await {
    Ok(_) => println!("Sent successfully"),
    Err(e) => eprintln!("Send failed: {}", e),
}
```

---

## Best Practices

1. **Always use `ConnectionConfig`** for production deployments
2. **Monitor metrics** regularly for performance insights
3. **Handle errors** gracefully with proper logging
4. **Use QoS priorities** for different message types
5. **Enable FEC** for lossy networks
6. **Set app state** on mobile for battery optimization

---

## See Also

- [Architecture Guide](ARCHITECTURE.md)
- [Performance Guide](PERFORMANCE.md)
- [Examples](../examples/)
