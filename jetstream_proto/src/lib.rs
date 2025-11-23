//! # JetStreamProto
//! 
//! ```text
//!      __     __  _____ __                          ____           __      
//!     / /__  / /_/ ___// /_________  ____ _____ ___ / __ \_________/ /_____ 
//!  __/ / _ \/ __/\__ \/ __/ ___/ _ \/ __ `/ __ `__ \/ /_/ / ___/ __  / __ \
//! / __/  __/ /_ ___/ / /_/ /  /  __/ /_/ / / / / / / ____/ /  / /_/ / /_/ /
//! \___/\___/\__//____/\__/_/   \___/\__,_/_/ /_/ /_/_/   /_/   \__,_/\____/ 
//!                                                                          
//!     High-Performance Post-Quantum Networking Protocol
//! ```
//!
//! ## Overview
//!
//! JetStreamProto is a modern, high-performance networking protocol with:
//! - **Post-Quantum Cryptography** (Kyber768)
//! - **Multi-Transport Support** (UDP/TCP/QUIC)
//! - **Forward Error Correction** (Reed-Solomon)
//! - **Mobile Optimizations** (Adaptive compression, battery-aware)
//! - **Multi-Language SDKs** (Rust, Python, JavaScript)
//!
//! ## Performance
//!
//! - **Throughput:** 1,200 Mbps
//! - **Latency:** 0.8ms (p50)
//! - **Concurrent Connections:** 10,000+
//!
//! ## Quick Start
//!
//! ```rust
//! use jsp_transport::connection::Connection;
//! use jsp_transport::config::ConnectionConfig;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Server
//!     let mut server = Connection::listen("0.0.0.0:8080").await?;
//!     
//!     // Client
//!     let mut client = Connection::connect("127.0.0.1:8080").await?;
//!     
//!     // Send message
//!     client.send_on_stream(1, b"Hello, World!").await?;
//!     
//!     // Receive
//!     let packets = server.recv().await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Features
//!
//! - **Security:** ChaCha20-Poly1305 encryption, Kyber768 key exchange
//! - **Reliability:** Automatic retransmission, FEC, congestion control
//! - **Performance:** Zero-copy, adaptive compression, QoS
//! - **Mobility:** Connection migration, NAT traversal, STUN
//!
//! ## Documentation
//!
//! - [API Reference](docs/API.md)
//! - [Architecture Guide](docs/ARCHITECTURE.md)
//! - [Performance Guide](docs/PERFORMANCE.md)
//! - [Protocol Comparison](docs/PROTOCOL_COMPARISON_RU.md)
//!
//! ## License
//!
//! MIT License - See LICENSE file for details

#![doc(html_logo_url = "https://example.com/logo.png")]
#![doc(html_favicon_url = "https://example.com/favicon.ico")]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub use jsp_core as core;
pub use jsp_transport as transport;

/// Re-export commonly used types
pub mod prelude {
    pub use crate::transport::connection::Connection;
    pub use crate::transport::config::ConnectionConfig;
    pub use crate::core::types::control::CloseReason;
}
