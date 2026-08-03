use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use thiserror::Error;
use uuid::Uuid;

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("unsupported protocol version {0}")]
    UnsupportedVersion(u32),
    #[error("cbor encode/decode error: {0}")]
    Cbor(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    UserRequest,
    AssistantMessage,
    ToolCall,
    ToolResult,
    PolicyDecision,
    ConfirmationRequest,
    ConfirmationResponse,
    Event,
    Error,
    DiagnoseResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Envelope {
    pub v: u32,
    pub msg_id: Uuid,
    pub correlation_id: Uuid,
    pub causation_id: Option<Uuid>,
    pub kind: MessageKind,
    pub ts: DateTime<Utc>,
    pub payload: serde_json::Value,
}

impl Envelope {
    pub fn new(
        kind: MessageKind,
        correlation_id: Uuid,
        causation_id: Option<Uuid>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            msg_id: Uuid::new_v4(),
            correlation_id,
            causation_id,
            kind,
            ts: Utc::now(),
            payload,
        }
    }

    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.v != PROTOCOL_VERSION {
            return Err(ProtocolError::UnsupportedVersion(self.v));
        }
        Ok(())
    }

    pub fn to_cbor(&self) -> Result<Vec<u8>, ProtocolError> {
        self.validate()?;
        let mut buf = Vec::new();
        ciborium::into_writer(self, &mut buf).map_err(|e| ProtocolError::Cbor(e.to_string()))?;
        Ok(buf)
    }

    pub fn from_cbor(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let env: Self = ciborium::from_reader(Cursor::new(bytes))
            .map_err(|e| ProtocolError::Cbor(e.to_string()))?;
        env.validate()?;
        Ok(env)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserRequest {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCallRequest {
    pub call_id: Uuid,
    pub tool: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCallResult {
    pub call_id: Uuid,
    pub tool: String,
    pub ok: bool,
    pub output: serde_json::Value,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PolicyVerdict {
    Allow,
    Deny,
    AskUser,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyDecisionRecord {
    pub call_id: Uuid,
    pub tool: String,
    pub verdict: PolicyVerdict,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfirmationRequest {
    pub call_id: Uuid,
    pub tool: String,
    pub summary: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmScope {
    Once,
    Session,
    Cancel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfirmationResponse {
    pub call_id: Uuid,
    pub confirmed: bool,
    #[serde(default = "default_once_scope")]
    pub scope: ConfirmScope,
}

fn default_once_scope() -> ConfirmScope {
    ConfirmScope::Once
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiagnoseResult {
    pub summary: String,
    pub culprit_pid: Option<u32>,
    pub culprit_name: Option<String>,
    pub proposed_action: Option<ProposedAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposedAction {
    pub tool: String,
    pub arguments: serde_json::Value,
    pub summary: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn cbor_roundtrip() {
        let corr = Uuid::new_v4();
        let env = Envelope::new(
            MessageKind::UserRequest,
            corr,
            None,
            json!({"text": "Почему тормозит?"}),
        );
        let bytes = env.to_cbor().unwrap();
        let decoded = Envelope::from_cbor(&bytes).unwrap();
        assert_eq!(decoded.v, PROTOCOL_VERSION);
        assert_eq!(decoded.correlation_id, corr);
        assert_eq!(decoded.kind, MessageKind::UserRequest);
    }

    #[test]
    fn rejects_bad_version() {
        let mut env = Envelope::new(
            MessageKind::Event,
            Uuid::new_v4(),
            None,
            json!({"ok": true}),
        );
        env.v = 99;
        let err = env.to_cbor().unwrap_err();
        assert!(matches!(err, ProtocolError::UnsupportedVersion(99)));
    }
}
