use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Frame {
    Data {
        payload: ByteBuf,
    },
    Control {
        ctrl_type: u8,
        data: ByteBuf,
    },
    Ack {
        ack_sequence: u64,
        /// SACK ranges: (start, end) inclusive
        ranges: Vec<(u64, u64)>,
    },

    Handshake {
        version: u32,
        // Client Hello / Server Hello data
        data: ByteBuf,
    },
}
