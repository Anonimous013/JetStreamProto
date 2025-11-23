use serde::{Deserialize, Serialize};
use super::delivery::DeliveryMode;
use super::connection_id::ConnectionId;

// Frame type constants
pub const FRAME_TYPE_DATA: u8 = 0x00;
pub const FRAME_TYPE_HEARTBEAT: u8 = 0x01;
pub const FRAME_TYPE_CLOSE: u8 = 0x02;
pub const FRAME_TYPE_STREAM_CONTROL: u8 = 0x03;
pub const FRAME_TYPE_SESSION_TICKET: u8 = 0x04;
pub const FRAME_TYPE_ACK: u8 = 0x05;
pub const FRAME_TYPE_STUN: u8 = 0x06;
pub const FRAME_TYPE_TURN: u8 = 0x07;
pub const FRAME_TYPE_PATH_CHALLENGE: u8 = 0x08;
pub const FRAME_TYPE_PATH_RESPONSE: u8 = 0x09;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header {
    /// Stream identifier for multiplexing (0 = control stream)
    pub stream_id: u32,
    /// Message type (see FRAME_TYPE_* constants)
    pub msg_type: u8,
    /// Protocol flags
    pub flags: u8,
    /// Sequence number for ordering
    pub sequence: u64,
    /// Timestamp (milliseconds since epoch)
    pub timestamp: u64,
    /// Nonce for encryption
    pub nonce: u64,
    /// Delivery mode for this message
    pub delivery_mode: DeliveryMode,
    /// Optional piggybacked cumulative ACK
    pub piggybacked_ack: Option<u64>,
    /// Length of the payload (required for coalescing)
    pub payload_len: Option<u32>,
    /// Connection ID for mobility support
    pub connection_id: Option<ConnectionId>,
}

impl Header {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        stream_id: u32,
        msg_type: u8,
        flags: u8,
        sequence: u64,
        timestamp: u64,
        nonce: u64,
        delivery_mode: DeliveryMode,
        piggybacked_ack: Option<u64>,
        payload_len: Option<u32>,
    ) -> Self {
        Self {
            stream_id,
            msg_type,
            flags,
            sequence,
            timestamp,
            nonce,
            delivery_mode,
            piggybacked_ack,
            payload_len,
            connection_id: None, // Set separately after construction
        }
    }

    pub fn is_control_frame(&self) -> bool {
        self.msg_type != FRAME_TYPE_DATA
    }
}
