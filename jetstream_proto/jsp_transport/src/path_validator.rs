use std::net::SocketAddr;
use std::time::{Duration, Instant};
use jsp_core::types::path_validation::{PathChallenge, PathResponse};

/// Path validation state
pub struct PathValidator {
    /// Address being validated
    pub new_addr: SocketAddr,
    /// Challenge sent to new address
    pub challenge: PathChallenge,
    /// When validation started
    pub started_at: Instant,
    /// Validation timeout
    pub timeout: Duration,
}

impl PathValidator {
    /// Create new path validator
    pub fn new(new_addr: SocketAddr, timeout: Duration) -> Self {
        Self {
            new_addr,
            challenge: PathChallenge::new(),
            started_at: Instant::now(),
            timeout,
        }
    }
    
    /// Check if validation has timed out
    pub fn is_expired(&self) -> bool {
        self.started_at.elapsed() > self.timeout
    }
    
    /// Verify a response matches this challenge
    pub fn verify_response(&self, response: &PathResponse) -> bool {
        response.matches(&self.challenge)
    }
    
    /// Get the challenge to send
    pub fn get_challenge(&self) -> &PathChallenge {
        &self.challenge
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_path_validator_creation() {
        let addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();
        let validator = PathValidator::new(addr, Duration::from_secs(3));
        
        assert_eq!(validator.new_addr, addr);
        assert!(!validator.is_expired());
    }

    #[test]
    fn test_response_verification() {
        let addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();
        let validator = PathValidator::new(addr, Duration::from_secs(3));
        
        let response = PathResponse::for_challenge(validator.get_challenge());
        assert!(validator.verify_response(&response));
        
        // Wrong response
        let wrong_challenge = PathChallenge::new();
        let wrong_response = PathResponse::for_challenge(&wrong_challenge);
        assert!(!validator.verify_response(&wrong_response));
    }

    #[test]
    fn test_timeout() {
        let addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();
        let validator = PathValidator::new(addr, Duration::from_millis(50));
        
        assert!(!validator.is_expired());
        thread::sleep(Duration::from_millis(100));
        assert!(validator.is_expired());
    }
}
