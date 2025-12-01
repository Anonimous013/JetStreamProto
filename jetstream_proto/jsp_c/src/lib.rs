use std::ffi::CStr;
use std::os::raw::{c_char, c_uint, c_ulonglong};
use std::ptr;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Opaque connection handle
pub struct JspConnection {
    inner: Arc<tokio::sync::Mutex<Option<jsp_transport::connection::Connection>>>,
    runtime: Arc<Runtime>,
}

/// Error codes
#[repr(C)]
pub enum JspError {
    Success = 0,
    NullPointer = 1,
    ConnectionFailed = 2,
    HandshakeFailed = 3,
    SendFailed = 4,
    ReceiveFailed = 5,
    InvalidMode = 6,
    NotConnected = 7,
}

/// Delivery modes
#[repr(C)]
pub enum JspDeliveryMode {
    Reliable = 0,
    BestEffort = 1,
    PartiallyReliable = 2,
}

/// Create a new connection
/// Returns NULL on failure
#[no_mangle]
pub extern "C" fn jsp_connection_new() -> *mut JspConnection {
    let runtime = match Runtime::new() {
        Ok(rt) => Arc::new(rt),
        Err(_) => return ptr::null_mut(),
    };

    let conn = Box::new(JspConnection {
        inner: Arc::new(tokio::sync::Mutex::new(None)),
        runtime,
    });

    Box::into_raw(conn)
}

/// Connect to a server
/// @param conn - Connection handle
/// @param addr - Server address (null-terminated string)
/// @return Error code
#[no_mangle]
pub extern "C" fn jsp_connection_connect(
    conn: *mut JspConnection,
    addr: *const c_char,
) -> JspError {
    if conn.is_null() || addr.is_null() {
        return JspError::NullPointer;
    }

    let conn = unsafe { &mut *conn };
    let addr_str = unsafe {
        match CStr::from_ptr(addr).to_str() {
            Ok(s) => s,
            Err(_) => return JspError::ConnectionFailed,
        }
    };

    let runtime = conn.runtime.clone();
    let result = runtime.block_on(async {
        jsp_transport::connection::Connection::connect_with_config(
            addr_str,
            jsp_transport::config::ConnectionConfig::default(),
        )
        .await
    });

    match result {
        Ok(connection) => {
            *conn.inner.blocking_lock() = Some(connection);
            JspError::Success
        }
        Err(_) => JspError::ConnectionFailed,
    }
}

/// Perform handshake
/// @param conn - Connection handle
/// @return Error code
#[no_mangle]
pub extern "C" fn jsp_connection_handshake(conn: *mut JspConnection) -> JspError {
    if conn.is_null() {
        return JspError::NullPointer;
    }

    let conn = unsafe { &*conn };
    let inner = conn.inner.clone();
    let runtime = conn.runtime.clone();

    let result = runtime.block_on(async {
        let mut connection = inner.lock().await;
        match connection.as_mut() {
            Some(conn) => conn.handshake().await,
            None => Err(anyhow::anyhow!("Not connected")),
        }
    });

    match result {
        Ok(_) => JspError::Success,
        Err(_) => JspError::HandshakeFailed,
    }
}

/// Get session ID
/// @param conn - Connection handle
/// @return Session ID (0 if not connected)
#[no_mangle]
pub extern "C" fn jsp_connection_session_id(conn: *const JspConnection) -> c_ulonglong {
    if conn.is_null() {
        return 0;
    }

    let conn = unsafe { &*conn };
    let inner = conn.inner.clone();
    let runtime = conn.runtime.clone();

    runtime.block_on(async {
        let connection = inner.lock().await;
        connection.as_ref().map(|c| c.session_id()).unwrap_or(0)
    })
}

