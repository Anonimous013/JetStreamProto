use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Режимы доставки сообщений
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DeliveryMode {
    /// Гарантированная доставка с retransmit
    /// Пакеты ретранслируются до получения ACK
    #[default]
    Reliable,
    
    /// Частично надежная доставка с TTL (Time To Live)
    /// Пакеты ретранслируются только в пределах TTL
    /// После истечения TTL пакет считается устаревшим и не ретранслируется
    PartiallyReliable { ttl_ms: u32 },
    
    /// Без гарантий доставки, минимальная задержка
    /// Пакеты отправляются один раз без retransmit
    BestEffort,
}

impl DeliveryMode {
    /// Требуется ли ACK для этого режима
    pub fn requires_ack(&self) -> bool {
        match self {
            DeliveryMode::Reliable => true,
            DeliveryMode::PartiallyReliable { .. } => true,
            DeliveryMode::BestEffort => false,
        }
    }
    
    /// Требуется ли retransmit для этого режима
    pub fn requires_retransmit(&self) -> bool {
        match self {
            DeliveryMode::Reliable => true,
            DeliveryMode::PartiallyReliable { .. } => true,
            DeliveryMode::BestEffort => false,
        }
    }
    
    /// Получить TTL для режима
    pub fn ttl(&self) -> Option<Duration> {
        match self {
            DeliveryMode::PartiallyReliable { ttl_ms } => {
                Some(Duration::from_millis(*ttl_ms as u64))
            }
            _ => None,
        }
    }
    
    /// Проверить, истек ли TTL для пакета
    pub fn is_expired(&self, elapsed: Duration) -> bool {
        match self {
            DeliveryMode::PartiallyReliable { ttl_ms } => {
                let ttl = Duration::from_millis(*ttl_ms as u64);
                elapsed >= ttl
            }
            DeliveryMode::BestEffort => true, // BestEffort всегда "expired" для retransmit
            DeliveryMode::Reliable => false,  // Reliable никогда не истекает
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_delivery_mode_requires_ack() {
        assert!(DeliveryMode::Reliable.requires_ack());
        assert!(DeliveryMode::PartiallyReliable { ttl_ms: 1000 }.requires_ack());
        assert!(!DeliveryMode::BestEffort.requires_ack());
    }
    
    #[test]
    fn test_delivery_mode_requires_retransmit() {
        assert!(DeliveryMode::Reliable.requires_retransmit());
        assert!(DeliveryMode::PartiallyReliable { ttl_ms: 1000 }.requires_retransmit());
        assert!(!DeliveryMode::BestEffort.requires_retransmit());
    }
    
    #[test]
    fn test_delivery_mode_ttl() {
        assert_eq!(DeliveryMode::Reliable.ttl(), None);
        assert_eq!(
            DeliveryMode::PartiallyReliable { ttl_ms: 1000 }.ttl(),
            Some(Duration::from_millis(1000))
        );
        assert_eq!(DeliveryMode::BestEffort.ttl(), None);
    }
    
    #[test]
    fn test_delivery_mode_is_expired() {
        let reliable = DeliveryMode::Reliable;
        let partially = DeliveryMode::PartiallyReliable { ttl_ms: 100 };
        let best_effort = DeliveryMode::BestEffort;
        
        // Reliable никогда не истекает
        assert!(!reliable.is_expired(Duration::from_secs(1000)));
        
        // PartiallyReliable истекает после TTL
        assert!(!partially.is_expired(Duration::from_millis(50)));
        assert!(partially.is_expired(Duration::from_millis(100)));
        assert!(partially.is_expired(Duration::from_millis(150)));
        
        // BestEffort всегда "expired"
        assert!(best_effort.is_expired(Duration::from_millis(0)));
    }
    
    #[test]
    fn test_delivery_mode_default() {
        assert_eq!(DeliveryMode::default(), DeliveryMode::Reliable);
    }
    
    #[test]
    fn test_delivery_mode_serialization() {
        use serde_cbor;
        
        let modes = vec![
            DeliveryMode::Reliable,
            DeliveryMode::PartiallyReliable { ttl_ms: 500 },
            DeliveryMode::BestEffort,
        ];
        
        for mode in modes {
            let serialized = serde_cbor::to_vec(&mode).unwrap();
            let deserialized: DeliveryMode = serde_cbor::from_slice(&serialized).unwrap();
            assert_eq!(mode, deserialized);
        }
    }
}
