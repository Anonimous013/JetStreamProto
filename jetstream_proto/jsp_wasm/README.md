# JetStreamProto WASM SDK

WebAssembly bindings for JetStreamProto - enabling high-performance networking in browsers and Node.js.

## ⚠️ Important Note

**Browser Limitation**: Web browsers do not support raw UDP sockets due to security restrictions. This WASM SDK provides the API structure, but requires one of the following transports:

1. **WebRTC DataChannels** (recommended for P2P)
2. **WebSockets** (for client-server)
3. **WebTransport** (future standard, limited browser support)

The current implementation is a **proof-of-concept** showing the API design. Full implementation would require integrating WebRTC or WebSocket transport layers.

## Installation

### From NPM (when published)
```bash
npm install jetstream-proto-wasm
```

### From source
```bash
# Install wasm-pack
cargo install wasm-pack

# Build for web
cd jsp_wasm
npm run build

# Build for Node.js
npm run build:nodejs
```

## Usage

### Browser (ES Modules)
```javascript
import init, { Connection } from './pkg/jetstream_proto_wasm.js';

async function main() {
    // Initialize WASM module
    await init();
    
    // Create connection
    const conn = new Connection();
    
    try {
        await conn.connect("127.0.0.1:8080");
        
        // Send data
        const data = new Uint8Array([1, 2, 3, 4]);
        await conn.send(1, data);
        
        // Receive data
        const packets = await conn.recv();
        console.log("Received:", packets);
        
        // Close
        await conn.close();
    } catch (error) {
        console.error("Error:", error);
    }
}

main();
```

### Node.js
```javascript
const { Connection } = require('./pkg-node/jetstream_proto_wasm.js');

async function main() {
    const conn = new Connection();
    
    try {
        await conn.connect("127.0.0.1:8080");
        // ... same as browser
    } catch (error) {
        console.error("Error:", error);
    }
}

main();
```

## API Reference

### Connection

#### `new Connection()`
Create a new connection instance.

#### `connect(addr: string): Promise<void>`
Connect to a server at the given address.

#### `send(streamId: number, data: Uint8Array): Promise<void>`
Send data on the specified stream.

#### `recv(): Promise<Array<[number, Uint8Array]>>`
Receive available packets. Returns array of [streamId, data] tuples.

#### `close(): Promise<void>`
Close the connection.

## Building

### Web Target
```bash
wasm-pack build --target web --out-dir pkg
```

### Node.js Target
```bash
wasm-pack build --target nodejs --out-dir pkg-node
```

### Bundler Target (Webpack, Rollup, etc.)
```bash
wasm-pack build --target bundler --out-dir pkg-bundler
```

## Testing

```bash
# Run tests in headless browser
wasm-pack test --headless --firefox
```

## Limitations

1. **No Raw UDP**: Browsers don't support raw UDP sockets
2. **Transport Required**: Need WebRTC/WebSocket transport layer
3. **Size**: WASM binary size (~few MB with full protocol)
4. **Performance**: Slightly slower than native due to WASM overhead

## Future Work

- [ ] WebRTC DataChannel transport
- [ ] WebSocket fallback transport
- [ ] WebTransport support
- [ ] Optimize WASM binary size
- [ ] Add comprehensive examples

## License

MIT
