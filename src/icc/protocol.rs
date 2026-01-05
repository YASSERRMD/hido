//! Intent protocol for message exchange.
//!
//! Defines message types and handlers for intent communication.

use crate::core::{now, Hash256, Result, Timestamp};
use crate::icc::intent::SemanticIntent;
use crate::uail::crypto::sha3_256;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Message types for the intent protocol.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    /// New intent request
    Request,
    /// Response to an intent
    Response,
    /// Acknowledgment
    Ack,
    /// Error message
    Error,
    /// Intent status update
    Status,
    /// Intent cancellation
    Cancel,
}

/// Status of an intent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentStatus {
    Pending,
    Accepted,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

/// An intent protocol message.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntentMessage {
    /// Message ID
    pub id: String,
    /// Message type
    pub message_type: MessageType,
    /// Sender DID
    pub sender: String,
    /// Recipient DID
    pub recipient: String,
    /// The intent (for Request type)
    pub intent: Option<SemanticIntent>,
    /// Response payload
    pub payload: Option<serde_json::Value>,
    /// Intent status (for Status type)
    pub status: Option<IntentStatus>,
    /// Error message (for Error type)
    pub error: Option<String>,
    /// Correlation ID (links request to response)
    pub correlation_id: String,
    /// Message timestamp
    pub timestamp: Timestamp,
    /// Digital signature
    pub signature: Option<Vec<u8>>,
}

impl IntentMessage {
    /// Create a new request message.
    pub fn request(intent: SemanticIntent, recipient: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            message_type: MessageType::Request,
            sender: intent.sender.clone(),
            recipient: recipient.to_string(),
            correlation_id: intent.correlation_id.clone().unwrap_or_else(|| intent.id.clone()),
            intent: Some(intent),
            payload: None,
            status: None,
            error: None,
            timestamp: now(),
            signature: None,
        }
    }

    /// Create a response message.
    pub fn response(
        original: &IntentMessage,
        sender: &str,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            message_type: MessageType::Response,
            sender: sender.to_string(),
            recipient: original.sender.clone(),
            correlation_id: original.correlation_id.clone(),
            intent: None,
            payload: Some(payload),
            status: None,
            error: None,
            timestamp: now(),
            signature: None,
        }
    }

    /// Create an acknowledgment message.
    pub fn ack(original: &IntentMessage, sender: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            message_type: MessageType::Ack,
            sender: sender.to_string(),
            recipient: original.sender.clone(),
            correlation_id: original.correlation_id.clone(),
            intent: None,
            payload: None,
            status: None,
            error: None,
            timestamp: now(),
            signature: None,
        }
    }

    /// Create an error message.
    pub fn error(original: &IntentMessage, sender: &str, error: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            message_type: MessageType::Error,
            sender: sender.to_string(),
            recipient: original.sender.clone(),
            correlation_id: original.correlation_id.clone(),
            intent: None,
            payload: None,
            status: None,
            error: Some(error.to_string()),
            timestamp: now(),
            signature: None,
        }
    }

    /// Create a status update message.
    pub fn status(original: &IntentMessage, sender: &str, status: IntentStatus) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            message_type: MessageType::Status,
            sender: sender.to_string(),
            recipient: original.sender.clone(),
            correlation_id: original.correlation_id.clone(),
            intent: None,
            payload: None,
            status: Some(status),
            error: None,
            timestamp: now(),
            signature: None,
        }
    }

    /// Compute message hash for signing.
    pub fn hash(&self) -> Result<Hash256> {
        let mut msg = self.clone();
        msg.signature = None;
        let json = serde_json::to_vec(&msg)?;
        Ok(sha3_256(&json))
    }

    /// Sign the message.
    pub fn sign(&mut self, sign_fn: impl FnOnce(&[u8]) -> Vec<u8>) -> Result<()> {
        let hash = self.hash()?;
        self.signature = Some(sign_fn(hash.as_bytes()));
        Ok(())
    }

    /// Verify the message signature.
    pub fn verify(&self, verify_fn: impl FnOnce(&[u8], &[u8]) -> Result<()>) -> Result<()> {
        let hash = self.hash()?;
        if let Some(sig) = &self.signature {
            verify_fn(hash.as_bytes(), sig)
        } else {
            Err(crate::core::Error::SignatureVerificationFailed)
        }
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    /// Deserialize from JSON.
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
}

