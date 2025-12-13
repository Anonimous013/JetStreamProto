package com.jetstream;

public class Connection implements AutoCloseable {
    static {
        System.loadLibrary("jsp_java");
    }

    private long nativeHandle;

    public Connection(String address) {
        this.nativeHandle = nativeConnect(address);
        if (this.nativeHandle == 0) {
            throw new RuntimeException("Failed to connect to " + address);
        }
    }

    public void send(int streamId, byte[] data) {
        if (!nativeSend(this.nativeHandle, streamId, data)) {
            throw new RuntimeException("Failed to send data");
        }
    }

    public byte[] read(int streamId) {
        return nativeRead(this.nativeHandle, streamId);
    }

    @Override
    public void close() {
        if (this.nativeHandle != 0) {
            nativeDisconnect(this.nativeHandle);
            this.nativeHandle = 0;
        }
    }

    // Native methods
    public interface DataListener {
        void onDataReceived(int streamId, byte[] data);
    }

    private DataListener listener;

    public void setDataListener(DataListener listener) {
        this.listener = listener;
        nativeSetDataListener(this);
    }

    // Called from native code
    private void onDataReceived(int streamId, byte[] data) {
        if (listener != null) {
            listener.onDataReceived(streamId, data);
        }
    }

    private native long nativeConnect(String address);
    private native void nativeDisconnect(long handle);
    private native boolean nativeSend(long handle, int streamId, byte[] data);
    private native void nativeSetDataListener(Connection connection);
}
