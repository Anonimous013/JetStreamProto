use jni::JNIEnv;
use jni::objects::{JClass, JString, JObject, JByteArray};
use jni::sys::{jint, jlong, jboolean, jbyteArray};
use jsp_transport::connection::Connection;
use jsp_transport::config::ConnectionConfig;
use std::sync::Arc;
use tokio::runtime::Runtime;
use bytes::Bytes;

// Global runtime for async operations
lazy_static::lazy_static! {
    static ref RUNTIME: Runtime = Runtime::new().unwrap();
}

static mut JAVA_VM: Option<jni::JavaVM> = None;
use std::sync::atomic::{AtomicBool, Ordering};

#[no_mangle]
pub extern "system" fn JNI_OnLoad(vm: jni::JavaVM, _reserved: *mut c_void) -> jint {
    unsafe { JAVA_VM = Some(vm) };
    jni::sys::JNI_VERSION_1_6
}

struct NativeConnection {
    conn: tokio::sync::Mutex<Connection>,
    running: Arc<AtomicBool>,
}

fn set_java_error(mut env: JNIEnv, error: String) {
    let _ = env.throw_new("java/lang/RuntimeException", error);
}

#[no_mangle]
pub extern "system" fn Java_com_jetstream_Connection_nativeConnect(
    env: JNIEnv,
    _class: JClass,
    addr: JString,
) -> jlong {
    let addr: String = match env.get_string(&addr) {
        Ok(s) => s.into(),
        Err(e) => {
            return 0;
        }
    };

    let result = RUNTIME.block_on(async {
        let config = ConnectionConfig::default(); 
        Connection::connect_with_config(&addr, config).await
    });

    match result {
        Ok(conn) => {
            let running = Arc::new(AtomicBool::new(true));
            let native_conn = NativeConnection {
                conn: tokio::sync::Mutex::new(conn),
                running,
            };
            Box::into_raw(Box::new(native_conn)) as jlong
        }
        Err(e) => {
            set_java_error(env, format!("Failed to connect: {}", e));
            0
        }
    }
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_jetstream_Connection_nativeDisconnect(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
) {
    if handle == 0 {
        return;
    }
    let native_conn = Box::from_raw(handle as *mut NativeConnection);
    native_conn.running.store(false, Ordering::SeqCst);
    // native_conn dropped here, connection closed
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_jetstream_Connection_nativeSend(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    stream_id: jint,
    data: jbyteArray,
) -> jboolean {
    if handle == 0 {
        return 0;
    }
    let native_conn = &*(handle as *mut NativeConnection);
    
    // Correct way to get bytes from jbyteArray
    let data_vec = match env.convert_byte_array(&JByteArray::from_raw(data)) {
        Ok(d) => d,
        Err(_) => return 0,
    };
    let bytes = Bytes::from(data_vec);

    let result = RUNTIME.block_on(async {
        let mut conn = native_conn.conn.lock().await;
        conn.send_on_stream(stream_id as u32, &bytes).await
    });

    match result {
        Ok(_) => 1,
        Err(e) => {
            set_java_error(env, format!("Send failed: {}", e));
            0
        }
    }
}

#[no_mangle]
pub unsafe extern "system" fn Java_com_jetstream_Connection_nativeSetDataListener(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
    listener: JObject,
) {
    if handle == 0 {
        return;
    }
    let native_conn = &*(handle as *mut NativeConnection);
    
    // Create a global reference to the listener object (Connection.java instance)
    let listener_ref = env.new_global_ref(listener).unwrap();
    let running = native_conn.running.clone();
    
    // Spawn background task to "receive" data and call callback
    RUNTIME.spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        
        while running.load(Ordering::Relaxed) {
            interval.tick().await;
            
            // SIMULATION: In real code, await connection.recv()
            // Here we simulate data arrival
            let stream_id = 1;
            let payload = b"Hello from Rust!";
            
            // Attach thread to JVM
            let vm = match unsafe { JAVA_VM.as_ref() } {
                Some(v) => v,
                None => break,
            };
            
            match vm.attach_current_thread() {
                Ok(mut env) => {
                    let data_arr = env.byte_array_from_slice(payload).unwrap();
                    let _ = env.call_method(
                        listener_ref.as_obj(),
                        "onDataReceived",
                        "(I[B)V",
                        &[stream_id.into(), data_arr.into()]
                    );
                }
                Err(e) => {
                    tracing::error!("Failed to attach JNI thread: {}", e);
                }
            }
        }
    });
}
