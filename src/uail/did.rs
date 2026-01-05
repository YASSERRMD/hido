//! Decentralized Identifier (DID) implementation.
//!
//! Provides DID generation, management, and verification following W3C DID spec.

use crate::core::{now, Error, Hash256, Result, Timestamp};
use crate::uail::crypto::{sha3_256, CryptoSuite};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A Decentralized Identifier with associated key material.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DIDKey {
    /// The DID string (e.g., "did:hido:abc123")
    pub id: String,
    /// Public key bytes (Ed25519)
    pub public_key: [u8; 32],
    /// Creation timestamp
    pub created: Timestamp,
    /// Key version (incremented on rotation)
    pub version: u32,
}

impl DIDKey {
    /// Create a new DID from a crypto suite.
    pub fn new(crypto: &CryptoSuite) -> Self {
        let public_key = crypto.verifying_key_bytes();
        let id = Self::generate_id(&public_key);
        Self {
            id,
            public_key,
            created: now(),
            version: 1,
        }
    }

    /// Generate DID string from public key.
    fn generate_id(public_key: &[u8; 32]) -> String {
        let hash = sha3_256(public_key);
        let short_hash = &hash.to_hex()[..16];
        format!("did:hido:{}", short_hash)
    }

    /// Get the verifying key for signature verification.
    pub fn verifying_key(&self) -> Result<ed25519_dalek::VerifyingKey> {
        ed25519_dalek::VerifyingKey::from_bytes(&self.public_key)
            .map_err(|e| Error::InvalidKeyFormat(e.to_string()))
    }
}

impl PartialEq for DIDKey {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for DIDKey {}

impl std::hash::Hash for DIDKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl std::fmt::Display for DIDKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

/// DID Document for public discovery and verification.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DIDDocument {
    /// The DID this document describes
    pub id: String,
    /// Verification methods (public keys)
    pub verification_method: Vec<VerificationMethod>,
    /// Authentication methods
    pub authentication: Vec<String>,
    /// Service endpoints
    pub service: Vec<ServiceEndpoint>,
    /// Document creation time
    pub created: Timestamp,
    /// Last update time
    pub updated: Timestamp,
}

/// A verification method in a DID Document.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerificationMethod {
    pub id: String,
    #[serde(rename = "type")]
    pub method_type: String,
    pub controller: String,
    #[serde(rename = "publicKeyBase64")]
    pub public_key_base64: String,
}

/// A service endpoint in a DID Document.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    pub id: String,
    #[serde(rename = "type")]
    pub service_type: String,
    #[serde(rename = "serviceEndpoint")]
    pub endpoint: String,
}

impl DIDDocument {
    /// Create a new DID Document from a DID key.
    pub fn new(did_key: &DIDKey) -> Self {
        let now = now();
        use base64::Engine;
        let verification_method = VerificationMethod {
            id: format!("{}#key-1", did_key.id),
            method_type: "Ed25519VerificationKey2020".to_string(),
            controller: did_key.id.clone(),
            public_key_base64: base64::engine::general_purpose::STANDARD.encode(did_key.public_key),
        };

        Self {
            id: did_key.id.clone(),
            verification_method: vec![verification_method.clone()],
            authentication: vec![verification_method.id],
            service: Vec::new(),
            created: now,
            updated: now,
        }
    }

    /// Add a service endpoint.
    pub fn add_service(&mut self, service_type: &str, endpoint: &str) {
        let service = ServiceEndpoint {
            id: format!("{}#service-{}", self.id, Uuid::new_v4()),
            service_type: service_type.to_string(),
            endpoint: endpoint.to_string(),
        };
        self.service.push(service);
        self.updated = now();
    }

    /// Compute document hash for integrity verification.
    pub fn hash(&self) -> Result<Hash256> {
        let json = serde_json::to_vec(self)?;
        Ok(sha3_256(&json))
    }
}

/// Configuration for DID generation.
#[derive(Clone, Debug)]
pub struct DIDConfig {
    /// Whether to auto-create DID documents
    pub auto_create_document: bool,
}

impl Default for DIDConfig {
    fn default() -> Self {
        Self {
            auto_create_document: true,
        }
    }
}

/// Manager for DID lifecycle operations.
pub struct DIDManager {
    config: DIDConfig,
    /// Storage for DIDs (in-memory for now)
    dids: HashMap<String, (DIDKey, CryptoSuite)>,
    /// Storage for DID Documents
    documents: HashMap<String, DIDDocument>,
}

impl DIDManager {
    /// Create a new DID Manager.
    pub fn new(config: DIDConfig) -> Self {
        Self {
            config,
            dids: HashMap::new(),
            documents: HashMap::new(),
        }
    }

