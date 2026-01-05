//! Verifiable Credentials implementation.
//!
//! Provides credential issuance, verification, and management.

use crate::core::{now, Hash256, Result, Timestamp};
use crate::uail::crypto::sha3_256;
use crate::uail::did::DIDKey;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// A Verifiable Credential following W3C VC Data Model.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerifiableCredential {
    /// Credential ID
    pub id: String,
    /// Credential type(s)
    #[serde(rename = "type")]
    pub credential_type: Vec<String>,
    /// Issuer DID
    pub issuer: String,
    /// Subject DID
    pub subject: String,
    /// Issuance date
    #[serde(rename = "issuanceDate")]
    pub issuance_date: Timestamp,
    /// Expiration date (optional)
    #[serde(rename = "expirationDate")]
    pub expiration_date: Option<Timestamp>,
    /// Credential claims
    pub claims: HashMap<String, serde_json::Value>,
    /// Proof/signature
    pub proof: Option<CredentialProof>,
}

/// Proof attached to a credential.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CredentialProof {
    #[serde(rename = "type")]
    pub proof_type: String,
    pub created: Timestamp,
    #[serde(rename = "verificationMethod")]
    pub verification_method: String,
    #[serde(rename = "proofValue")]
    pub proof_value: String,
}

impl VerifiableCredential {
    /// Create a new unsigned credential.
    pub fn new(
        issuer: &DIDKey,
        subject: &DIDKey,
        credential_type: Vec<String>,
        claims: HashMap<String, serde_json::Value>,
        expiration: Option<Timestamp>,
    ) -> Self {
        let mut types = vec!["VerifiableCredential".to_string()];
        types.extend(credential_type);

        Self {
            id: format!("urn:uuid:{}", Uuid::new_v4()),
            credential_type: types,
            issuer: issuer.id.clone(),
            subject: subject.id.clone(),
            issuance_date: now(),
            expiration_date: expiration,
            claims,
            proof: None,
        }
    }

    /// Check if credential is expired.
    pub fn is_expired(&self) -> bool {
        if let Some(exp) = self.expiration_date {
            now() > exp
        } else {
            false
        }
    }

    /// Compute credential hash (for signing).
    pub fn hash(&self) -> Result<Hash256> {
        // Create a copy without proof for hashing
        let mut cred = self.clone();
        cred.proof = None;
        let json = serde_json::to_vec(&cred)?;
        Ok(sha3_256(&json))
    }

    /// Get claims as a typed value.
    pub fn get_claim<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.claims
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

/// Verification result for a credential.
#[derive(Debug)]
pub struct CredentialVerification {
    /// Whether the credential is valid
    pub valid: bool,
    /// Whether the credential is expired
    pub expired: bool,
    /// Whether the credential is revoked
    pub revoked: bool,
    /// Whether the signature is valid
    pub signature_valid: bool,
    /// Error message if invalid
    pub error: Option<String>,
}

/// Manager for credential operations.
pub struct CredentialManager {
    /// Revoked credential IDs
    revoked: HashSet<String>,
    /// Issued credentials (in-memory storage)
    credentials: HashMap<String, VerifiableCredential>,
}

impl CredentialManager {
    /// Create a new credential manager.
    pub fn new() -> Self {
        Self {
            revoked: HashSet::new(),
            credentials: HashMap::new(),
        }
    }

    /// Issue a new credential.
    pub fn issue(
        &mut self,
        issuer: &DIDKey,
        subject: &DIDKey,
        credential_type: Vec<String>,
        claims: HashMap<String, serde_json::Value>,
        expiration: Option<Timestamp>,
        sign_fn: impl FnOnce(&[u8]) -> Vec<u8>,
    ) -> Result<VerifiableCredential> {
        let mut credential =
            VerifiableCredential::new(issuer, subject, credential_type, claims, expiration);

        // Compute hash and sign
        let hash = credential.hash()?;
        let signature = sign_fn(hash.as_bytes());

        use base64::Engine;
        credential.proof = Some(CredentialProof {
            proof_type: "Ed25519Signature2020".to_string(),
            created: now(),
            verification_method: format!("{}#key-1", issuer.id),
            proof_value: base64::engine::general_purpose::STANDARD.encode(&signature),
        });

        self.credentials
            .insert(credential.id.clone(), credential.clone());
        Ok(credential)
    }

