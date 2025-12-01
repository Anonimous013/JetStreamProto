# JetStreamProto Go SDK

Go language bindings for JetStreamProto protocol using cgo.

## Installation

```bash
# First, build the C library
cd ../jsp_c
cargo build --release

# Then, use the Go package
go get github.com/yourusername/jetstream-proto
```

## Usage

### Basic Example

```go
package main

import (
	"fmt"
	"log"
	
	jetstream "github.com/yourusername/jetstream-proto"
)

func main() {
	// Create connection
	conn, err := jetstream.NewConnection()
	if err != nil {
		log.Fatal(err)
	}
	defer conn.Free()

	// Connect to server
	if err := conn.Connect("127.0.0.1:8080"); err != nil {
		log.Fatal(err)
	}

	// Perform handshake
	if err := conn.Handshake(); err != nil {
		log.Fatal(err)
	}

	// Get session ID
	sessionID := conn.SessionID()
	fmt.Printf("Connected with session ID: %d\n", sessionID)

	// Open a reliable stream
	streamID, err := conn.OpenStream(1, jetstream.Reliable)
	if err != nil {
		log.Fatal(err)
	}

	// Send data
	message := []byte("Hello, JetStream!")
	if err := conn.Send(streamID, message); err != nil {
		log.Fatal(err)
	}

	// Close connection
	if err := conn.Close(); err != nil {
		log.Fatal(err)
	}
}
```

### HTTP Server Example

```go
package main

import (
	"fmt"
	"log"
	"net/http"
	
	jetstream "github.com/yourusername/jetstream-proto"
)

var conn *jetstream.Connection

func init() {
	var err error
	conn, err = jetstream.NewConnection()
	if err != nil {
		log.Fatal(err)
	}

	if err := conn.Connect("127.0.0.1:8080"); err != nil {
		log.Fatal(err)
	}

	if err := conn.Handshake(); err != nil {
		log.Fatal(err)
	}
}

func sendHandler(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	// Read message from request body
	message := make([]byte, r.ContentLength)
	r.Body.Read(message)

	// Open stream and send
	streamID, err := conn.OpenStream(1, jetstream.Reliable)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	if err := conn.Send(streamID, message); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	fmt.Fprintf(w, "Sent %d bytes on stream %d\n", len(message), streamID)
}

func main() {
	defer conn.Free()

	http.HandleFunc("/send", sendHandler)
	log.Println("Server starting on :8081")
	log.Fatal(http.ListenAndServe(":8081", nil))
}
```

### Concurrent Usage

```go
package main

import (
	"fmt"
	"sync"
	
	jetstream "github.com/yourusername/jetstream-proto"
)

func main() {
	conn, _ := jetstream.NewConnection()
	defer conn.Free()

	conn.Connect("127.0.0.1:8080")
	conn.Handshake()

	var wg sync.WaitGroup
	
	// Send messages concurrently
	for i := 0; i < 10; i++ {
		wg.Add(1)
		go func(id int) {
			defer wg.Done()
			
			streamID, _ := conn.OpenStream(1, jetstream.Reliable)
			message := []byte(fmt.Sprintf("Message %d", id))
			conn.Send(streamID, message)
		}(i)
	}

	wg.Wait()
	conn.Close()
}
```

## API Reference

### Types

#### `Connection`
Represents a JetStream connection.

#### `DeliveryMode`
Delivery guarantee modes:
- `Reliable` - Guaranteed delivery with retransmission
- `BestEffort` - No delivery guarantees, lowest latency
- `PartiallyReliable` - Time-limited retries

### Functions

#### `NewConnection() (*Connection, error)`
Creates a new JetStream connection.

#### `(*Connection) Connect(addr string) error`
Connects to a JetStream server.

**Parameters:**
- `addr` - Server address (e.g., "127.0.0.1:8080")

#### `(*Connection) Handshake() error`
Performs handshake with the server.

#### `(*Connection) SessionID() uint64`
Returns the session ID.

#### `(*Connection) OpenStream(priority uint8, mode DeliveryMode) (uint32, error)`
Opens a new stream.

**Parameters:**
- `priority` - Stream priority (0-255)
- `mode` - Delivery mode

**Returns:** Stream ID

#### `(*Connection) Send(streamID uint32, data []byte) error`
Sends data on a stream.

**Parameters:**
- `streamID` - Stream ID
- `data` - Data to send

#### `(*Connection) Close() error`
Closes the connection gracefully.

#### `(*Connection) Free()`
Releases connection resources. Must be called when done.

## Building

### Prerequisites
- Go 1.21 or higher
- Rust toolchain (for building C library)
- C compiler (gcc/clang)

### Build Steps

```bash
# 1. Build the C library
cd ../jsp_c
cargo build --release

# 2. Build Go package
cd ../jsp_go
go build

# 3. Run tests
go test -v
```

## Thread Safety

The Go bindings are thread-safe. The same connection can be used from multiple goroutines.

## Memory Management

Always call `Free()` when done with a connection:

```go
conn, _ := jetstream.NewConnection()
defer conn.Free()  // Ensures cleanup
```

## License

MIT
