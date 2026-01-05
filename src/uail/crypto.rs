//! Cryptographic utilities for HIDO.
//!
//! Provides Ed25519 signing/verification and SHA3-256 hashing.

use crate::core::{Error, Hash256, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha3::{Digest, Sha3_256};

/// Cryptographic suite for HIDO operations.
#[derive(Clone)]
pub struct CryptoSuite {
    signing_key: SigningKey,
}

impl CryptoSuite {
    /// Create a new CryptoSuite with a random key pair.
    pub fn new() -> Self {
        use rand::RngCore;
        let mut csprng = rand::rngs::OsRng;
        let mut secret_key_bytes = [0u8; 32];
        csprng.fill_bytes(&mut secret_key_bytes);
        let signing_key = SigningKey::from_bytes(&secret_key_bytes);
        Self { signing_key }
    }

    /// Create from existing signing key bytes.
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self> {
        let signing_key = SigningKey::from_bytes(bytes);
        Ok(Self { signing_key })
    }

    /// Get the signing key bytes.
    pub fn signing_key_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Get the verifying (public) key.
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    /// Get the verifying key bytes.
    pub fn verifying_key_bytes(&self) -> [u8; 32] {
        self.verifying_key().to_bytes()
    }

    /// Sign a message.
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        let signature = self.signing_key.sign(message);
        signature.to_bytes().to_vec()
    }

    /// Verify a signature.
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<()> {
        self.verify_with_key(&self.verifying_key(), message, signature)
    }

    /// Verify a signature with a specific public key.
    pub fn verify_with_key(
        &self,
        public_key: &VerifyingKey,
        message: &[u8],
        signature: &[u8],
    ) -> Result<()> {
        let sig_bytes: [u8; 64] = signature
            .try_into()
            .map_err(|_| Error::InvalidKeyFormat("Invalid signature length".into()))?;
        let sig = Signature::from_bytes(&sig_bytes);
        public_key.verify(message, &sig)?;
        Ok(())
    }
}

impl Default for CryptoSuite {
    fn default() -> Self {
        Self::new()
    }
}

/// Sign a message with a signing key.
pub fn sign(signing_key: &SigningKey, message: &[u8]) -> Vec<u8> {
    let signature = signing_key.sign(message);
    signature.to_bytes().to_vec()
}

/// Verify a signature with a public key.
pub fn verify(public_key: &VerifyingKey, message: &[u8], signature: &[u8]) -> Result<()> {
    let sig_bytes: [u8; 64] = signature
        .try_into()
        .map_err(|_| Error::InvalidKeyFormat("Invalid signature length".into()))?;
    let sig = Signature::from_bytes(&sig_bytes);
    public_key.verify(message, &sig)?;
    Ok(())
}

/// Compute SHA3-256 hash of data.
pub fn sha3_256(data: &[u8]) -> Hash256 {
    let mut hasher = Sha3_256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&result);
    Hash256::new(bytes)
}

/// Compute SHA3-256 hash of multiple data chunks.
pub fn sha3_256_multi(chunks: &[&[u8]]) -> Hash256 {
    let mut hasher = Sha3_256::new();
    for chunk in chunks {
        hasher.update(chunk);
    }
    let result = hasher.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&result);
    Hash256::new(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crypto_suite_new() {
        let suite = CryptoSuite::new();
        assert_eq!(suite.signing_key_bytes().len(), 32);
        assert_eq!(suite.verifying_key_bytes().len(), 32);
    }

    #[test]
    fn test_sign_and_verify() {
        let suite = CryptoSuite::new();
        let message = b"Hello, HIDO!";
        let signature = suite.sign(message);
        assert!(suite.verify(message, &signature).is_ok());
    }

    #[test]
    fn test_verify_wrong_message() {
        let suite = CryptoSuite::new();
        let message = b"Hello, HIDO!";
        let wrong_message = b"Wrong message";
        let signature = suite.sign(message);
        assert!(suite.verify(wrong_message, &signature).is_err());
    }

    #[test]
    fn test_sha3_256() {
        let data = b"test data";
        let hash1 = sha3_256(data);
        let hash2 = sha3_256(data);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_sha3_256_different_data() {
        let hash1 = sha3_256(b"data1");
        let hash2 = sha3_256(b"data2");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_sha3_256_multi() {
        let chunks: &[&[u8]] = &[b"chunk1", b"chunk2"];
        let hash = sha3_256_multi(chunks);
        assert_eq!(hash.as_bytes().len(), 32);
    }

    #[test]
    fn test_crypto_suite_from_bytes() {
        let suite1 = CryptoSuite::new();
        let bytes = suite1.signing_key_bytes();
        let suite2 = CryptoSuite::from_bytes(&bytes).unwrap();
        assert_eq!(suite1.verifying_key_bytes(), suite2.verifying_key_bytes());
    }
}
