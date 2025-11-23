"""
JetStreamProto Python SDK - Server Example

This example demonstrates how to use the JetStreamProto Python SDK
to create a server and handle incoming connections.
"""

import jetstream_proto

def main():
    print("JetStreamProto Python Server Example")
    print("=" * 50)
    
    # Create server
    print("\n1. Creating server...")
    server = jetstream_proto.Server()
    
    # Start listening
    listen_addr = "127.0.0.1:8080"
    print(f"2. Listening on {listen_addr}...")
    try:
        server.listen(listen_addr)
        print("   ✓ Server started successfully!")
    except Exception as e:
        print(f"   ✗ Server start failed: {e}")
        return
    
    print("\n3. Waiting for data...")
    print("   (Press Ctrl+C to stop)")
    
    try:
        while True:
            # Receive data
            packets = server.recv()
            if packets:
                for stream_id, data in packets:
                    print(f"\n   ✓ Received on stream {stream_id}: {data}")
                    
                    # Echo back
                    try:
                        server.send(stream_id, data)
                        print(f"   ✓ Echoed back: {data}")
                    except Exception as e:
                        print(f"   ✗ Echo failed: {e}")
    except KeyboardInterrupt:
        print("\n\n4. Shutting down...")
        print("   ✓ Server stopped")
    except Exception as e:
        print(f"\n   ✗ Error: {e}")
    
    print("\n" + "=" * 50)
    print("Example completed!")

if __name__ == "__main__":
    main()
