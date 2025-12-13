
import Foundation

public class JetStreamConnection {
    private var handle: UnsafeMutableRawPointer?
    private var dataCallback: ((UInt32, Data) -> Void)?

    public init() {}

    public func connect(address: String) -> Bool {
        let cAddr = (address as NSString).utf8String
        self.handle = jsp_connect(cAddr)
        
        if self.handle != nil {
            // Register global callback trampoline
            // Note: In a real app, we need a way to route back to 'self'.
            // For simplicity, we assume one active connection or use a global singleton map.
            jsp_set_data_listener(self.handle, globalDataCallback)
        }
        
        return self.handle != nil
    }

    public func onData(callback: @escaping (UInt32, Data) -> Void) {
        self.dataCallback = callback
    }
    
    // Internal handler called from global C callback (needs routing logic in real app)
    fileprivate func handleData(streamId: UInt32, data: Data) {
        DispatchQueue.main.async {
            self.dataCallback?(streamId, data)
        }
    }

    public func disconnect() {
        if let h = self.handle {
            jsp_disconnect(h)
            self.handle = nil
        }
    }

    public func send(streamId: UInt32, data: Data) -> Bool {
        guard let h = self.handle else { return false }
        
        return data.withUnsafeBytes { ptr in
            guard let baseAddress = ptr.baseAddress else { return false }
            let result = jsp_send(h, streamId, baseAddress.assumingMemoryBound(to: UInt8.self), data.count)
            return result == 0
        }
    }
    
    deinit {
        disconnect()
    }
}

// C-Bridge Definitions (would normally be in Bridging-Header.h)
@_silgen_name("jsp_connect")
func jsp_connect(_ addr: UnsafePointer<Int8>?) -> UnsafeMutableRawPointer?

@_silgen_name("jsp_disconnect")
func jsp_disconnect(_ handle: UnsafeMutableRawPointer?)

@_silgen_name("jsp_send")
func jsp_send(_ handle: UnsafeMutableRawPointer?, _ streamId: UInt32, _ data: UnsafePointer<UInt8>?, _ len: Int) -> Int32

@_silgen_name("jsp_set_data_listener")
func jsp_set_data_listener(_ handle: UnsafeMutableRawPointer?, _ callback: @convention(c) (UInt32, UnsafePointer<UInt8>?, Int) -> Void)

// Global C-callback implementation
func globalDataCallback(streamId: UInt32, dataPtr: UnsafePointer<UInt8>?, len: Int) {
    guard let ptr = dataPtr else { return }
    let data = Data(bytes: ptr, count: len)
    // NotificationCenter or Singleton dispatch would go here.
    // For demo, we just print.
    print("Received data on stream \(streamId): \(data.count) bytes")
}
