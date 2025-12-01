use cxx::{CxxVector, CxxString};
use std::sync::Mutex;
use tokio::runtime::Runtime;
use jsp_transport::connection::Connection as JspConnection;
use jsp_transport::config::ConnectionConfig;

#[cxx::bridge(namespace = "jsp_cpp")]
mod ffi {
    extern "Rust" {
        type RustConnection;
        fn new_rust_connection(addr: &CxxString) -> Box<RustConnection>;
        fn connect(&mut self);
        fn send(&mut self, stream_id: u32, data: &CxxVector<u8>);
        fn receive(&mut self, stream_id: &mut u32) -> Vec<u8>;
        fn close(&mut self);
    }
}

struct RustConnection {
    rt: Runtime,
    conn: Option<JspConnection>,
    addr: String,
}

fn new_rust_connection(addr: &CxxString) -> Box<RustConnection> {
    Box::new(RustConnection {
        rt: Runtime::new().unwrap(),
        conn: None,
        addr: addr.to_string(),
    })
}

impl RustConnection {
    fn connect(&mut self) {
        let addr = self.addr.clone();
        let conn = self.rt.block_on(async {
            let mut c = JspConnection::connect_with_config(&addr, ConnectionConfig::default()).await.unwrap();
            c.handshake().await.unwrap();
            c
        });
        self.conn = Some(conn);
    }

    fn send(&mut self, stream_id: u32, data: &CxxVector<u8>) {
        if let Some(conn) = &mut self.conn {
            let data_vec = data.as_slice().to_vec();
            self.rt.block_on(async {
                conn.send_on_stream(stream_id as u64, &data_vec).await.unwrap();
            });
        }
    }

    fn receive(&mut self, stream_id: &mut u32) -> Vec<u8> {
        if let Some(conn) = &mut self.conn {
            let (sid, data) = self.rt.block_on(async {
                conn.recv().await.unwrap().pop().unwrap_or((0, vec![]))
            });
            *stream_id = sid as u32;
            return data;
        }
        vec![]
    }

    fn close(&mut self) {
        // Cleanup
        self.conn = None;
    }
}