/// Open a new stream
/// @param conn - Connection handle
/// @param priority - Stream priority (0-255)
/// @param mode - Delivery mode
/// @param stream_id_out - Output parameter for stream ID
/// @return Error code
#[no_mangle]
pub extern "C" fn jsp_connection_open_stream(
    conn: *mut JspConnection,
    priority: c_uint,
    mode: JspDeliveryMode,
    stream_id_out: *mut c_uint,
) -> JspError {
    if conn.is_null() || stream_id_out.is_null() {
        return JspError::NullPointer;
    }

    let delivery_mode = match mode {
        JspDeliveryMode::Reliable => jsp_core::types::delivery::DeliveryMode::Reliable,
        JspDeliveryMode::BestEffort => jsp_core::types::delivery::DeliveryMode::BestEffort,
        JspDeliveryMode::PartiallyReliable => {
            jsp_core::types::delivery::DeliveryMode::PartiallyReliable { ttl_ms: 5000 }
        }
    };

    let conn = unsafe { &*conn };
    let inner = conn.inner.clone();
    let runtime = conn.runtime.clone();

    let result = runtime.block_on(async {
        let mut connection = inner.lock().await;
        match connection.as_mut() {
            Some(conn) => conn.open_stream(priority as u8, delivery_mode),
            None => Err(anyhow::anyhow!("Not connected")),
        }
    });

    match result {
        Ok(stream_id) => {
            unsafe { *stream_id_out = stream_id };
            JspError::Success
        }
        Err(_) => JspError::NotConnected,
    }
}

/// Send data on a stream
/// @param conn - Connection handle
/// @param stream_id - Stream ID
/// @param data - Data buffer
/// @param len - Data length
/// @return Error code
#[no_mangle]
pub extern "C" fn jsp_connection_send(
    conn: *mut JspConnection,
    stream_id: c_uint,
    data: *const u8,
    len: usize,
) -> JspError {
    if conn.is_null() || data.is_null() {
        return JspError::NullPointer;
    }

    let conn = unsafe { &*conn };
    let data_slice = unsafe { std::slice::from_raw_parts(data, len) };
    let inner = conn.inner.clone();
    let runtime = conn.runtime.clone();

    let result = runtime.block_on(async {
        let mut connection = inner.lock().await;
        match connection.as_mut() {
            Some(conn) => conn.send_on_stream(stream_id, data_slice).await,
            None => Err(anyhow::anyhow!("Not connected")),
        }
    });

    match result {
        Ok(_) => JspError::Success,
        Err(_) => JspError::SendFailed,
    }
}

/// Close connection
/// @param conn - Connection handle
/// @return Error code
#[no_mangle]
pub extern "C" fn jsp_connection_close(conn: *mut JspConnection) -> JspError {
    if conn.is_null() {
        return JspError::NullPointer;
    }

    let conn = unsafe { &*conn };
    let inner = conn.inner.clone();
    let runtime = conn.runtime.clone();

    let result = runtime.block_on(async {
        let mut connection = inner.lock().await;
        match connection.as_mut() {
            Some(conn) => {
                conn.close(
                    jsp_core::types::control::CloseReason::Normal,
                    Some("Closed by C API".to_string()),
                )
                .await
            }
            None => Ok(()),
        }
    });

    match result {
        Ok(_) => JspError::Success,
        Err(_) => JspError::SendFailed,
    }
}

/// Free connection
/// @param conn - Connection handle
#[no_mangle]
pub extern "C" fn jsp_connection_free(conn: *mut JspConnection) {
    if !conn.is_null() {
        unsafe {
            let _ = Box::from_raw(conn);
        }
    }
}

/// Get error message for error code
/// @param error - Error code
/// @return Error message (static string)
#[no_mangle]
pub extern "C" fn jsp_error_message(error: JspError) -> *const c_char {
    let msg = match error {
        JspError::Success => "Success\0",
        JspError::NullPointer => "Null pointer\0",
        JspError::ConnectionFailed => "Connection failed\0",
        JspError::HandshakeFailed => "Handshake failed\0",
        JspError::SendFailed => "Send failed\0",
        JspError::ReceiveFailed => "Receive failed\0",
        JspError::InvalidMode => "Invalid delivery mode\0",
        JspError::NotConnected => "Not connected\0",
    };

    msg.as_ptr() as *const c_char
}
