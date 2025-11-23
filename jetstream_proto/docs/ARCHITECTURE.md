# JetStreamProto Architecture

System design and component interactions for JetStreamProto v0.4.0

## Overview

JetStreamProto is designed as a layered architecture with clear separation of concerns:

```
┌───────────────────────────────────────────────────────────┐
│                     Application Layer                      │
│            (User Code, Examples, Bindings)                 │
└───────────────────────────────────────────────────────────┘
                            ▼
┌───────────────────────────────────────────────────────────┐
│                    Connection Layer                        │
│  ┌──────────┬──────────┬──────────┬──────────────────┐   │
│  │ Session  │ Streams  │ Metrics  │ Heartbeat        │   │
│  │ Manager  │ Mux      │ Tracking │ Manager          │   │
│  └──────────┴──────────┴──────────┴──────────────────┘   │
└───────────────────────────────────────────────────────────┘
                            ▼
┌───────────────────────────────────────────────────────────┐
│                    Transport Layer                         │
│  ┌──────────┬──────────┬──────────┬──────────────────┐   │
│  │   UDP    │   TCP    │   QUIC   │  Reliability     │   │
│  │ Socket   │ Fallback │ Support  │  Layer           │   │
│  └──────────┴──────────┴──────────┴──────────────────┘   │
└───────────────────────────────────────────────────────────┘
                            ▼
┌───────────────────────────────────────────────────────────┐
│                      Core Protocol                         │
│  ┌──────────┬──────────┬──────────┬──────────────────┐   │
│  │ Crypto   │ Codec    │  Types   │  Compression     │   │
│  │ (ChaCha) │ (CBOR/FB)│ (Header) │  (LZ4)           │   │
│  └──────────┴──────────┴──────────┴──────────────────┘   │
└───────────────────────────────────────────────────────────┘
```

## Core Components

### 1. Connection (`jsp_transport/connection.rs`)

The main entry point for all network operations.

**Responsibilities:**
- Session lifecycle management
- Stream multiplexing
- Packet routing
- Metrics collection
- Heartbeat coordination

**Key Features:**
- Async/await based API
- Thread-safe (Arc + Mutex)
- Configurable behavior
- Multi-transport support

**Data Flow:**
```
Application
    ↓ send_on_stream()
Connection
    ↓ encode + encrypt
Session
    ↓ packetize
Transport
    ↓ UDP/TCP/QUIC
Network
```

### 2. Session (`jsp_transport/session.rs`)

Manages cryptographic state and packet framing.

**Responsibilities:**
- Key exchange (Kyber768)
- Encryption/Decryption (ChaCha20-Poly1305)
- Packet serialization
- Connection ID management

**Handshake Flow:**
```
Client                          Server
  │                               │
  ├─── ClientHello ──────────────>│
  │    (Kyber PK, Nonce)          │
  │                               │
  │<──── ServerHello ─────────────┤
  │    (Kyber CT, Server PK)      │
  │                               │
  ├─── Derive Shared Secret ──────┤
  │                               │
  └─── Encrypted Communication ───┘
```

### 3. Transport Layer

#### UDP Transport (Default)
- Raw socket operations
- SO_REUSEPORT for multi-threading
- 4MB send/receive buffers
- MTU discovery

#### TCP Fallback
- Automatic fallback detection
- Length-prefixed framing
- Keep-alive support

#### QUIC Support
- Quinn library integration
- 0-RTT resumption
- Connection migration
- Built-in TLS 1.3

### 4. Reliability Layer (`jsp_transport/reliability.rs`)

Ensures reliable delivery over unreliable transports.

**Features:**
- Sequence numbers
- ACK/NACK mechanism
- Retransmission with exponential backoff
- Congestion control (NewReno)
- FEC (Reed-Solomon)

**Packet Flow:**
```
Send Path:
  Data → Reliability Layer → Add Seq# → FEC Encode → Transport

Receive Path:
  Transport → FEC Decode → Check Seq# → Reorder → Deliver
```

### 5. Compression (`jsp_transport/compression/`)

