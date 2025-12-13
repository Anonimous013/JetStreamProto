//! WebRTC Data Channel

use tokio::sync::mpsc;
use bytes::Bytes;

/// Data channel state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataChannelState {
    Connecting,
    Open,
    Closing,
    Closed,
}

/// Data channel for WebRTC transport
pub struct DataChannel {
    label: String,
    #[allow(dead_code)]
    ordered: bool,
    #[allow(dead_code)]
    max_retransmits: Option<u16>,
    state: DataChannelState,
    tx: mpsc::UnboundedSender<Bytes>,
    rx: mpsc::UnboundedReceiver<Bytes>,
}

impl DataChannel {
    /// Create a new data channel
    pub fn new(label: String, ordered: bool, max_retransmits: Option<u16>) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        
        Self {
            label,
            ordered,
            max_retransmits,
            state: DataChannelState::Connecting,
            tx,
            rx,
        }
    }
    
    /// Get channel label
    pub fn label(&self) -> &str {
        &self.label
    }
    
    /// Get channel state
    pub fn state(&self) -> DataChannelState {
        self.state
    }
    
    /// Send data on the channel
    pub async fn send(&self, data: Bytes) -> Result<(), Box<dyn std::error::Error>> {
        if self.state != DataChannelState::Open {
            return Err("Data channel not open".into());
        }
        
        self.tx.send(data)?;
        Ok(())
    }
    
    /// Receive data from the channel
    pub async fn recv(&mut self) -> Option<Bytes> {
        self.rx.recv().await
    }
    
    /// Close the data channel
    pub fn close(&mut self) {
        self.state = DataChannelState::Closing;
        // Channel will be closed when tx is dropped
    }
    
    /// Check if channel is reliable
    pub fn is_reliable(&self) -> bool {
        self.max_retransmits.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_channel_creation() {
        let channel = DataChannel::new("test".to_string(), true, None);
        assert_eq!(channel.label(), "test");
        assert_eq!(channel.state(), DataChannelState::Connecting);
        assert!(channel.is_reliable());
    }
    
    #[tokio::test]
    async fn test_data_channel_send_recv() {
        let mut channel = DataChannel::new("test".to_string(), true, None);
        channel.state = DataChannelState::Open;
        
        let data = Bytes::from("hello");
        channel.send(data.clone()).await.unwrap();
        
        let received = channel.recv().await.unwrap();
        assert_eq!(received, data);
    }
}
