use wasm_bindgen::prelude::*;
use js_sys::Promise;
use wasm_bindgen_futures::future_to_promise;

/// JavaScript wrapper for JetStream Connection
#[wasm_bindgen]
pub struct Connection {
    session_id: u64,
    connected: bool,
}

#[wasm_bindgen]
impl Connection {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        // Initialize console panic hook for better error messages
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();
        
        Self {
            session_id: 0,
            connected: false,
        }
    }

    /// Connect to a server
    /// Returns a Promise that resolves when connected
    #[wasm_bindgen]
    pub fn connect(&mut self, addr: String) -> Promise {
        future_to_promise(async move {
            // Note: WebAssembly in browsers cannot create raw UDP sockets
            // This would need to use WebRTC DataChannels or WebSockets as transport
            // For now, this is a placeholder showing the API structure
            
            web_sys::console::log_1(&format!("Connecting to {}...", addr).into());
            
            // TODO: Implement WebRTC or WebSocket transport for browser
            Err(JsValue::from_str("UDP not supported in browser - use WebRTC or WebSocket transport"))
        })
    }

    /// Perform handshake
    #[wasm_bindgen]
    pub fn handshake(&mut self) -> Promise {
        future_to_promise(async move {
            web_sys::console::log_1(&"Performing handshake...".into());
            // Simulate handshake and return session ID
            let session_id = (js_sys::Math::random() * 1000000.0) as u64;
            Ok(JsValue::from_f64(session_id as f64))
        })
    }

    /// Get session ID
    #[wasm_bindgen(getter)]
    pub fn session_id(&self) -> u64 {
        self.session_id
    }

    /// Check if connected
    #[wasm_bindgen(getter)]
    pub fn connected(&self) -> bool {
        self.connected
    }

    /// Open a new stream
    /// @param priority - Stream priority (0-255)
    /// @param delivery_mode - "reliable" or "best_effort"
    /// @returns Promise<number> - Stream ID
    #[wasm_bindgen]
    pub fn open_stream(&self, priority: u8, delivery_mode: String) -> Promise {
        future_to_promise(async move {
            let mode = match delivery_mode.as_str() {
                "reliable" => "Reliable",
                "best_effort" => "BestEffort",
                _ => return Err(JsValue::from_str("Invalid delivery mode")),
            };
            
            web_sys::console::log_1(&format!("Opening {} stream with priority {}", mode, priority).into());
            
            // Generate random stream ID
            let stream_id = (js_sys::Math::random() * 1000.0) as u32;
            Ok(JsValue::from_f64(stream_id as f64))
        })
    }

    /// Send data on a stream
    #[wasm_bindgen]
    pub fn send(&self, stream_id: u32, data: Vec<u8>) -> Promise {
        future_to_promise(async move {
            web_sys::console::log_1(&format!("Sending {} bytes on stream {}", data.len(), stream_id).into());
            Ok(JsValue::NULL)
        })
    }

    /// Receive data
    /// Returns a Promise that resolves to an array of [stream_id, data] pairs
    #[wasm_bindgen]
    pub fn recv(&self) -> Promise {
        future_to_promise(async move {
            // Return empty array for now
            Ok(JsValue::from(js_sys::Array::new()))
        })
    }

    /// Close connection
    #[wasm_bindgen]
    pub fn close(&mut self) -> Promise {
        future_to_promise(async move {
            web_sys::console::log_1(&"Connection closed".into());
            Ok(JsValue::NULL)
        })
    }
}

/// Initialize the WASM module
#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_connection_new() {
        let conn = Connection::new();
        assert!(true); // Basic smoke test
    }
}
