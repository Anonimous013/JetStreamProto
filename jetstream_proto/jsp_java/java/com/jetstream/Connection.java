package com.jetstream;

public class Connection implements AutoCloseable {
    static {
        System.loadLibrary("jsp_java");
    }

    private long handle;

    public Connection(String addr) {
        this.handle = nativeConnect(addr);
        if (this.handle == -1) {
            throw new RuntimeException("Failed to connect");
        }
    }

    public void handshake() {
        if (nativeHandshake(this.handle) != 0) {
            throw new RuntimeException("Handshake failed");
        }
    }

    public void send(int streamId, byte[] data) {
        if (nativeSend(this.handle, streamId, data) != 0) {
            throw new RuntimeException("Send failed");
        }
    }

    @Override
    public void close() {
        if (this.handle != 0) {
            nativeClose(this.handle);
            this.handle = 0;
        }
    }

    private native long nativeConnect(String addr);
    private native int nativeHandshake(long handle);
    private native int nativeSend(long handle, int streamId, byte[] data);
    private native void nativeClose(long handle);
}
