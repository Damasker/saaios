//! Wire protocol for the ADR-020 portal socket: `saai-shell` mediates
//! `clipboard.read`/`clipboard.write` and `portal.open_file` on behalf of
//! sandboxed apps (ADR-020 section 8), the same way `saai-app-protocol`
//! mediates `saai-appd` and `saai-entity-protocol` mediates `saai-entityd`.
//!
//! S07's Change 7 is explicitly scoped as an entry point, not a finished
//! portal system (see the S07 sprint doc's Scope section) -- `open_file`
//! is a real, typed request an app can send and get a real, typed
//! `not_implemented` error back for, proving the routing and the
//! capability check both already exist, without pretending a file-picker
//! UI exists yet. `clipboard.read`/`clipboard.write` are the one complete
//! end-to-end scenario: a real round trip through a real in-memory
//! clipboard, gated by the same capability check.

use std::io;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const PORTAL_WIRE_SCHEMA_V1: u32 = 1;
pub const MAX_WIRE_MESSAGE_BYTES: usize = 64 * 1024;
const MAX_REQUEST_ID_BYTES: usize = 64;

/// Caps how much text one `clipboard.write` can store. Generous enough for
/// any realistic paste; small enough that a misbehaving (or malicious, once
/// third-party apps exist) app can't use the portal to pin arbitrary
/// amounts of memory inside `saai-shell`, which is not sandboxed itself.
pub const MAX_CLIPBOARD_TEXT_BYTES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientRequest {
    ClipboardRead {
        schema: u32,
        request_id: String,
    },
    ClipboardWrite {
        schema: u32,
        request_id: String,
        text: String,
    },
    /// Entry point only (ADR-020 section 8, S07 Change 7): `saai-shell`
    /// always answers this with a `not_implemented` error today. Picking a
    /// real file and bind-mounting it into the requesting app's sandbox for
    /// the session is a separate flow this version does not build -- this
    /// variant exists so the request/response routing and the capability
    /// check already have a place for it once that flow is designed.
    OpenFile {
        schema: u32,
        request_id: String,
    },
}

impl ClientRequest {
    pub fn schema(&self) -> u32 {
        match self {
            Self::ClipboardRead { schema, .. }
            | Self::ClipboardWrite { schema, .. }
            | Self::OpenFile { schema, .. } => *schema,
        }
    }