/// Protocol handler trait for processing intent messages.
pub trait IntentProtocol: Send + Sync {
    /// Handle an incoming message.
    fn handle(&self, message: IntentMessage) -> Result<Option<IntentMessage>>;

    /// Get supported intent domains.
    fn supported_domains(&self) -> Vec<crate::icc::intent::IntentDomain>;

    /// Get supported actions.
    fn supported_actions(&self) -> Vec<String>;
}

/// Simple echo protocol for testing.
pub struct EchoProtocol {
    agent_id: String,
}

impl EchoProtocol {
    pub fn new(agent_id: &str) -> Self {
        Self {
            agent_id: agent_id.to_string(),
        }
    }
}

impl IntentProtocol for EchoProtocol {
    fn handle(&self, message: IntentMessage) -> Result<Option<IntentMessage>> {
        match message.message_type {
            MessageType::Request => {
                let response = IntentMessage::response(
                    &message,
                    &self.agent_id,
                    serde_json::json!({
                        "echo": true,
                        "original_intent_id": message.intent.as_ref().map(|i| i.id.clone())
                    }),
                );
                Ok(Some(response))
            }
            _ => Ok(None),
        }
    }

    fn supported_domains(&self) -> Vec<crate::icc::intent::IntentDomain> {
        vec![
            crate::icc::intent::IntentDomain::Data,
            crate::icc::intent::IntentDomain::Communication,
        ]
    }

    fn supported_actions(&self) -> Vec<String> {
        vec!["echo".to_string(), "ping".to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icc::intent::{IntentDomain, SemanticIntent};
    use crate::uail::crypto::CryptoSuite;
    use crate::uail::DIDKey;

    fn create_test_did() -> DIDKey {
        let crypto = CryptoSuite::new();
        DIDKey::new(&crypto)
    }

    #[test]
    fn test_request_message() {
        let sender = create_test_did();
        let intent = SemanticIntent::new(&sender, IntentDomain::Data, "read");
        let msg = IntentMessage::request(intent.clone(), "did:hido:recipient");

        assert_eq!(msg.message_type, MessageType::Request);
        assert!(msg.intent.is_some());
        assert_eq!(msg.correlation_id, intent.correlation_id.unwrap());
    }

    #[test]
    fn test_response_message() {
        let sender = create_test_did();
        let intent = SemanticIntent::new(&sender, IntentDomain::Data, "read");
        let request = IntentMessage::request(intent, "did:hido:recipient");

        let response = IntentMessage::response(
            &request,
            "did:hido:recipient",
            serde_json::json!({"result": "ok"}),
        );

        assert_eq!(response.message_type, MessageType::Response);
        assert_eq!(response.recipient, request.sender);
        assert_eq!(response.correlation_id, request.correlation_id);
    }

    #[test]
    fn test_message_signing() {
        let crypto = CryptoSuite::new();
        let sender = DIDKey::new(&crypto);
        let intent = SemanticIntent::new(&sender, IntentDomain::Compute, "process");
        let mut msg = IntentMessage::request(intent, "did:hido:recipient");

        msg.sign(|data| crypto.sign(data)).unwrap();
        assert!(msg.signature.is_some());
        assert!(msg.verify(|data, sig| crypto.verify(data, sig)).is_ok());
    }

    #[test]
    fn test_echo_protocol() {
        let sender = create_test_did();
        let intent = SemanticIntent::new(&sender, IntentDomain::Communication, "echo");
        let request = IntentMessage::request(intent, "did:hido:echo-agent");

        let protocol = EchoProtocol::new("did:hido:echo-agent");
        let response = protocol.handle(request).unwrap().unwrap();

        assert_eq!(response.message_type, MessageType::Response);
        assert!(response.payload.is_some());
    }

    #[test]
    fn test_message_serialization() {
        let sender = create_test_did();
        let intent = SemanticIntent::new(&sender, IntentDomain::Data, "read");
        let msg = IntentMessage::request(intent, "did:hido:recipient");

        let json = msg.to_json().unwrap();
        let parsed = IntentMessage::from_json(&json).unwrap();

        assert_eq!(parsed.id, msg.id);
        assert_eq!(parsed.message_type, msg.message_type);
    }
}
