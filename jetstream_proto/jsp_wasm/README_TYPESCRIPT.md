# JetStreamProto TypeScript/JavaScript SDK

WebAssembly bindings for JetStreamProto protocol.

## Installation

```bash
npm install jetstream-proto
# or
yarn add jetstream-proto
```

## Usage

### Basic Example

```typescript
import { Connection } from 'jetstream-proto';

async function main() {
  // Create connection
  const conn = new Connection();
  
  // Connect to server (WebSocket transport)
  await conn.connect("ws://localhost:8080");
  
  // Perform handshake
  const sessionId = await conn.handshake();
  console.log(`Connected with session ID: ${sessionId}`);
  
  // Open a reliable stream
  const streamId = await conn.open_stream(1, "reliable");
  
  // Send data
  const message = new TextEncoder().encode("Hello, JetStream!");
  await conn.send(streamId, message);
  
  // Receive data
  const packets = await conn.recv();
  for (const [sid, data] of packets) {
    const text = new TextDecoder().decode(data);
    console.log(`Stream ${sid}: ${text}`);
  }
  
  // Close connection
  await conn.close();
}

main().catch(console.error);
```

### React Example

```typescript
import { useEffect, useState } from 'react';
import { Connection } from 'jetstream-proto';

function ChatComponent() {
  const [conn, setConn] = useState<Connection | null>(null);
  const [messages, setMessages] = useState<string[]>([]);

  useEffect(() => {
    const connection = new Connection();
    
    connection.connect("ws://localhost:8080")
      .then(() => connection.handshake())
      .then(() => {
        setConn(connection);
        // Start receiving messages
        receiveLoop(connection);
      })
      .catch(console.error);

    return () => {
      connection.close();
    };
  }, []);

  async function receiveLoop(connection: Connection) {
    while (connection.connected) {
      const packets = await connection.recv();
      for (const [_, data] of packets) {
        const text = new TextDecoder().decode(data);
        setMessages(prev => [...prev, text]);
      }
    }
  }

  async function sendMessage(text: string) {
    if (!conn) return;
    
    const streamId = await conn.open_stream(1, "reliable");
    const data = new TextEncoder().encode(text);
    await conn.send(streamId, data);
  }

  return (
    <div>
      <div>
        {messages.map((msg, i) => <div key={i}>{msg}</div>)}
      </div>
      <button onClick={() => sendMessage("Hello!")}>
        Send Message
      </button>
    </div>
  );
}
```

### Node.js Example

```javascript
const { Connection } = require('jetstream-proto');

async function main() {
  const conn = new Connection();
  
  try {
    await conn.connect("ws://localhost:8080");
    await conn.handshake();
    
    const streamId = await conn.open_stream(1, "reliable");
    await conn.send(streamId, Buffer.from("Hello from Node.js!"));
    
    const packets = await conn.recv();
    console.log(`Received ${packets.length} packets`);
    
  } finally {
    await conn.close();
  }
}

main();
```

## API Reference

### Connection

#### Constructor

```typescript
new Connection()
```

Creates a new JetStream connection instance.

#### Methods

##### `connect(addr: string): Promise<void>`

Connect to a JetStream server.

- **addr**: Server address (WebSocket URL, e.g., `ws://localhost:8080`)

##### `handshake(): Promise<number>`

Perform handshake with the server. Returns the session ID.

##### `open_stream(priority: number, delivery_mode: DeliveryMode): Promise<number>`

Open a new stream.

- **priority**: Stream priority (0-255, higher = more important)
- **delivery_mode**: `"reliable"` or `"best_effort"`
- **Returns**: Stream ID

##### `send(stream_id: number, data: Uint8Array): Promise<void>`

Send data on a stream.

- **stream_id**: Stream ID to send on
- **data**: Data to send (Uint8Array or Buffer)

##### `recv(): Promise<Array<[number, Uint8Array]>>`

Receive data from all streams.

- **Returns**: Array of `[stream_id, data]` pairs

##### `close(): Promise<void>`

Close the connection gracefully.

#### Properties

##### `session_id: number` (read-only)

Current session ID.

##### `connected: boolean` (read-only)

Connection status.

## Delivery Modes

- **`reliable`**: Guaranteed delivery with retransmission
- **`best_effort`**: No delivery guarantees, lowest latency

## Building from Source

```bash
# Install wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build WASM package
cd jsp_wasm
wasm-pack build --target web

# For Node.js
wasm-pack build --target nodejs
```

## Browser Support

- Chrome/Edge 57+
- Firefox 52+
- Safari 11+

Requires WebAssembly support.

## License

MIT
