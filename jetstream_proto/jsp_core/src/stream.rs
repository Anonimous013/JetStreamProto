use std::collections::HashMap;
use std::time::Instant;
use crate::types::delivery::DeliveryMode;

/// Stream state for multiplexing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamState {
    /// Stream is being opened
    Opening,
    /// Stream is open and ready for data
    Open,
    /// Stream is being closed gracefully
    Closing,
    /// Stream is closed
    Closed,
}

/// Individual stream within a connection
#[derive(Debug)]
pub struct Stream {
    /// Stream identifier
    pub id: u32,
    /// Current state
    pub state: StreamState,
    /// Send sequence number
    pub send_seq: u64,
    /// Receive sequence number
    pub recv_seq: u64,
    /// Last activity timestamp
    pub last_activity: Instant,
    /// Stream priority (higher = more important)
    pub priority: u8,
    /// Send window size for flow control
    pub send_window: u32,
    /// Receive window size for flow control
    pub recv_window: u32,
    /// Delivery mode for this stream
    pub delivery_mode: DeliveryMode,
}

impl Stream {
    pub fn new(id: u32, priority: u8, delivery_mode: DeliveryMode) -> Self {
        Self {
            id,
            state: StreamState::Opening,
            send_seq: 0,
            recv_seq: 0,
            last_activity: Instant::now(),
            priority,
            send_window: 65536, // 64KB default
            recv_window: 65536,
            delivery_mode,
        }
    }

    pub fn open(&mut self) {
        self.state = StreamState::Open;
        self.last_activity = Instant::now();
    }

    pub fn close(&mut self) {
        self.state = StreamState::Closing;
        self.last_activity = Instant::now();
    }

    pub fn finalize_close(&mut self) {
        self.state = StreamState::Closed;
    }

    pub fn update_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    pub fn is_active(&self) -> bool {
        matches!(self.state, StreamState::Open | StreamState::Opening)
    }

    pub fn can_send(&self) -> bool {
        self.state == StreamState::Open && self.send_window > 0
    }

    pub fn consume_send_window(&mut self, bytes: u32) {
        self.send_window = self.send_window.saturating_sub(bytes);
    }

    pub fn add_recv_window(&mut self, bytes: u32) {
        self.recv_window = self.recv_window.saturating_add(bytes);
    }
}

/// Stream manager for handling multiple streams
#[derive(Debug)]
pub struct StreamManager {
    streams: HashMap<u32, Stream>,
    next_stream_id: u32,
    max_streams: u32,
}

impl StreamManager {
    pub fn new(max_streams: u32) -> Self {
        Self {
            streams: HashMap::new(),
            next_stream_id: 1,
            max_streams,
        }
    }

    pub fn open_stream(&mut self, priority: u8, delivery_mode: DeliveryMode) -> Result<u32, &'static str> {
        if self.streams.len() >= self.max_streams as usize {
            return Err("Maximum streams reached");
        }

        let stream_id = self.next_stream_id;
        self.next_stream_id += 1;

        let mut stream = Stream::new(stream_id, priority, delivery_mode);
        stream.open();
        self.streams.insert(stream_id, stream);

        Ok(stream_id)
    }

    pub fn close_stream(&mut self, stream_id: u32) -> Result<(), &'static str> {
        let stream = self.streams.get_mut(&stream_id)
            .ok_or("Stream not found")?;
        stream.close();
        Ok(())
    }

    pub fn remove_stream(&mut self, stream_id: u32) {
        self.streams.remove(&stream_id);
    }

    pub fn get_stream(&self, stream_id: u32) -> Option<&Stream> {
        self.streams.get(&stream_id)
    }

    pub fn get_stream_mut(&mut self, stream_id: u32) -> Option<&mut Stream> {
        self.streams.get_mut(&stream_id)
    }

    pub fn active_stream_count(&self) -> usize {
        self.streams.values().filter(|s| s.is_active()).count()
    }

    pub fn cleanup_closed_streams(&mut self) {
        self.streams.retain(|_, stream| stream.state != StreamState::Closed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_lifecycle() {
        let mut stream = Stream::new(1, 0, DeliveryMode::Reliable);
        assert_eq!(stream.state, StreamState::Opening);

        stream.open();
        assert_eq!(stream.state, StreamState::Open);
        assert!(stream.is_active());

        stream.close();
        assert_eq!(stream.state, StreamState::Closing);

        stream.finalize_close();
        assert_eq!(stream.state, StreamState::Closed);
        assert!(!stream.is_active());
    }

    #[test]
    fn test_stream_flow_control() {
        let mut stream = Stream::new(1, 0, DeliveryMode::Reliable);
        stream.open();

        assert!(stream.can_send());
        stream.consume_send_window(1000);
        assert_eq!(stream.send_window, 64536);

        stream.add_recv_window(500);
        assert_eq!(stream.recv_window, 66036);
    }

    #[test]
    fn test_stream_manager() {
        let mut manager = StreamManager::new(10);

        let id1 = manager.open_stream(0, DeliveryMode::Reliable).unwrap();
        assert_eq!(id1, 1);

        let id2 = manager.open_stream(1, DeliveryMode::BestEffort).unwrap();
        assert_eq!(id2, 2);

        assert_eq!(manager.active_stream_count(), 2);

        manager.close_stream(id1).unwrap();
        manager.get_stream_mut(id1).unwrap().finalize_close();
        manager.cleanup_closed_streams();

        assert_eq!(manager.active_stream_count(), 1);
    }

    #[test]
    fn test_max_streams_limit() {
        let mut manager = StreamManager::new(2);

        manager.open_stream(0, DeliveryMode::Reliable).unwrap();
        manager.open_stream(0, DeliveryMode::Reliable).unwrap();

        let result = manager.open_stream(0, DeliveryMode::Reliable);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Maximum streams reached");
    }
}
