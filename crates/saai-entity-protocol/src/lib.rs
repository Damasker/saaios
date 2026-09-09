pub use saai_entity_store::{Entity, Event, Space, SpaceSelection};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::io;
use thiserror::Error;
use uuid::Uuid;

pub const ENTITYD_WIRE_SCHEMA_V1: u32 = 1;
pub const MAX_WIRE_MESSAGE_BYTES: usize = 128 * 1024;
const MAX_REQUEST_ID_BYTES: usize = 64;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientRequest {
    ListSpaces {
        schema: u32,
        request_id: String,
    },
    GetSelection {
        schema: u32,
        request_id: String,
    },
    SelectSpace {
        schema: u32,
        request_id: String,
        space_id: String,
    },
    ListEntities {
        schema: u32,
        request_id: String,
        space_id: String,
    },
    CreateEntity {
        schema: u32,
        request_id: String,
        space_id: String,
        entity_type: String,
        title: String,
        #[serde(default)]
        properties: Map<String, Value>,
    },
    UpdateEntity {
        schema: u32,
        request_id: String,
        space_id: String,
        entity_id: Uuid,
        expected_revision: u64,
        entity_type: String,
        title: String,
        #[serde(default)]
        properties: Map<String, Value>,
    },
    DeleteEntity {
        schema: u32,
        request_id: String,
        space_id: String,
        entity_id: Uuid,
        expected_revision: u64,
    },
    Subscribe {
        schema: u32,
        request_id: String,
    },
}

impl ClientRequest {
    pub fn schema(&self) -> u32 {
        match self {
            Self::ListSpaces { schema, .. }
            | Self::GetSelection { schema, .. }
            | Self::SelectSpace { schema, .. }
            | Self::ListEntities { schema, .. }
            | Self::CreateEntity { schema, .. }
            | Self::UpdateEntity { schema, .. }
            | Self::DeleteEntity { schema, .. }
            | Self::Subscribe { schema, .. } => *schema,
        }
    }

    pub fn request_id(&self) -> &str {
        match self {
            Self::ListSpaces { request_id, .. }
            | Self::GetSelection { request_id, .. }
            | Self::SelectSpace { request_id, .. }
            | Self::ListEntities { request_id, .. }
            | Self::CreateEntity { request_id, .. }
            | Self::UpdateEntity { request_id, .. }
            | Self::DeleteEntity { request_id, .. }
            | Self::Subscribe { request_id, .. } => request_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResponseResult {
    Spaces {
        spaces: Vec<Space>,
    },
    Selection {
        selection: SpaceSelection,
    },
    Entities {
        space_id: String,
        entities: Vec<Entity>,
    },
    Entity {
        entity: Entity,
        event: Event,
    },
    Deleted {
        space_id: String,
        entity_id: Uuid,
        revision: u64,
        event: Event,
    },
    Subscribed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EntitydEvent {
    EntityChanged { record: Event },
    SelectionChanged { selection: SpaceSelection },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ServerMessage {
    Response {
        schema: u32,
        request_id: String,
        ok: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        result: Option<Box<ResponseResult>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<WireError>,
    },
    Event {
        schema: u32,
        event: EntitydEvent,
    },
}

impl ServerMessage {
    pub fn success(request_id: impl Into<String>, result: ResponseResult) -> Self {
        Self::Response {
            schema: ENTITYD_WIRE_SCHEMA_V1,
            request_id: request_id.into(),
            ok: true,
            result: Some(Box::new(result)),
            error: None,
        }
    }

    pub fn error(
        request_id: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::Response {
            schema: ENTITYD_WIRE_SCHEMA_V1,
            request_id: request_id.into(),
            ok: false,
            result: None,
            error: Some(WireError {
                code: code.into(),
                message: message.into(),
            }),
        }
    }

    pub fn event(event: EntitydEvent) -> Self {
        Self::Event {
            schema: ENTITYD_WIRE_SCHEMA_V1,
            event,
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
    if request.schema() != ENTITYD_WIRE_SCHEMA_V1 {
        return Err(ProtocolError::UnsupportedSchema(request.schema()));
    }
    if !valid_request_id(request.request_id()) {
        return Err(ProtocolError::InvalidRequestId);
    }
    Ok(request)
}

pub fn encode_request(request: &ClientRequest) -> Result<Vec<u8>, ProtocolError> {
    encode_line(request)
}

pub fn encode_message(message: &ServerMessage) -> Result<Vec<u8>, ProtocolError> {
    encode_line(message)
}

fn encode_line<T: Serialize>(value: &T) -> Result<Vec<u8>, ProtocolError> {
    let mut encoded = serde_json::to_vec(value)?;
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
    use super::*;

    #[test]
    fn all_commands_decode_with_explicit_space_scope() {
        let fixtures = [
            r#"{"schema":1,"request_id":"1","command":"list_spaces"}"#,
            r#"{"schema":1,"request_id":"2","command":"get_selection"}"#,
            r#"{"schema":1,"request_id":"3","command":"select_space","space_id":"home"}"#,
            r#"{"schema":1,"request_id":"4","command":"list_entities","space_id":"work"}"#,
            r#"{"schema":1,"request_id":"5","command":"create_entity","space_id":"home","entity_type":"task.item","title":"test"}"#,
            r#"{"schema":1,"request_id":"6","command":"update_entity","space_id":"home","entity_id":"00000000-0000-0000-0000-000000000001","expected_revision":1,"entity_type":"task.item","title":"test"}"#,
            r#"{"schema":1,"request_id":"7","command":"delete_entity","space_id":"home","entity_id":"00000000-0000-0000-0000-000000000001","expected_revision":1}"#,
            r#"{"schema":1,"request_id":"8","command":"subscribe"}"#,
        ];
        for fixture in fixtures {
            assert_eq!(decode_request(fixture.as_bytes()).unwrap().schema(), 1);
        }
    }

    #[test]
    fn malformed_boundary_is_rejected() {
        for fixture in [
            br#"{"schema":2,"request_id":"1","command":"list_spaces"}"#.as_slice(),
            br#"{"schema":1,"request_id":"bad id","command":"list_spaces"}"#.as_slice(),
            br#"{"schema":1,"request_id":"1","command":"list_spaces","extra":true}"#.as_slice(),
            br#"{"schema":1,"request_id":"1","command":"list_entities"}"#.as_slice(),
        ] {
            assert!(decode_request(fixture).is_err());
        }
        assert!(matches!(
            decode_request(&vec![b' '; MAX_WIRE_MESSAGE_BYTES + 1]),
            Err(ProtocolError::MessageTooLarge)
        ));
    }

    #[test]
    fn success_error_and_event_are_single_json_lines() {
        let success = ServerMessage::success("1", ResponseResult::Spaces { spaces: vec![] });
        let error = ServerMessage::error("2", "invalid", "rejected");
        for message in [success, error] {
            let encoded = encode_message(&message).unwrap();
            assert_eq!(encoded.last(), Some(&b'\n'));
            let decoded: ServerMessage =
                serde_json::from_slice(&encoded[..encoded.len() - 1]).unwrap();
            assert_eq!(decoded, message);
        }
    }
}