#### LZ4 Compression
- Fast compression/decompression
- Adaptive enable/disable
- Configurable threshold

#### Adaptive Compression
- RTT-based level adjustment
- Packet loss awareness
- Mobile optimization

### 6. QoS (`jsp_transport/qos.rs`)

Priority-based packet scheduling.

**Priority Levels:**
```
System (0) ─────> Highest priority (control packets)
Media  (1) ─────> Video/audio streams
Chat   (2) ─────> Text messages
Bulk   (3) ─────> File transfers (lowest)
```

**Scheduling:**
- Weighted Fair Queuing
- Per-priority queues
- Starvation prevention

### 7. Metrics (`jsp_transport/metrics.rs`)

Real-time performance monitoring.

**Tracked Metrics:**
- Packets sent/received
- Bytes sent/received
- RTT (Round Trip Time)
- Packet loss rate
- Retransmission count
- Compression ratio

**Implementation:**
- Atomic counters (lock-free)
- Snapshot API
- Reset capability

## Advanced Features

### Mobile Optimizations

#### Network Status Awareness
```rust
NetworkStatus
  ├─ WiFi ────> Full performance
  ├─ Cellular ─> Adaptive compression
  └─ Unknown ──> Conservative mode
```

#### Battery-Aware Heartbeats
```
Foreground: 5s interval  (responsive)
Background: 30s interval (battery saving)
```

### Gateway/Load Balancer

**Architecture:**
```
Clients                Gateway              Backends
  │                      │                    │
  ├─────────────────────>│                    │
  │   (Session 1)        │                    │
  │                      ├──────────────────> │ Backend 1
  │                      │   (NAT mapping)    │
  │                      │                    │
  ├─────────────────────>│                    │
  │   (Session 2)        │                    │
  │                      ├──────────────────> │ Backend 2
  │                      │   (Round Robin)    │
```

**Features:**
- Session stickiness
- Round Robin load balancing
- Health checking (planned)
- Service discovery (planned)

## Security Model

### Encryption

**Algorithm:** ChaCha20-Poly1305
- 256-bit keys
- 96-bit nonces
- AEAD (Authenticated Encryption)

**Key Derivation:**
```
Kyber768 KEM
    ↓
Shared Secret (256 bits)
    ↓
HKDF-SHA256
    ↓
Session Keys (Tx/Rx)
```

### Post-Quantum Security

- **Kyber768**: NIST PQC finalist
- **Hybrid approach**: Can combine with X25519
- **Forward secrecy**: Session-based keys

## Performance Optimizations

### Zero-Copy Where Possible
- `Bytes` for reference-counted buffers
- Avoid unnecessary allocations
- Reuse packet buffers

### Async I/O
- Tokio runtime
- Non-blocking operations
- Efficient task scheduling

### Lock-Free Metrics
- Atomic operations
- No mutex contention
- Minimal overhead

## Scalability

### Concurrent Connections
- Tokio task per connection
- Shared transport layer
- Efficient resource usage

### Memory Management
- Connection pooling (planned)
- Buffer recycling
- Bounded queues

## Error Handling

### Error Propagation
```rust
Result<T, Error>
    ↓
Custom Error types
    ↓
Contextual information
```

### Recovery Strategies
- Automatic retransmission
- Transport fallback
- Circuit breaker
- Graceful degradation

## Testing Strategy

### Unit Tests
- Per-component testing
- Mock dependencies
- Edge case coverage

### Integration Tests
- End-to-end scenarios
- Multi-transport testing
- Failure injection

### Benchmarks
- Throughput measurement
- Latency profiling
- Resource usage tracking

## Future Enhancements

1. **Connection Pooling**: Reuse connections
2. **WebRTC Transport**: Browser support
3. **Multipath**: Multiple network interfaces
4. **BBR Congestion Control**: Better throughput
5. **DTLS Support**: Alternative to custom crypto

## See Also

- [API Reference](API.md)
- [Performance Guide](PERFORMANCE.md)
- [Examples](../examples/)
