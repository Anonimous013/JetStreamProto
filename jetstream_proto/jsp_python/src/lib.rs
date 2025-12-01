use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Python wrapper for JetStream Connection
#[pyclass]
struct Connection {
    inner: Option<Arc<tokio::sync::Mutex<jsp_transport::connection::Connection>>>,
    runtime: Arc<Runtime>,
}

#[pymethods]
impl Connection {
    #[new]
    fn new() -> PyResult<Self> {
        let runtime = Arc::new(
            Runtime::new()
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?
        );
        
        Ok(Self {
            inner: None,
            runtime,
        })
    }

    /// Connect to a server
    fn connect(&mut self, addr: String) -> PyResult<()> {
        let runtime = self.runtime.clone();
        
        let conn = runtime.block_on(async {
            jsp_transport::connection::Connection::connect_with_config(
                &addr,
                jsp_transport::config::ConnectionConfig::default()
            ).await
        }).map_err(|e| PyRuntimeError::new_err(format!("Connection failed: {}", e)))?;
        
        self.inner = Some(Arc::new(tokio::sync::Mutex::new(conn)));
        Ok(())
    }

    /// Perform handshake
    fn handshake(&self) -> PyResult<()> {
        let inner = self.inner.as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("Not connected"))?;
        
        let inner_clone = inner.clone();
        let runtime = self.runtime.clone();
        
        runtime.block_on(async move {
            let mut conn = inner_clone.lock().await;
            conn.handshake().await
        }).map_err(|e| PyRuntimeError::new_err(format!("Handshake failed: {}", e)))?;
        
        Ok(())
    }

    /// Get session ID
    fn session_id(&self) -> PyResult<u64> {
        let inner = self.inner.as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("Not connected"))?;
        
        let inner_clone = inner.clone();
        let runtime = self.runtime.clone();
        
        let session_id = runtime.block_on(async move {
            let conn = inner_clone.lock().await;
            conn.session_id()
        });
        
        Ok(session_id)
    }

    /// Open a new stream
    fn open_stream(&self, priority: u8, delivery_mode: String) -> PyResult<u32> {
        let inner = self.inner.as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("Not connected"))?;
        
        let mode = match delivery_mode.as_str() {
            "reliable" => jsp_core::types::delivery::DeliveryMode::Reliable,
            "best_effort" => jsp_core::types::delivery::DeliveryMode::BestEffort,
            _ => return Err(PyRuntimeError::new_err("Invalid delivery mode. Use 'reliable' or 'best_effort'")),
        };
        
        let inner_clone = inner.clone();
        let runtime = self.runtime.clone();
        
        let stream_id = runtime.block_on(async move {
            let mut conn = inner_clone.lock().await;
            conn.open_stream(priority, mode)
        }).map_err(|e| PyRuntimeError::new_err(format!("Open stream failed: {}", e)))?;
        
        Ok(stream_id)
    }

    /// Send data on a stream
    fn send(&self, stream_id: u32, data: Vec<u8>) -> PyResult<()> {
        let inner = self.inner.as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("Not connected"))?;
        
        let inner_clone = inner.clone();
        let runtime = self.runtime.clone();
        
        runtime.block_on(async move {
            let mut conn = inner_clone.lock().await;
            conn.send_on_stream(stream_id, &data).await
        }).map_err(|e| PyRuntimeError::new_err(format!("Send failed: {}", e)))?;
        
        Ok(())
    }

    /// Receive data
    fn recv(&self) -> PyResult<Vec<(u32, Vec<u8>)>> {
        let inner = self.inner.as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("Not connected"))?;
        
        let inner_clone = inner.clone();
        let runtime = self.runtime.clone();
        
        let packets = runtime.block_on(async move {
            let mut conn = inner_clone.lock().await;
            conn.recv().await
        }).map_err(|e| PyRuntimeError::new_err(format!("Recv failed: {}", e)))?;
        
        Ok(packets.into_iter()
            .map(|(stream_id, data)| (stream_id, data.to_vec()))
            .collect())
    }

    /// Close connection
    fn close(&mut self) -> PyResult<()> {
        if let Some(inner) = self.inner.take() {
            let runtime = self.runtime.clone();
            runtime.block_on(async move {
                let mut conn = inner.lock().await;
                conn.close(
                    jsp_core::types::control::CloseReason::Normal,
                    Some("Connection closed by Python SDK".to_string())
                ).await
            }).map_err(|e| PyRuntimeError::new_err(format!("Close failed: {}", e)))?;
        }
        Ok(())
    }
}

/// Python wrapper for JetStream Server
#[pyclass]
struct Server {
    inner: Option<Arc<tokio::sync::Mutex<jsp_transport::connection::Connection>>>,
    runtime: Arc<Runtime>,
}

#[pymethods]
impl Server {
    #[new]
    fn new() -> PyResult<Self> {
        let runtime = Arc::new(
            Runtime::new()
                .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?
        );
        
        Ok(Self {
            inner: None,
            runtime,
        })
    }

    /// Start listening on an address
    fn listen(&mut self, addr: String) -> PyResult<()> {
        let runtime = self.runtime.clone();
        
        let conn = runtime.block_on(async {
            jsp_transport::connection::Connection::listen_with_config(
                &addr,
                jsp_transport::config::ConnectionConfig::default()
            ).await
        }).map_err(|e| PyRuntimeError::new_err(format!("Listen failed: {}", e)))?;
        
        self.inner = Some(Arc::new(tokio::sync::Mutex::new(conn)));
        Ok(())
    }

    /// Receive data
    fn recv(&self) -> PyResult<Vec<(u32, Vec<u8>)>> {
        let inner = self.inner.as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("Not listening"))?;
        
        let inner_clone = inner.clone();
        let runtime = self.runtime.clone();
        
        let packets = runtime.block_on(async move {
            let mut conn = inner_clone.lock().await;
            conn.recv().await
        }).map_err(|e| PyRuntimeError::new_err(format!("Recv failed: {}", e)))?;
        
        Ok(packets.into_iter()
            .map(|(stream_id, data)| (stream_id, data.to_vec()))
            .collect())
    }

    /// Send data on a stream
    fn send(&self, stream_id: u32, data: Vec<u8>) -> PyResult<()> {
        let inner = self.inner.as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("Not listening"))?;
        
        let inner_clone = inner.clone();
        let runtime = self.runtime.clone();
        
        runtime.block_on(async move {
            let mut conn = inner_clone.lock().await;
            conn.send_on_stream(stream_id, &data).await
        }).map_err(|e| PyRuntimeError::new_err(format!("Send failed: {}", e)))?;
        
        Ok(())
    }
}

/// JetStreamProto Python module
#[pymodule]
fn jetstream_proto(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Connection>()?;
    m.add_class::<Server>()?;
    Ok(())
}