    pub fn request_id(&self) -> &str {
        match self {
            Self::ClipboardRead { request_id, .. }
            | Self::ClipboardWrite { request_id, .. }
            | Self::OpenFile { request_id, .. } => request_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum ResponseResult {
    ClipboardText { text: String },
    ClipboardWritten,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Response {
        schema: u32,
        request_id: String,
        ok: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        result: Option<ResponseResult>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<WireError>,
    },
}

impl ServerMessage {
    pub fn success(request_id: impl Into<String>, result: ResponseResult) -> Self {
        Self::Response {
            schema: PORTAL_WIRE_SCHEMA_V1,
            request_id: request_id.into(),
            ok: true,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(
        request_id: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::Response {
            schema: PORTAL_WIRE_SCHEMA_V1,
            request_id: request_id.into(),
            ok: false,
            result: None,
            error: Some(WireError {
                code: code.into(),
                message: message.into(),
            }),
        }
    }
}

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("wire message exceeds the {MAX_WIRE_MESSAGE_BYTES}-byte limit")]
    MessageTooLarge,
    #[error("wire message is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("wire transport failed: {0}")]
    Io(#[from] io::Error),
    #[error("unsupported wire schema {0}")]
    UnsupportedSchema(u32),
    #[error("request_id must contain 1..={MAX_REQUEST_ID_BYTES} safe ASCII bytes")]
    InvalidRequestId,
}

pub fn decode_request(line: &[u8]) -> Result<ClientRequest, ProtocolError> {
    if line.len() > MAX_WIRE_MESSAGE_BYTES {
        return Err(ProtocolError::MessageTooLarge);
    }
    let request: ClientRequest = serde_json::from_slice(line)?;
    if request.schema() != PORTAL_WIRE_SCHEMA_V1 {
        return Err(ProtocolError::UnsupportedSchema(request.schema()));
    }
    if !valid_request_id(request.request_id()) {
        return Err(ProtocolError::InvalidRequestId);
    }
    Ok(request)
}

pub fn encode_message(message: &ServerMessage) -> Result<Vec<u8>, ProtocolError> {
    let mut encoded = serde_json::to_vec(message)?;
    if encoded.len() > MAX_WIRE_MESSAGE_BYTES {
        return Err(ProtocolError::MessageTooLarge);
    }
    encoded.push(b'\n');
    Ok(encoded)
}

pub fn encode_request(request: &ClientRequest) -> Result<Vec<u8>, ProtocolError> {
    let mut encoded = serde_json::to_vec(request)?;
    if encoded.len() > MAX_WIRE_MESSAGE_BYTES {
        return Err(ProtocolError::MessageTooLarge);
    }
    encoded.push(b'\n');
    Ok(encoded)
}

fn valid_request_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_REQUEST_ID_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

#[cfg(test)]
mod tests {
    use super::{
        decode_request, encode_message, encode_request, ClientRequest, ProtocolError,
        ResponseResult, ServerMessage, PORTAL_WIRE_SCHEMA_V1,
    };

    #[test]
    fn decodes_each_v1_command() {
        let fixtures = [
            r#"{"schema":1,"request_id":"r1","command":"clipboard_read"}"#,
            r#"{"schema":1,"request_id":"r2","command":"clipboard_write","text":"hi"}"#,
            r#"{"schema":1,"request_id":"r3","command":"open_file"}"#,
        ];
        for fixture in fixtures {
            assert_eq!(decode_request(fixture.as_bytes()).unwrap().schema(), 1);
        }
        assert!(matches!(
            decode_request(fixtures[0].as_bytes()).unwrap(),
            ClientRequest::ClipboardRead { .. }
        ));
    }

    #[test]
    fn rejects_unknown_schema_field_command_and_bad_request_id() {
        assert!(matches!(
            decode_request(br#"{"schema":2,"request_id":"r","command":"clipboard_read"}"#),
            Err(ProtocolError::UnsupportedSchema(2))
        ));
        for fixture in [
            br#"{"schema":1,"request_id":"r","command":"clipboard_read","extra":true}"#.as_slice(),
            br#"{"schema":1,"request_id":"r","command":"purge"}"#.as_slice(),
            br#"{"schema":1,"request_id":"bad id","command":"clipboard_read"}"#.as_slice(),
        ] {
            assert!(decode_request(fixture).is_err());
        }
    }

    #[test]
    fn rejects_oversized_message_before_json_parse() {
        let oversized = vec![b' '; super::MAX_WIRE_MESSAGE_BYTES + 1];
        assert!(matches!(
            decode_request(&oversized),
            Err(ProtocolError::MessageTooLarge)
        ));
    }

    #[test]
    fn response_encodes_as_one_json_line_and_round_trips() {
        let response = ServerMessage::success(
            "r1",
            ResponseResult::ClipboardText {
                text: "hello".into(),
            },
        );
        let encoded = encode_message(&response).unwrap();
        assert_eq!(encoded.last(), Some(&b'\n'));
        let decoded: ServerMessage = serde_json::from_slice(&encoded[..encoded.len() - 1]).unwrap();
        assert_eq!(decoded, response);

        let error = ServerMessage::error("r2", "capability_denied", "no clipboard.read");
        let encoded = encode_message(&error).unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(&encoded[..encoded.len() - 1]).unwrap();
        assert_eq!(json["schema"], PORTAL_WIRE_SCHEMA_V1);
        assert_eq!(json["ok"], false);
        assert_eq!(json["error"]["code"], "capability_denied");
    }

    #[test]
    fn request_encodes_as_one_json_line() {
        let request = ClientRequest::ClipboardWrite {
            schema: PORTAL_WIRE_SCHEMA_V1,
            request_id: "app:1".into(),
            text: "copied text".into(),
        };
        let encoded = encode_request(&request).unwrap();
        assert_eq!(encoded.last(), Some(&b'\n'));
        assert_eq!(
            decode_request(&encoded[..encoded.len() - 1]).unwrap(),
            request
        );
    }
}
