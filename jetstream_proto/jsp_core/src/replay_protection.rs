use std::collections::HashSet;
use std::time::{SystemTime, Duration};

/// Защита от replay-атак для 0-RTT
pub struct ReplayProtection {
    /// Sliding window для nonce tracking
    nonce_window: HashSet<u64>,
    
    /// Максимальный размер окна
    max_window_size: usize,
    
    /// Минимальный nonce в текущем окне
    min_nonce: u64,
    
    /// Максимальный nonce в текущем окне
    max_nonce: u64,
    
    /// Максимальная разница во времени (clock skew tolerance)
    max_clock_skew: Duration,
    
    /// Время последней очистки
    last_cleanup: SystemTime,
    
    /// Интервал очистки
    cleanup_interval: Duration,
}

impl ReplayProtection {
    /// Создать новый ReplayProtection
    pub fn new(max_window_size: usize, max_clock_skew: Duration) -> Self {
        Self {
            nonce_window: HashSet::new(),
            max_window_size,
            min_nonce: 0,
            max_nonce: 0,
            max_clock_skew,
            last_cleanup: SystemTime::now(),
            cleanup_interval: Duration::from_secs(60),
        }
    }
    
    /// Проверить и зарегистрировать nonce
    pub fn check_and_register(&mut self, nonce: u64, timestamp: u64) -> Result<(), ReplayError> {
        // 1. Проверка timestamp
        self.validate_timestamp(timestamp)?;
        
        // 2. Проверка nonce
        if self.is_duplicate(nonce) {
            return Err(ReplayError::DuplicateNonce);
        }
        
        // 3. Регистрация nonce
        self.register_nonce(nonce);
        
        // 4. Периодическая очистка
        self.maybe_cleanup();
        
        Ok(())
    }
    
    fn validate_timestamp(&self, timestamp: u64) -> Result<(), ReplayError> {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_| ReplayError::InvalidTimestamp)?
            .as_secs();
        
        let diff = now.abs_diff(timestamp);
        
        if diff > self.max_clock_skew.as_secs() {
            return Err(ReplayError::TimestampOutOfRange);
        }
        
        Ok(())
    }
    
    fn is_duplicate(&self, nonce: u64) -> bool {
        // Если nonce меньше минимального в окне, это старый пакет
        if !self.nonce_window.is_empty() && nonce < self.min_nonce {
            return true;
        }
        
        // Проверить наличие в окне
        self.nonce_window.contains(&nonce)
    }
    
    fn register_nonce(&mut self, nonce: u64) {
        // Обновить границы окна
        if self.nonce_window.is_empty() {
            self.min_nonce = nonce;
            self.max_nonce = nonce;
        } else {
            if nonce < self.min_nonce {
                self.min_nonce = nonce;
            }
            if nonce > self.max_nonce {
                self.max_nonce = nonce;
            }
        }
        
        // Добавить nonce
        self.nonce_window.insert(nonce);
        
        // Если окно переполнено, удалить старые nonce
        if self.nonce_window.len() > self.max_window_size {
            self.shrink_window();
        }
    }
    
    fn shrink_window(&mut self) {
        // Удалить nonce меньше (max_nonce - window_size + 1)
        // Это оставит ровно max_window_size элементов
        let threshold = self.max_nonce.saturating_sub(self.max_window_size as u64 - 1);
        self.nonce_window.retain(|&n| n >= threshold);
        self.min_nonce = threshold;
    }
    
    fn maybe_cleanup(&mut self) {
        let now = SystemTime::now();
        if now.duration_since(self.last_cleanup).unwrap_or(Duration::ZERO) > self.cleanup_interval {
            // Очистка выполнена через shrink_window
            self.last_cleanup = now;
        }
    }
    
    /// Получить текущий размер окна
    pub fn window_size(&self) -> usize {
        self.nonce_window.len()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReplayError {
    #[error("Duplicate nonce detected - possible replay attack")]
    DuplicateNonce,
    
    #[error("Timestamp out of acceptable range (clock skew too large)")]
    TimestampOutOfRange,
    
    #[error("Invalid timestamp")]
    InvalidTimestamp,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nonce_tracking() {
        let mut rp = ReplayProtection::new(100, Duration::from_secs(60));
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Первый nonce должен пройти
        assert!(rp.check_and_register(1, now).is_ok());
        assert_eq!(rp.window_size(), 1);
        
        // Повторный nonce должен быть отклонен
        assert!(matches!(
            rp.check_and_register(1, now),
            Err(ReplayError::DuplicateNonce)
        ));
        
        // Новый nonce должен пройти
        assert!(rp.check_and_register(2, now).is_ok());
        assert_eq!(rp.window_size(), 2);
    }
    
    #[test]
    fn test_timestamp_validation() {
        let mut rp = ReplayProtection::new(100, Duration::from_secs(60));
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Текущее время должно пройти
        assert!(rp.check_and_register(1, now).is_ok());
        
        // Старое время (за пределами skew) должно быть отклонено
        assert!(matches!(
            rp.check_and_register(2, now - 400),
            Err(ReplayError::TimestampOutOfRange)
        ));
        
        // Будущее время (за пределами skew) должно быть отклонено
        assert!(matches!(
            rp.check_and_register(3, now + 400),
            Err(ReplayError::TimestampOutOfRange)
        ));
        
        // Время в пределах skew должно пройти
        assert!(rp.check_and_register(4, now + 30).is_ok());
        assert!(rp.check_and_register(5, now - 30).is_ok());
    }
    
    #[test]
    fn test_sliding_window() {
        let mut rp = ReplayProtection::new(10, Duration::from_secs(60));
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Заполнить окно
        for i in 1..=15 {
            assert!(rp.check_and_register(i, now).is_ok());
        }
        
        // Окно должно быть ограничено max_window_size
        assert!(rp.window_size() <= 10);
        
        // Старые nonce (за пределами окна) должны быть отклонены
        assert!(matches!(
            rp.check_and_register(1, now),
            Err(ReplayError::DuplicateNonce)
        ));
        
        // Новые nonce должны проходить
        assert!(rp.check_and_register(16, now).is_ok());
    }
    
    #[test]
    fn test_out_of_order_nonces() {
        let mut rp = ReplayProtection::new(100, Duration::from_secs(60));
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Test that we can add nonces in any order (as long as they're within the window)
        assert!(rp.check_and_register(100, now).is_ok());
        assert!(rp.check_and_register(105, now).is_ok());
        assert!(rp.check_and_register(102, now).is_ok());  // Out of order but within window
        assert!(rp.check_and_register(110, now).is_ok());
        
        // Duplicates should fail
        assert!(rp.check_and_register(100, now).is_err());
        assert!(rp.check_and_register(105, now).is_err());
    }
}
