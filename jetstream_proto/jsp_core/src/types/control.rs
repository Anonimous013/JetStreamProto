use serde::{Deserialize, Serialize};

/// Heartbeat frame for connection liveness checks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HeartbeatFrame {
    /// Sequence number for tracking heartbeats
    pub sequence: u64,
    /// True if this is a response (pong), false if request (ping)
    pub is_response: bool,
}

impl HeartbeatFrame {
    pub fn ping(sequence: u64) -> Self {
        Self {
            sequence,
            is_response: false,
        }
    }

    pub fn pong(sequence: u64) -> Self {
        Self {
            sequence,
            is_response: true,
        }
    }
}

/// Close frame for graceful connection shutdown
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloseFrame {
    /// Reason code for closure
    pub reason_code: CloseReason,
    /// Optional human-readable message
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CloseReason {
    /// Normal closure initiated by application
    Normal = 0,
    /// Going away (e.g., server shutdown)
    GoingAway = 1,
    /// Protocol error
    ProtocolError = 2,
    /// Session timeout
    Timeout = 3,
    /// Rate limit exceeded
    RateLimitExceeded = 4,
    /// Internal error
    InternalError = 5,
}

impl CloseFrame {
    pub fn normal() -> Self {
        Self {
            reason_code: CloseReason::Normal,
            message: None,
        }
    }

    pub fn with_reason(reason_code: CloseReason, message: impl Into<String>) -> Self {
        Self {
            reason_code,
            message: Some(message.into()),
        }
    }
}

/// Session ticket for 0-RTT resumption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTicket {
    /// Ticket identifier
    pub ticket_id: [u8; 32],
    /// Encrypted session state
    pub encrypted_state: Vec<u8>,
    /// Ticket creation timestamp (seconds since UNIX epoch)
    pub created_at: u64,
    /// Ticket lifetime in seconds
    pub lifetime: u32,
}

/// Stream control frame for multiplexing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StreamFrame {
    /// Stream identifier
    pub stream_id: u32,
    /// Stream control operation
    pub operation: StreamOperation,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StreamOperation {
    /// Open a new stream
    Open,
    /// Close an existing stream
    Close,
    /// Reset stream due to error
    Reset,
    /// Stream data (payload in separate field)
    Data,
}

/// Acknowledgment frame
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AckFrame {
    /// Cumulative acknowledgment (highest sequence number received in order)
    pub cumulative_ack: u64,
    /// SACK ranges (start, end) inclusive
    pub sack_ranges: Vec<(u64, u64)>,
}

/// Configuration for session timeouts and limits
#[derive(Debug, Clone, Copy)]
pub struct SessionConfig {
    /// Session timeout in seconds (default: 30)
    pub timeout_secs: u64,
    /// Maximum idle time before sending heartbeat (default: 5)
    pub heartbeat_interval_secs: u64,
    /// Number of missed heartbeats before timeout (default: 3)
    pub heartbeat_timeout_count: u32,
    /// Maximum concurrent streams per connection (default: 100)
    pub max_streams: u32,
    
    /// Enable replay protection for 0-RTT (default: true)
    pub enable_replay_protection: bool,
    /// Replay protection window size (default: 10000)
    pub replay_window_size: usize,
    /// Maximum clock skew tolerance in seconds (default: 300)
    pub max_clock_skew_secs: u64,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            heartbeat_interval_secs: 5,
            heartbeat_timeout_count: 3,
            max_streams: 100,
            enable_replay_protection: true,
            replay_window_size: 10000,
            max_clock_skew_secs: 300, // 5 minutes
        }
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Copy)]
pub struct RateLimitConfig {
    /// Maximum messages per second per connection
    pub messages_per_second: u32,
    /// Maximum bytes per second per connection
    pub bytes_per_second: u64,
    /// Global server-wide message rate limit
    pub global_messages_per_second: Option<u32>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            messages_per_second: 100,
            bytes_per_second: 1_048_576, // 1 MB/s
            global_messages_per_second: Some(10_000),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heartbeat_frame() {
        let ping = HeartbeatFrame::ping(42);
        assert_eq!(ping.sequence, 42);
        assert!(!ping.is_response);

        let pong = HeartbeatFrame::pong(42);
        assert_eq!(pong.sequence, 42);
        assert!(pong.is_response);
    }

    #[test]
    fn test_close_frame() {
        let close = CloseFrame::normal();
        assert_eq!(close.reason_code, CloseReason::Normal);
        assert!(close.message.is_none());

        let close_with_msg = CloseFrame::with_reason(
            CloseReason::Timeout,
            "Session expired"
        );
        assert_eq!(close_with_msg.reason_code, CloseReason::Timeout);
        assert_eq!(close_with_msg.message.as_deref(), Some("Session expired"));
    }

    #[test]
    fn test_stream_frame() {
        let open = StreamFrame {
            stream_id: 1,
            operation: StreamOperation::Open,
        };
        assert_eq!(open.stream_id, 1);
        assert_eq!(open.operation, StreamOperation::Open);
    }

    #[test]
    fn test_default_configs() {
        let session_config = SessionConfig::default();
        assert_eq!(session_config.timeout_secs, 30);
        assert_eq!(session_config.heartbeat_interval_secs, 5);

        let rate_config = RateLimitConfig::default();
        assert_eq!(rate_config.messages_per_second, 100);
    }
}
