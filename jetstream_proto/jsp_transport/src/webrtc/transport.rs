//! WebRTC Transport Implementation

use super::WebRTCConfig;
use super::data_channel::DataChannel;
use super::ice::{IceGatherer, IceConnectionState};
use bytes::Bytes;
use std::sync::Arc;
use tokio::sync::Mutex;

/// WebRTC transport for JetStreamProto
pub struct WebRTCTransport {
    config: WebRTCConfig,
    data_channel: Arc<Mutex<Option<DataChannel>>>,
    ice_gatherer: Arc<Mutex<IceGatherer>>,
    state: Arc<Mutex<IceConnectionState>>,
}

impl WebRTCTransport {
    /// Create a new WebRTC transport
    pub fn new(config: WebRTCConfig) -> Result<Self, Box<dyn std::error::Error>> {
        config.validate()?;
        
        Ok(Self {
            config,
            data_channel: Arc::new(Mutex::new(None)),
            ice_gatherer: Arc::new(Mutex::new(IceGatherer::new())),
            state: Arc::new(Mutex::new(IceConnectionState::New)),
        })
    }
    
    /// Initialize the WebRTC connection
    pub async fn initialize(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Gather ICE candidates
        let mut gatherer = self.ice_gatherer.lock().await;
        gatherer.gather_host_candidates().await?;
        
        // Create data channel
        let channel = DataChannel::new(
            self.config.data_channel_label.clone(),
            self.config.ordered,
            self.config.max_retransmits,
        );
        
        *self.data_channel.lock().await = Some(channel);
        *self.state.lock().await = IceConnectionState::Checking;
        
        tracing::info!("WebRTC transport initialized");
        Ok(())
    }
    
    /// Send data over WebRTC
    pub async fn send(&self, data: &[u8]) -> Result<usize, Box<dyn std::error::Error>> {
        let channel_guard = self.data_channel.lock().await;
        let channel = channel_guard.as_ref()
            .ok_or("Data channel not initialized")?;
        
        let bytes = Bytes::copy_from_slice(data);
        let len = bytes.len();
        channel.send(bytes).await?;
        
        Ok(len)
    }
    
    /// Receive data from WebRTC
    pub async fn recv(&self, buf: &mut [u8]) -> Result<usize, Box<dyn std::error::Error>> {
        let mut channel_guard = self.data_channel.lock().await;
        let channel = channel_guard.as_mut()
            .ok_or("Data channel not initialized")?;
        
        let data = channel.recv().await
            .ok_or("Channel closed")?;
        
        let len = data.len().min(buf.len());
        buf[..len].copy_from_slice(&data[..len]);
        
        Ok(len)
    }
    
    /// Get ICE connection state
    pub async fn connection_state(&self) -> IceConnectionState {
        *self.state.lock().await
    }
    
    /// Close the WebRTC connection
    pub async fn close(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut channel_guard = self.data_channel.lock().await;
        if let Some(channel) = channel_guard.as_mut() {
            channel.close();
        }
        
        *self.state.lock().await = IceConnectionState::Closed;
        tracing::info!("WebRTC transport closed");
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_webrtc_transport_creation() {
        let config = WebRTCConfig::default();
        let transport = WebRTCTransport::new(config);
        assert!(transport.is_ok());
    }
    
    #[tokio::test]
    async fn test_webrtc_transport_initialize() {
        let config = WebRTCConfig::default();
        let transport = WebRTCTransport::new(config).unwrap();
        
        let result = transport.initialize().await;
        assert!(result.is_ok());
        
        let state = transport.connection_state().await;
        assert_eq!(state, IceConnectionState::Checking);
    }
}
