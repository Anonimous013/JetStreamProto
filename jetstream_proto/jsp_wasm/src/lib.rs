use wasm_bindgen::prelude::*;
use js_sys::Promise;
use wasm_bindgen_futures::future_to_promise;

/// JavaScript wrapper for JetStream Connection
#[wasm_bindgen]
pub struct Connection {
    // For WASM, we'll need a different approach since we can't use tokio runtime directly
    // This is a simplified version - full implementation would need wasm-compatible async
}

#[wasm_bindgen]
impl Connection {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        // Initialize console panic hook for better error messages
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();
        
        Self {}
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
