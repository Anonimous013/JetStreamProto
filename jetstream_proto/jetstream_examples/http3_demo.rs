//! HTTP/3 Server Example
//! 
//! Demonstrates HTTP/3 compatibility layer.

use jsp_transport::http3::{Http3Server, Request, Response, Frame};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info,jsp_transport=debug")
        .init();

    println!("🚀 JetStreamProto HTTP/3 Compatibility Demo");
    println!("============================================\n");

    // Create request handler
    let handler = Arc::new(|req: Request| {
        println!("📥 Request: {} {}", req.method, req.path);
        
        match req.path.as_str() {
            "/" => Response::ok().text("Welcome to JetStreamProto HTTP/3!"),
            "/api/status" => Response::ok().json(r#"{"status":"ok","protocol":"HTTP/3"}"#),
            "/api/echo" => {
                let body = String::from_utf8_lossy(&req.body);
                Response::ok()
                    .header("Content-Type", "text/plain")
                    .body(format!("Echo: {}", body).into_bytes())
            }
            _ => Response::not_found().text("Not Found"),
        }
    });

    println!("🌐 Creating HTTP/3 server...");
    let server = Http3Server::new(handler);
    println!("✅ Server created\n");

    // Simulate some requests
    println!("📨 Simulating HTTP/3 requests:\n");

    // Request 1: GET /
    println!("1️⃣  GET /");
    let req1_frame = Frame::headers(bytes::Bytes::from("GET / HTTP/3"));
    let resp1 = server.handle_frame(req1_frame)?;
    println!("   Response: {:?}\n", resp1.frame_type);

    // Request 2: GET /api/status
    println!("2️⃣  GET /api/status");
    let req2_frame = Frame::headers(bytes::Bytes::from("GET /api/status HTTP/3"));
    let resp2 = server.handle_frame(req2_frame)?;
    println!("   Response: {:?}\n", resp2.frame_type);

    // Request 3: POST /api/echo
    println!("3️⃣  POST /api/echo");
    let req3_frame = Frame::headers(bytes::Bytes::from("POST /api/echo HTTP/3"));
    let resp3 = server.handle_frame(req3_frame)?;
    println!("   Response: {:?}\n", resp3.frame_type);

    // Request 4: GET /notfound
    println!("4️⃣  GET /notfound");
    let req4_frame = Frame::headers(bytes::Bytes::from("GET /notfound HTTP/3"));
    let resp4 = server.handle_frame(req4_frame)?;
    println!("   Response: {:?}\n", resp4.frame_type);

    println!("💡 HTTP/3 Features:");
    println!("  ✅ Frame-based protocol");
    println!("  ✅ QUIC transport integration");
    println!("  ✅ Request/Response handling");
    println!("  ✅ Header compression (QPACK)");
    println!("  ✅ Multiplexing support");
    println!("  ✅ 0-RTT connection resumption");
    println!();

    println!("📊 Benefits over HTTP/2:");
    println!("  • Faster connection establishment");
    println!("  • Better loss recovery");
    println!("  • No head-of-line blocking");
    println!("  • Connection migration");
    println!();

    println!("✅ HTTP/3 demo completed!");

    Ok(())
}
