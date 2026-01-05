//! Universal Agent Identity Layer (UAIL)
//!
//! Provides decentralized identity management:
//! - DID (Decentralized Identifier) generation and management
//! - Verifiable Credentials
//! - Cryptographic utilities

pub mod credential;
pub mod crypto;
pub mod did;

pub use credential::{CredentialManager, VerifiableCredential};
pub use crypto::{sign, verify, CryptoSuite};
pub use did::{DIDConfig, DIDDocument, DIDKey, DIDManager};