    /// Generate a new DID.
    pub async fn generate(&mut self) -> Result<DIDKey> {
        let crypto = CryptoSuite::new();
        let did_key = DIDKey::new(&crypto);

        if self.config.auto_create_document {
            let document = DIDDocument::new(&did_key);
            self.documents.insert(did_key.id.clone(), document);
        }

        self.dids.insert(did_key.id.clone(), (did_key.clone(), crypto));
        Ok(did_key)
    }

    /// Resolve a DID to its document.
    pub async fn resolve(&self, did: &str) -> Result<DIDDocument> {
        self.documents
            .get(did)
            .cloned()
            .ok_or_else(|| Error::DIDNotFound(did.to_string()))
    }

    /// Get the DID key.
    pub fn get(&self, did: &str) -> Option<&DIDKey> {
        self.dids.get(did).map(|(key, _)| key)
    }

    /// Rotate the key for a DID.
    pub async fn rotate(&mut self, did: &str) -> Result<DIDKey> {
        let (old_key, _) = self
            .dids
            .get(did)
            .ok_or_else(|| Error::DIDNotFound(did.to_string()))?;

        let new_crypto = CryptoSuite::new();
        let mut new_key = DIDKey::new(&new_crypto);
        // Keep the same DID, just update keys
        new_key.id = did.to_string();
        new_key.version = old_key.version + 1;

        // Update document
        if let Some(doc) = self.documents.get_mut(did) {
            use base64::Engine;
            let verification_method = VerificationMethod {
                id: format!("{}#key-{}", did, new_key.version),
                method_type: "Ed25519VerificationKey2020".to_string(),
                controller: did.to_string(),
                public_key_base64: base64::engine::general_purpose::STANDARD.encode(new_key.public_key),
            };
            doc.verification_method.push(verification_method.clone());
            doc.authentication.push(verification_method.id);
            doc.updated = now();
        }

        self.dids.insert(did.to_string(), (new_key.clone(), new_crypto));
        Ok(new_key)
    }

    /// Sign data with a DID's private key.
    pub fn sign(&self, did: &str, message: &[u8]) -> Result<Vec<u8>> {
        let (_, crypto) = self
            .dids
            .get(did)
            .ok_or_else(|| Error::DIDNotFound(did.to_string()))?;
        Ok(crypto.sign(message))
    }

    /// Verify a signature from a DID.
    pub fn verify(&self, did: &str, message: &[u8], signature: &[u8]) -> Result<()> {
        let (key, _) = self
            .dids
            .get(did)
            .ok_or_else(|| Error::DIDNotFound(did.to_string()))?;
        let verifying_key = key.verifying_key()?;
        crate::uail::crypto::verify(&verifying_key, message, signature)
    }

    /// List all managed DIDs.
    pub fn list(&self) -> Vec<&DIDKey> {
        self.dids.values().map(|(key, _)| key).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_did_generation() {
        let mut manager = DIDManager::new(DIDConfig::default());
        let did = manager.generate().await.unwrap();
        assert!(did.id.starts_with("did:hido:"));
        assert_eq!(did.version, 1);
    }

    #[tokio::test]
    async fn test_did_resolve() {
        let mut manager = DIDManager::new(DIDConfig::default());
        let did = manager.generate().await.unwrap();
        let doc = manager.resolve(&did.id).await.unwrap();
        assert_eq!(doc.id, did.id);
        assert!(!doc.verification_method.is_empty());
    }

    #[tokio::test]
    async fn test_did_not_found() {
        let manager = DIDManager::new(DIDConfig::default());
        let result = manager.resolve("did:hido:nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_did_rotate() {
        let mut manager = DIDManager::new(DIDConfig::default());
        let did1 = manager.generate().await.unwrap();
        let did2 = manager.rotate(&did1.id).await.unwrap();
        assert_eq!(did1.id, did2.id);
        assert_eq!(did2.version, 2);
        assert_ne!(did1.public_key, did2.public_key);
    }

    #[tokio::test]
    async fn test_did_sign_verify() {
        let mut manager = DIDManager::new(DIDConfig::default());
        let did = manager.generate().await.unwrap();
        let message = b"test message";
        let signature = manager.sign(&did.id, message).unwrap();
        assert!(manager.verify(&did.id, message, &signature).is_ok());
    }

    #[tokio::test]
    async fn test_did_verify_wrong_message() {
        let mut manager = DIDManager::new(DIDConfig::default());
        let did = manager.generate().await.unwrap();
        let message = b"test message";
        let signature = manager.sign(&did.id, message).unwrap();
        assert!(manager.verify(&did.id, b"wrong", &signature).is_err());
    }

    #[test]
    fn test_did_document_hash() {
        let crypto = CryptoSuite::new();
        let did_key = DIDKey::new(&crypto);
        let doc = DIDDocument::new(&did_key);
        let hash1 = doc.hash().unwrap();
        let hash2 = doc.hash().unwrap();
        assert_eq!(hash1, hash2);
    }
}
