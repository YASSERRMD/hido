//! Error types for HIDO.

use thiserror::Error;

/// Result type alias for HIDO operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in HIDO operations.
#[derive(Error, Debug)]
pub enum Error {
    // Identity errors
    #[error("DID generation failed: {0}")]
    DIDGenerationFailed(String),

    #[error("DID not found: {0}")]
    DIDNotFound(String),

    #[error("DID verification failed: {0}")]
    DIDVerificationFailed(String),

    #[error("Key rotation failed: {0}")]
    KeyRotationFailed(String),

    // Credential errors
    #[error("Credential issuance failed: {0}")]
    CredentialIssuanceFailed(String),

    #[error("Credential verification failed: {0}")]
    CredentialVerificationFailed(String),

    #[error("Credential expired")]
    CredentialExpired,

    #[error("Credential revoked")]
    CredentialRevoked,

    // Cryptography errors
    #[error("Signing failed: {0}")]
    SigningFailed(String),

    #[error("Signature verification failed")]
    SignatureVerificationFailed,

    #[error("Invalid key format: {0}")]
    InvalidKeyFormat(String),

    // Intent errors
    #[error("Intent serialization failed: {0}")]
    IntentSerializationFailed(String),

    #[error("Intent validation failed: {0}")]
    IntentValidationFailed(String),

    #[error("No capable agent found for intent")]
    NoCapableAgent,

    // Compression errors
    #[error("Compression failed: {0}")]
    CompressionFailed(String),

    #[error("Decompression failed: {0}")]
    DecompressionFailed(String),

    // Blockchain errors
    #[error("Block creation failed: {0}")]
    BlockCreationFailed(String),

    #[error("Block verification failed: {0}")]
    BlockVerificationFailed(String),

    #[error("Chain integrity violated at height {0}")]
    ChainIntegrityViolated(u64),

    #[error("Invalid parent hash")]
    InvalidParentHash,

    #[error("Block tampered")]
    BlockTampered,

    // Serialization errors
    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    // Generic errors
    #[error("Internal error: {0}")]
    Internal(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::SerializationError(err.to_string())
    }
}

impl From<bincode::Error> for Error {
    fn from(err: bincode::Error) -> Self {
        Error::SerializationError(err.to_string())
    }
}

impl From<ed25519_dalek::SignatureError> for Error {
    fn from(_: ed25519_dalek::SignatureError) -> Self {
        Error::SignatureVerificationFailed
    }
}
