# JetStreamProto C API

C language bindings for JetStreamProto protocol.

## Building

```bash
cd jsp_c
cargo build --release

# The library will be at:
# target/release/libjsp_c.so (Linux)
# target/release/libjsp_c.dylib (macOS)
# target/release/jsp_c.dll (Windows)

# Header file generated at:
# jetstream_proto.h
```

## Usage

### Basic Example

```c
#include "jetstream_proto.h"
#include <stdio.h>
#include <string.h>

int main() {
    // Create connection
    JspConnection* conn = jsp_connection_new();
    if (!conn) {
        fprintf(stderr, "Failed to create connection\n");
        return 1;
    }

    // Connect to server
    JspError err = jsp_connection_connect(conn, "127.0.0.1:8080");
    if (err != JSP_ERROR_SUCCESS) {
        fprintf(stderr, "Connection failed: %s\n", jsp_error_message(err));
        jsp_connection_free(conn);
        return 1;
    }

    // Perform handshake
    err = jsp_connection_handshake(conn);
    if (err != JSP_ERROR_SUCCESS) {
        fprintf(stderr, "Handshake failed: %s\n", jsp_error_message(err));
        jsp_connection_free(conn);
        return 1;
    }

    // Get session ID
    unsigned long long session_id = jsp_connection_session_id(conn);
    printf("Connected with session ID: %llu\n", session_id);

    // Open a reliable stream
    unsigned int stream_id;
    err = jsp_connection_open_stream(conn, 1, JSP_DELIVERY_MODE_RELIABLE, &stream_id);
    if (err != JSP_ERROR_SUCCESS) {
        fprintf(stderr, "Failed to open stream: %s\n", jsp_error_message(err));
        jsp_connection_free(conn);
        return 1;
    }

    // Send data
    const char* message = "Hello, JetStream!";
    err = jsp_connection_send(conn, stream_id, (const uint8_t*)message, strlen(message));
    if (err != JSP_ERROR_SUCCESS) {
        fprintf(stderr, "Send failed: %s\n", jsp_error_message(err));
    }

    // Close connection
    jsp_connection_close(conn);
    jsp_connection_free(conn);

    return 0;
}
```

### Compilation

```bash
# Linux
gcc -o example example.c -L./target/release -ljsp_c -lpthread -ldl -lm

# macOS
gcc -o example example.c -L./target/release -ljsp_c -lpthread

# Windows (MSVC)
cl example.c /I. /link jsp_c.lib ws2_32.lib userenv.lib
```

## API Reference

### Types

#### `JspConnection`
Opaque connection handle.

#### `JspError`
Error codes:
- `JSP_ERROR_SUCCESS` (0) - Success
- `JSP_ERROR_NULL_POINTER` (1) - Null pointer
- `JSP_ERROR_CONNECTION_FAILED` (2) - Connection failed
- `JSP_ERROR_HANDSHAKE_FAILED` (3) - Handshake failed
- `JSP_ERROR_SEND_FAILED` (4) - Send failed
- `JSP_ERROR_RECEIVE_FAILED` (5) - Receive failed
- `JSP_ERROR_INVALID_MODE` (6) - Invalid delivery mode
- `JSP_ERROR_NOT_CONNECTED` (7) - Not connected

#### `JspDeliveryMode`
Delivery modes:
- `JSP_DELIVERY_MODE_RELIABLE` (0) - Guaranteed delivery
- `JSP_DELIVERY_MODE_BEST_EFFORT` (1) - No guarantees
- `JSP_DELIVERY_MODE_PARTIALLY_RELIABLE` (2) - Time-limited retries

### Functions

#### `jsp_connection_new()`
```c
JspConnection* jsp_connection_new(void);
```
Create a new connection. Returns NULL on failure.

#### `jsp_connection_connect()`
```c
JspError jsp_connection_connect(JspConnection* conn, const char* addr);
```
Connect to a server.

**Parameters:**
- `conn` - Connection handle
- `addr` - Server address (null-terminated string)

**Returns:** Error code

#### `jsp_connection_handshake()`
```c
JspError jsp_connection_handshake(JspConnection* conn);
```
Perform handshake with server.

#### `jsp_connection_session_id()`
```c
unsigned long long jsp_connection_session_id(const JspConnection* conn);
```
Get session ID. Returns 0 if not connected.

#### `jsp_connection_open_stream()`
```c
JspError jsp_connection_open_stream(
    JspConnection* conn,
    unsigned int priority,
    JspDeliveryMode mode,
    unsigned int* stream_id_out
);
```
Open a new stream.

**Parameters:**
- `conn` - Connection handle
- `priority` - Stream priority (0-255)
- `mode` - Delivery mode
- `stream_id_out` - Output parameter for stream ID

#### `jsp_connection_send()`
```c
JspError jsp_connection_send(
    JspConnection* conn,
    unsigned int stream_id,
    const uint8_t* data,
    size_t len
);
```
Send data on a stream.

#### `jsp_connection_close()`
```c
JspError jsp_connection_close(JspConnection* conn);
```
Close connection gracefully.

#### `jsp_connection_free()`
```c
void jsp_connection_free(JspConnection* conn);
```
Free connection resources.

#### `jsp_error_message()`
```c
const char* jsp_error_message(JspError error);
```
Get error message for error code.

## Thread Safety

All functions are thread-safe. The connection handle can be used from multiple threads.

## Memory Management

- Call `jsp_connection_free()` to release connection resources
- Strings returned by `jsp_error_message()` are static and don't need to be freed
- Data passed to `jsp_connection_send()` is copied internally

## License

MIT
