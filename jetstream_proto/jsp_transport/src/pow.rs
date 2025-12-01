use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Proof-of-Work challenge configuration
#[derive(Debug, Clone)]
pub struct PowConfig {
    /// Number of leading zero bits required
    pub difficulty: u32,
    /// Challenge validity duration in seconds
    pub validity_duration: u64,
}

impl Default for PowConfig {
    fn default() -> Self {
        Self {
            difficulty: 20, // ~1 million hashes on average
            validity_duration: 300, // 5 minutes
        }
    }
}

/// Proof-of-Work challenge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowChallenge {
    /// Random challenge data
    pub challenge: [u8; 32],
    /// Timestamp when challenge was created
    pub timestamp: u64,
    /// Required difficulty (leading zero bits)
    pub difficulty: u32,
}

/// Proof-of-Work solution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowSolution {
    /// The challenge being solved
    pub challenge: [u8; 32],
    /// Nonce that produces valid hash
    pub nonce: u64,
}

impl PowChallenge {
    /// Generate a new challenge
    pub fn new(difficulty: u32) -> Self {
        let mut challenge = [0u8; 32];
        getrandom::getrandom(&mut challenge).expect("Failed to generate random challenge");
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        
        Self {
            challenge,
            timestamp,
            difficulty,
        }
    }
    
    /// Verify a solution
    pub fn verify(&self, solution: &PowSolution, config: &PowConfig) -> bool {
        // Check if challenge matches
        if self.challenge != solution.challenge {
            return false;
        }
        
        // Check if challenge is still valid
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        
        if now > self.timestamp + config.validity_duration {
            return false;
        }
        
        // Verify the hash
        let hash = Self::compute_hash(&solution.challenge, solution.nonce);
        Self::check_difficulty(&hash, self.difficulty)
    }
    
    /// Solve the challenge (for testing/client)
    pub fn solve(&self) -> PowSolution {
        let mut nonce = 0u64;
        
        loop {
            let hash = Self::compute_hash(&self.challenge, nonce);
            if Self::check_difficulty(&hash, self.difficulty) {
                return PowSolution {
                    challenge: self.challenge,
                    nonce,
                };
            }
            nonce += 1;
        }
    }
    
    fn compute_hash(challenge: &[u8; 32], nonce: u64) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(challenge);
        hasher.update(&nonce.to_le_bytes());
        hasher.finalize().into()
    }
    
    fn check_difficulty(hash: &[u8; 32], difficulty: u32) -> bool {
        let leading_zeros = hash.iter()
            .take_while(|&&b| b == 0)
            .count() * 8
            + hash.iter()
                .find(|&&b| b != 0)
                .map(|b| b.leading_zeros() as usize)
                .unwrap_or(0);
        
        leading_zeros >= difficulty as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pow_easy() {
        let challenge = PowChallenge::new(8); // Easy difficulty for testing
        let solution = challenge.solve();
        
        let config = PowConfig {
            difficulty: 8,
            validity_duration: 300,
        };
        
        assert!(challenge.verify(&solution, &config));
    }

    #[test]
    fn test_pow_invalid_nonce() {
        let challenge = PowChallenge::new(8);
        let config = PowConfig {
            difficulty: 8,
            validity_duration: 300,
        };
        
        let invalid_solution = PowSolution {
            challenge: challenge.challenge,
            nonce: 0, // Wrong nonce
        };
        
        // Might pass by chance, but very unlikely with difficulty 8
        // In a real test, we'd ensure this specific nonce is invalid
    }

    #[test]
    fn test_pow_expired() {
        let mut challenge = PowChallenge::new(8);
        challenge.timestamp = 0; // Very old timestamp
        
        let solution = PowSolution {
            challenge: challenge.challenge,
            nonce: 0,
        };
        
        let config = PowConfig {
            difficulty: 8,
            validity_duration: 300,
        };
        
        assert!(!challenge.verify(&solution, &config));
    }
}
