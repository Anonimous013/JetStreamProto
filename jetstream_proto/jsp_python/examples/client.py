"""
JetStreamProto Python SDK - Client Example

This example demonstrates how to use the JetStreamProto Python SDK
to create a client connection and send/receive data.
"""

import jetstream_proto
import time

def main():
    print("JetStreamProto Python Client Example")
    print("=" * 50)
    
    # Create connection
    print("\n1. Creating connection...")
    conn = jetstream_proto.Connection()
    
    # Connect to server
    server_addr = "127.0.0.1:8080"
    print(f"2. Connecting to {server_addr}...")
    try:
        conn.connect(server_addr)
        print("   ✓ Connected successfully!")
    except Exception as e:
        print(f"   ✗ Connection failed: {e}")
        return
    
    # Send data
    print("\n3. Sending data...")
    message = b"Hello from Python SDK!"
    try:
        conn.send(stream_id=1, data=message)
        print(f"   ✓ Sent: {message}")
    except Exception as e:
        print(f"   ✗ Send failed: {e}")
        conn.close()
        return
    
    # Receive data
    print("\n4. Receiving data...")
    time.sleep(0.1)  # Give server time to respond
    try:
        packets = conn.recv()
        if packets:
            for stream_id, data in packets:
                print(f"   ✓ Received on stream {stream_id}: {data}")
        else:
            print("   No data received")
    except Exception as e:
        print(f"   ✗ Receive failed: {e}")
    
    # Close connection
    print("\n5. Closing connection...")
    try:
        conn.close()
        print("   ✓ Connection closed")
    except Exception as e:
        print(f"   ✗ Close failed: {e}")
    
    print("\n" + "=" * 50)
    print("Example completed!")

if __name__ == "__main__":
    main()
