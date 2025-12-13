use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::sync::Arc;
use jsp_transport::connection::Connection;
use jsp_transport::config::ConnectionConfig;
use tokio::runtime::Runtime;
use bytes::Bytes;

lazy_static::lazy_static! {
    static ref RUNTIME: Runtime = Runtime::new().unwrap();
}

use std::sync::atomic::{AtomicBool, Ordering};

pub type DataCallback = extern "C" fn(stream_id: u32, data: *const u8, len: usize);

pub struct NativeConnection {
    conn: tokio::sync::Mutex<Connection>,
    running: Arc<AtomicBool>,
}

#[no_mangle]
pub unsafe extern "C" fn jsp_connect(addr: *const c_char) -> *mut c_void {
    if addr.is_null() {
        return std::ptr::null_mut();
    }
    
    let c_str = CStr::from_ptr(addr);
    let addr_str = match c_str.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return std::ptr::null_mut(),
    };

    let result = RUNTIME.block_on(async {
        let config = ConnectionConfig::default();
        Connection::connect_with_config(&addr_str, config).await
    });

    match result {
        Ok(conn) => {
            let running = Arc::new(AtomicBool::new(true));
            let native_conn = NativeConnection {
                conn: tokio::sync::Mutex::new(conn),
                running,
            };
            Box::into_raw(Box::new(native_conn)) as *mut c_void
        }
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn jsp_disconnect(handle: *mut c_void) {
    if handle.is_null() {
        return;
    }
    let native_conn = Box::from_raw(handle as *mut NativeConnection);
    native_conn.running.store(false, Ordering::SeqCst);
}

#[no_mangle]
pub unsafe extern "C" fn jsp_send(handle: *mut c_void, stream_id: u32, data: *const u8, len: usize) -> i32 {
    if handle.is_null() || data.is_null() {
        return -1;
    }

    let native_conn = &*(handle as *mut NativeConnection);
    let slice = std::slice::from_raw_parts(data, len);
    let bytes = Bytes::copy_from_slice(slice);

    let result = RUNTIME.block_on(async {
        let mut conn = native_conn.conn.lock().await;
        conn.send_on_stream(stream_id, &bytes).await
    });

    match result {
        Ok(_) => 0,
        Err(_) => -1,
    }
#[no_mangle]
pub unsafe extern "C" fn jsp_set_data_listener(handle: *mut c_void, callback: DataCallback) {
    if handle.is_null() {
        return;
    }
    let native_conn = &*(handle as *mut NativeConnection);
    let running = native_conn.running.clone();
    
    // Callback must be Send to be moved into task?
    // Function pointers are Copy and Send.
    // However, invoking it from another thread requires care if Swift side isn't thread safe.
    // We assume the C callback is thread-safe or dispatches to main queue.

    RUNTIME.spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        
        while running.load(Ordering::Relaxed) {
             interval.tick().await;

             // SIMULATION
             let stream_id = 1;
             let payload = b"Hello from Rust (iOS)!";
             
             callback(stream_id, payload.as_ptr(), payload.len());
        }
    });
}
