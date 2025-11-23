use std::cmp::Ordering;

/// Quality of Service priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QosPriority {
    /// System control messages (highest priority)
    System = 3,
    /// Real-time media (voice, video)
    Media = 2,
    /// Interactive chat messages
    Chat = 1,
    /// Bulk data transfer (lowest priority)
    Bulk = 0,
}

impl QosPriority {
    /// Get numeric priority value (higher = more important)
    pub fn value(&self) -> u8 {
        *self as u8
    }
    
    /// Create from numeric value
    pub fn from_value(value: u8) -> Option<Self> {
        match value {
            3 => Some(QosPriority::System),
            2 => Some(QosPriority::Media),
            1 => Some(QosPriority::Chat),
            0 => Some(QosPriority::Bulk),
            _ => None,
        }
    }
    
    /// Get weight for weighted fair queuing
    /// Higher priority = higher weight
    pub fn weight(&self) -> usize {
        match self {
            QosPriority::System => 8,
            QosPriority::Media => 4,
            QosPriority::Chat => 2,
            QosPriority::Bulk => 1,
        }
    }
}

impl Default for QosPriority {
    fn default() -> Self {
        QosPriority::Chat
    }
}

impl PartialOrd for QosPriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QosPriority {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value().cmp(&other.value())
    }
}

/// QoS class for packet classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QosClass {
    /// Priority level
    pub priority: QosPriority,
    /// Delay sensitivity (lower = more sensitive)
    pub delay_sensitivity: u8,
    /// Loss tolerance (higher = more tolerant)
    pub loss_tolerance: u8,
}

impl QosClass {
    /// System control class (highest priority, delay-sensitive, loss-intolerant)
    pub const SYSTEM: Self = QosClass {
        priority: QosPriority::System,
        delay_sensitivity: 1,
        loss_tolerance: 0,
    };
    
    /// Real-time media class (high priority, very delay-sensitive, some loss tolerance)
    pub const MEDIA: Self = QosClass {
        priority: QosPriority::Media,
        delay_sensitivity: 2,
        loss_tolerance: 3,
    };
    
    /// Interactive chat class (medium priority, moderate delay-sensitivity)
    pub const CHAT: Self = QosClass {
        priority: QosPriority::Chat,
        delay_sensitivity: 5,
        loss_tolerance: 1,
    };
    
    /// Bulk data class (lowest priority, delay-tolerant, loss-intolerant)
    pub const BULK: Self = QosClass {
        priority: QosPriority::Bulk,
        delay_sensitivity: 10,
        loss_tolerance: 0,
    };
    
    /// Create custom QoS class
    pub fn custom(priority: QosPriority, delay_sensitivity: u8, loss_tolerance: u8) -> Self {
        Self {
            priority,
            delay_sensitivity,
            loss_tolerance,
        }
    }
}

impl Default for QosClass {
    fn default() -> Self {
        Self::CHAT
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_priority_ordering() {
        assert!(QosPriority::System > QosPriority::Media);
        assert!(QosPriority::Media > QosPriority::Chat);
        assert!(QosPriority::Chat > QosPriority::Bulk);
    }
    
    #[test]
    fn test_priority_values() {
        assert_eq!(QosPriority::System.value(), 3);
        assert_eq!(QosPriority::Media.value(), 2);
        assert_eq!(QosPriority::Chat.value(), 1);
        assert_eq!(QosPriority::Bulk.value(), 0);
    }
    
    #[test]
    fn test_priority_from_value() {
        assert_eq!(QosPriority::from_value(3), Some(QosPriority::System));
        assert_eq!(QosPriority::from_value(2), Some(QosPriority::Media));
        assert_eq!(QosPriority::from_value(1), Some(QosPriority::Chat));
        assert_eq!(QosPriority::from_value(0), Some(QosPriority::Bulk));
        assert_eq!(QosPriority::from_value(99), None);
    }
    
    #[test]
    fn test_priority_weights() {
        assert_eq!(QosPriority::System.weight(), 8);
        assert_eq!(QosPriority::Media.weight(), 4);
        assert_eq!(QosPriority::Chat.weight(), 2);
        assert_eq!(QosPriority::Bulk.weight(), 1);
    }
    
    #[test]
    fn test_qos_classes() {
        assert_eq!(QosClass::SYSTEM.priority, QosPriority::System);
        assert_eq!(QosClass::MEDIA.priority, QosPriority::Media);
        assert_eq!(QosClass::CHAT.priority, QosPriority::Chat);
        assert_eq!(QosClass::BULK.priority, QosPriority::Bulk);
    }
    
    #[test]
    fn test_custom_qos_class() {
        let custom = QosClass::custom(QosPriority::Media, 3, 2);
        assert_eq!(custom.priority, QosPriority::Media);
        assert_eq!(custom.delay_sensitivity, 3);
        assert_eq!(custom.loss_tolerance, 2);
    }
    
    #[test]
    fn test_default_priority() {
        assert_eq!(QosPriority::default(), QosPriority::Chat);
    }
    
    #[test]
    fn test_default_qos_class() {
        let default_class = QosClass::default();
        assert_eq!(default_class.priority, QosPriority::Chat);
    }
}