    /// Verify a credential.
    pub fn verify(
        &self,
        credential: &VerifiableCredential,
        verify_fn: impl FnOnce(&[u8], &[u8]) -> Result<()>,
    ) -> CredentialVerification {
        // Check expiration
        let expired = credential.is_expired();

        // Check revocation
        let revoked = self.revoked.contains(&credential.id);

        // Verify signature
        let signature_valid = if let Some(proof) = &credential.proof {
            let hash = match credential.hash() {
                Ok(h) => h,
                Err(e) => {
                    return CredentialVerification {
                        valid: false,
                        expired,
                        revoked,
                        signature_valid: false,
                        error: Some(e.to_string()),
                    }
                }
            };

            use base64::Engine;
            let signature = match base64::engine::general_purpose::STANDARD.decode(&proof.proof_value) {
                Ok(s) => s,
                Err(e) => {
                    return CredentialVerification {
                        valid: false,
                        expired,
                        revoked,
                        signature_valid: false,
                        error: Some(e.to_string()),
                    }
                }
            };

            verify_fn(hash.as_bytes(), &signature).is_ok()
        } else {
            false
        };

        let valid = signature_valid && !expired && !revoked;

        CredentialVerification {
            valid,
            expired,
            revoked,
            signature_valid,
            error: if valid {
                None
            } else {
                Some("Credential validation failed".to_string())
            },
        }
    }

    /// Revoke a credential.
    pub fn revoke(&mut self, credential_id: &str) {
        self.revoked.insert(credential_id.to_string());
    }

    /// Check if a credential is revoked.
    pub fn is_revoked(&self, credential_id: &str) -> bool {
        self.revoked.contains(credential_id)
    }

    /// Get a stored credential by ID.
    pub fn get(&self, credential_id: &str) -> Option<&VerifiableCredential> {
        self.credentials.get(credential_id)
    }
}

impl Default for CredentialManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uail::crypto::CryptoSuite;

    fn create_test_did() -> (DIDKey, CryptoSuite) {
        let crypto = CryptoSuite::new();
        let did = DIDKey::new(&crypto);
        (did, crypto)
    }

    #[test]
    fn test_credential_creation() {
        let (issuer, _) = create_test_did();
        let (subject, _) = create_test_did();

        let mut claims = HashMap::new();
        claims.insert("role".to_string(), serde_json::json!("admin"));

        let cred = VerifiableCredential::new(
            &issuer,
            &subject,
            vec!["RoleCredential".to_string()],
            claims,
            None,
        );

        assert!(cred.id.starts_with("urn:uuid:"));
        assert!(cred.credential_type.contains(&"VerifiableCredential".to_string()));
        assert!(cred.credential_type.contains(&"RoleCredential".to_string()));
        assert!(!cred.is_expired());
    }

    #[test]
    fn test_credential_issuance() {
        let (issuer, issuer_crypto) = create_test_did();
        let (subject, _) = create_test_did();

        let mut manager = CredentialManager::new();
        let mut claims = HashMap::new();
        claims.insert("level".to_string(), serde_json::json!(5));

        let cred = manager
            .issue(
                &issuer,
                &subject,
                vec!["CapabilityCredential".to_string()],
                claims,
                None,
                |msg| issuer_crypto.sign(msg),
            )
            .unwrap();

        assert!(cred.proof.is_some());
    }

    #[test]
    fn test_credential_verification() {
        let (issuer, issuer_crypto) = create_test_did();
        let (subject, _) = create_test_did();

        let mut manager = CredentialManager::new();
        let claims = HashMap::new();

        let cred = manager
            .issue(
                &issuer,
                &subject,
                vec!["TestCredential".to_string()],
                claims,
                None,
                |msg| issuer_crypto.sign(msg),
            )
            .unwrap();

        let result = manager.verify(&cred, |msg, sig| issuer_crypto.verify(msg, sig));
        assert!(result.valid);
        assert!(result.signature_valid);
        assert!(!result.expired);
        assert!(!result.revoked);
    }

    #[test]
    fn test_credential_revocation() {
        let (issuer, issuer_crypto) = create_test_did();
        let (subject, _) = create_test_did();

        let mut manager = CredentialManager::new();
        let claims = HashMap::new();

        let cred = manager
            .issue(
                &issuer,
                &subject,
                vec!["TestCredential".to_string()],
                claims,
                None,
                |msg| issuer_crypto.sign(msg),
            )
            .unwrap();

        manager.revoke(&cred.id);
        assert!(manager.is_revoked(&cred.id));

        let result = manager.verify(&cred, |msg, sig| issuer_crypto.verify(msg, sig));
        assert!(!result.valid);
        assert!(result.revoked);
    }

    #[test]
    fn test_credential_get_claim() {
        let (issuer, _) = create_test_did();
        let (subject, _) = create_test_did();

        let mut claims = HashMap::new();
        claims.insert("score".to_string(), serde_json::json!(100));

        let cred = VerifiableCredential::new(
            &issuer,
            &subject,
            vec!["ScoreCredential".to_string()],
            claims,
            None,
        );

        let score: Option<i32> = cred.get_claim("score");
        assert_eq!(score, Some(100));
    }
}
