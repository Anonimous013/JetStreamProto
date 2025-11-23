# JetStreamProto Python SDK

Python bindings for JetStreamProto - a high-performance, secure UDP-based protocol.

## Installation

### From PyPI (when published)
```bash
pip install jetstream-proto
```

### From source
```bash
# Install maturin
pip install maturin

# Build and install
cd jsp_python
maturin develop --release
```

## Quick Start

### Client Example
```python
import jetstream_proto

# Create connection
conn = jetstream_proto.Connection()
conn.connect("127.0.0.1:8080")

# Send data
conn.send(stream_id=1, data=b"Hello, JetStream!")

# Receive data
packets = conn.recv()
for stream_id, data in packets:
    print(f"Stream {stream_id}: {data}")

# Close
conn.close()
```

### Server Example
```python
import jetstream_proto

# Create server
server = jetstream_proto.Server()
server.listen("127.0.0.1:8080")

# Receive data
packets = server.recv()
for stream_id, data in packets:
    print(f"Received on stream {stream_id}: {data}")
    # Echo back
    server.send(stream_id, data)
```

## Features

- ✅ High-performance UDP transport
- ✅ Built-in encryption (ChaCha20-Poly1305)
- ✅ Post-quantum cryptography (Kyber768)
- ✅ Multiple delivery modes (Reliable, PartiallyReliable, BestEffort)
- ✅ Stream multiplexing
- ✅ Congestion control (NewReno)
- ✅ FEC (Forward Error Correction)
- ✅ QUIC support
- ✅ Mobile optimizations

## API Reference

### Connection

#### `Connection()`
Create a new client connection.

#### `connect(addr: str) -> None`
Connect to a server at the given address (e.g., "127.0.0.1:8080").

#### `send(stream_id: int, data: bytes) -> None`
Send data on the specified stream.

#### `recv() -> List[Tuple[int, bytes]]`
Receive available packets. Returns list of (stream_id, data) tuples.

#### `close() -> None`
Close the connection.

### Server

#### `Server()`
Create a new server.

#### `listen(addr: str) -> None`
Start listening on the given address (e.g., "0.0.0.0:8080").

#### `recv() -> List[Tuple[int, bytes]]`
Receive available packets from clients.

#### `send(stream_id: int, data: bytes) -> None`
Send data on the specified stream.

## Development

### Building
```bash
maturin build --release
```

### Testing
```bash
# Install in development mode
maturin develop

# Run Python tests
pytest tests/
```

## License

MIT
