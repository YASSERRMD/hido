//! Common types used across HIDO modules.

use serde::{Deserialize, Serialize};

/// A 256-bit hash value (SHA3-256).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hash256(pub [u8; 32]);

impl Hash256 {
    /// Create a new Hash256 from bytes.
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Create a zero hash.
    pub fn zero() -> Self {
        Self([0u8; 32])
    }

    /// Get the bytes of the hash.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Convert to hex string.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Parse from hex string.
    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let bytes = hex::decode(s)?;
        if bytes.len() != 32 {
            return Err(hex::FromHexError::InvalidStringLength);
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

impl std::fmt::Display for Hash256 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl Default for Hash256 {
    fn default() -> Self {
        Self::zero()
    }
}

/// Timestamp wrapper for consistent serialization.
pub type Timestamp = chrono::DateTime<chrono::Utc>;

/// Get current UTC timestamp.
pub fn now() -> Timestamp {
    chrono::Utc::now()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash256_zero() {
        let hash = Hash256::zero();
        assert_eq!(hash.0, [0u8; 32]);
    }

    #[test]
    fn test_hash256_hex_roundtrip() {
        let bytes = [1u8; 32];
        let hash = Hash256::new(bytes);
        let hex_str = hash.to_hex();
        let parsed = Hash256::from_hex(&hex_str).unwrap();
        assert_eq!(hash, parsed);
    }

    #[test]
    fn test_hash256_display() {
        let hash = Hash256::zero();
        let display = format!("{}", hash);
        assert_eq!(display.len(), 64); // 32 bytes * 2 hex chars
    }
}
