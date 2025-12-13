# Multi-Hop Tunnel Manager Example

This example demonstrates how to use the JetStreamProto multi-hop tunnel manager to create a 4-layer anonymity chain.

## Configuration

The example creates a chain with:
1. **WireGuard Entry** - First hop for initial encryption
2. **Shadowsocks** - Second hop with TLS obfuscation
3. **XRay VLESS/TLS** - Third hop with advanced obfuscation
4. **WireGuard Exit** - Final hop before reaching the internet

## Prerequisites

Before running this example, you need:

1. **WireGuard servers** configured at the endpoints
2. **Shadowsocks server** running with matching credentials
3. **XRay binary** installed and available in PATH
4. **Valid keys and UUIDs** for all hops

## Configuration File

You can also load configuration from a YAML file:

```yaml
enabled: true
hop_timeout_secs: 10
auto_failover: true
health_check_interval_secs: 30
chain:
  - type: wireguard
    endpoint: "your-server:51820"
    private_key: "base64-encoded-key"
    peer_public_key: "base64-encoded-peer-key"
    
  - type: shadowsocks
    endpoint: "your-server:8388"
    password: "your-password"
    method: "aes-256-gcm"
    obfs: "tls"
    
  - type: xray
    endpoint: "your-server:443"
    server_name: "example.com"
    uuid: "your-uuid"
    tls: true
    
  - type: wireguard_exit
    endpoint: "exit-server:51820"
    private_key: "exit-private-key"
    peer_public_key: "exit-peer-key"
```

Then load it:

```rust
let config = MultiHopConfig::from_yaml_file("multihop.yaml")?;
```

## Running

```bash
cargo run --example multihop_demo
```

## Expected Output

```
🚀 JetStreamProto Multi-Hop Tunnel Demo
========================================

📋 Configuration:
  - Hops: 4
  - Auto-failover: true
  - Health check interval: 30s

🔧 Creating multi-hop engine...
▶️  Starting multi-hop chain...
✅ Multi-hop chain started successfully!

📊 Chain Statistics:
  - Total hops: 4
  - Total latency: 245.32ms
  - Throughput: 1048576 bytes/sec

📤 Sending test data through multi-hop chain...
✅ Data sent successfully through all hops!
```

## Performance Considerations

- **Latency**: Expect 50-150ms per hop (total: 200-600ms for 4 hops)
- **Throughput**: Typically 75-85% of baseline due to encryption overhead
- **Memory**: ~10MB per hop
- **CPU**: Moderate usage for encryption/decryption

## Security

This configuration provides:
- **4-layer encryption** (each hop adds encryption)
- **Traffic obfuscation** (DPI resistance)
- **IP anonymization** (exit IP different from entry)
- **Forward secrecy** (WireGuard key rotation)

## Troubleshooting

If the example fails to start:

1. Check that all servers are reachable
2. Verify credentials are correct
3. Ensure XRay binary is in PATH
4. Check firewall rules allow connections
5. Review logs with `RUST_LOG=debug`
