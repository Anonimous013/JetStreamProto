# Multi-Hop Tunnel Manager - Quick Reference

## 🚀 Quick Start

```bash
# 1. Create configuration file
cp multihop_config.yaml my_config.yaml
# Edit my_config.yaml with your server details

# 2. Run the demo
cargo run --example multihop_demo

# 3. Or use in your code
```

## 📋 Configuration Options

### WireGuard Hop
```yaml
- type: wireguard  # or wireguard_exit for exit node
  endpoint: "server:51820"
  private_key: "base64-key"
  peer_public_key: "base64-peer-key"
  allowed_ips: ["0.0.0.0/0"]
  persistent_keepalive: 25  # seconds
  listen_port: 0  # 0 = auto
```

### Shadowsocks Hop
```yaml
- type: shadowsocks
  endpoint: "server:8388"
  password: "your-password"
  method: "aes-256-gcm"  # or chacha20-ietf-poly1305
  obfs: "tls"  # none, tls, http
  udp_relay: false
  local_port: 0
```

### XRay Hop
```yaml
- type: xray
  endpoint: "server:443"
  server_name: "example.com"
  uuid: "uuid-v4"
  tls: true
  websocket: false
  ws_path: "/"
  local_port: 0
```

## 💻 Usage in Code

```rust
use jsp_transport::multihop::{MultiHopConfig, MultiHopEngine};

// Load from YAML
let config = MultiHopConfig::from_yaml_file("multihop.yaml")?;

// Create and start engine
let mut engine = MultiHopEngine::new(config);
engine.start().await?;

// Send data through chain
engine.send(b"Hello, World!").await?;

// Get metrics
if let Some(router) = engine.router() {
    println!("Latency: {}ms", router.total_latency_ms().await);
    println!("Throughput: {} bps", router.total_throughput().await);
}

// Stop engine
engine.stop().await?;
```

## 🔧 Integration with JetStreamProto Connection

```rust
use jsp_transport::config::ConnectionConfig;
use jsp_transport::multihop::MultiHopConfig;

let multihop_config = MultiHopConfig::from_yaml_file("multihop.yaml")?;

let config = ConnectionConfig::builder()
    .multihop_config(Some(multihop_config))
    .build();

let conn = Connection::connect_with_config("127.0.0.1:8080", config).await?;
// Traffic now routes through multi-hop chain automatically
```

## 📊 Performance Tips

1. **Minimize Hops**: Each hop adds ~50-150ms latency
2. **Use Fast Servers**: Choose geographically close servers
3. **Enable Compression**: For text-heavy traffic
4. **Monitor Metrics**: Use controller API to track performance
5. **Tune Buffer Pool**: Adjust capacity based on traffic patterns

## 🔒 Security Best Practices

1. **Rotate Keys**: Change WireGuard keys regularly
2. **Strong Passwords**: Use 32+ character passwords for Shadowsocks
3. **Unique UUIDs**: Generate fresh UUIDs for each XRay deployment
4. **TLS Everywhere**: Enable TLS on all hops that support it
5. **Monitor Logs**: Watch for suspicious activity

## 🐛 Troubleshooting

### Connection Fails
- Check all server endpoints are reachable
- Verify credentials are correct
- Ensure XRay binary is in PATH
- Check firewall rules

### High Latency
- Reduce number of hops
- Choose faster servers
- Check network conditions
- Monitor per-hop metrics

### Low Throughput
- Check server bandwidth limits
- Verify no packet loss
- Adjust buffer pool size
- Monitor congestion

## 📚 Additional Resources

- [Full Documentation](MULTIHOP_README.md)
- [Example Demo](multihop_demo.rs)
- [Configuration Reference](multihop_config.yaml)
- [JetStreamProto README](../README.md)
