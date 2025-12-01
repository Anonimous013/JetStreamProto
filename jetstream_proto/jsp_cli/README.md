# JetStreamProto CLI Tools

Command-line tools for monitoring and managing JetStreamProto connections.

## Installation

```bash
cargo build --release -p jsp_cli
```

The binary will be available at `target/release/jsp-cli`.

## Commands

### Monitor

Monitor connection status and metrics in real-time.

```bash
jsp-cli monitor --addr 127.0.0.1:8080 --interval 1
```

**Options:**
- `-a, --addr <ADDR>` - Server address (default: 127.0.0.1:8080)
- `-i, --interval <SECS>` - Update interval in seconds (default: 1)

**Example Output:**
```
JetStreamProto Connection Monitor
==================================================
Connecting to: 127.0.0.1:8080

✓ Connected successfully
✓ Handshake completed

Session ID: 12345

Update 1
  Status: Connected
  Session: 12345
  Transport: UDP
  Throughput: 5.2 MB/s
  Latency: 45 ms
  Packet Loss: 0.1%
```

### Profile

Profile connection performance over a duration.

```bash
jsp-cli profile --addr 127.0.0.1:8080 --duration 60 --output report.json
```

**Options:**
- `-a, --addr <ADDR>` - Server address (default: 127.0.0.1:8080)
- `-d, --duration <SECS>` - Duration in seconds (default: 60)
- `-o, --output <FILE>` - Output file for JSON report (optional)

**Example Output:**
```
JetStreamProto Performance Profiler
==================================================
Target: 127.0.0.1:8080
Duration: 60 seconds

Connecting...
✓ Connected

Stream ID: 1

Profiling...

Profile Results
==================================================
Duration: 60 seconds
Messages Sent: 5432
Total Bytes: 354880 bytes
Avg Throughput: 47.32 Mbps
Avg Latency: 45.00 ms
Packet Loss: 0.10%

Report saved to: report.json
```

### Config

Manage configuration files.

#### Generate

Generate default configuration:

```bash
jsp-cli config generate --output config.json
```

#### Validate

Validate configuration file:

```bash
jsp-cli config validate --file config.json
```

#### Show

Show current configuration:

```bash
jsp-cli config show
```

**Example Output:**
```
Current Configuration
==================================================
Server Address: 127.0.0.1:8080
Session Timeout: 300 seconds
Heartbeat Interval: 30 seconds
Max Streams: 100
Rate Limit: 1000 messages/sec
Compression: Enabled
Encryption: Enabled
```

### Send

Send test messages to server.

```bash
jsp-cli send --addr 127.0.0.1:8080 --message "Hello!" --count 10
```

**Options:**
- `-a, --addr <ADDR>` - Server address (default: 127.0.0.1:8080)
- `-m, --message <MSG>` - Message to send (default: "Hello, JetStream!")
- `-c, --count <N>` - Number of messages (default: 1)

**Example Output:**
```
JetStreamProto Send Test
==================================================
Target: 127.0.0.1:8080
Message: Hello!
Count: 10

Connecting...
✓ Connected

Stream ID: 1

Sending messages...
  ✓ Sent: Hello! #1
  ✓ Sent: Hello! #2
  ✓ Sent: Hello! #3
  ...

✓ Sent 10 messages successfully
```

## Configuration File Format

```json
{
  "server_address": "127.0.0.1:8080",
  "session_timeout_secs": 300,
  "heartbeat_interval_secs": 30,
  "max_streams": 100,
  "rate_limit_messages": 1000,
  "enable_compression": true,
  "enable_encryption": true
}
```

## Usage Examples

### Quick Connection Test
```bash
jsp-cli send --addr 127.0.0.1:8080 --message "Test" --count 1
```

### Performance Benchmark
```bash
jsp-cli profile --addr 127.0.0.1:8080 --duration 120 --output benchmark.json
```

### Continuous Monitoring
```bash
jsp-cli monitor --addr 127.0.0.1:8080 --interval 2
```

### Configuration Management
```bash
# Generate config
jsp-cli config generate --output my-config.json

# Edit my-config.json as needed

# Validate
jsp-cli config validate --file my-config.json
```

## Features

- ✅ Real-time connection monitoring
- ✅ Performance profiling with metrics
- ✅ Configuration generation and validation
- ✅ Test message sending
- ✅ Colored output for better readability
- ✅ JSON export for reports

## Dependencies

- `clap` - Command-line argument parsing
- `tokio` - Async runtime
- `serde_json` - JSON serialization
- `colored` - Terminal colors
- `tabled` - Table formatting

## License

Same as JetStreamProto project.
