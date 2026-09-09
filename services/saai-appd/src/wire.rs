use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const APPD_WIRE_SCHEMA_V1: u32 = 1;
pub const MAX_WIRE_MESSAGE_BYTES: usize = 64 * 1024;
const MAX_REQUEST_ID_BYTES: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientRequest {
    List {
        schema: u32,
        request_id: String,
    },
    Install {
        schema: u32,
        request_id: String,
        package_path: PathBuf,
    },
    Launch {
        schema: u32,
        request_id: String,
        app_id: String,
    },
    Stop {
        schema: u32,
        request_id: String,
        app_id: String,
    },
    Remove {
        schema: u32,
        request_id: String,
        app_id: String,
    },
}

impl ClientRequest {
    pub fn schema(&self) -> u32 {
        match self {
            Self::List { schema, .. }
            | Self::Install { schema, .. }
            | Self::Launch { schema, .. }
            | Self::Stop { schema, .. }
            | Self::Remove { schema, .. } => *schema,
        }
    }

    pub fn request_id(&self) -> &str {
        match self {
            Self::List { request_id, .. }
            | Self::Install { request_id, .. }
            | Self::Launch { request_id, .. }
            | Self::Stop { request_id, .. }
            | Self::Remove { request_id, .. } => request_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    pub state: String,
    pub pids: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum ResponseResult {
    List {
        apps: Vec<AppSummary>,
    },
    Installed {
        app: AppSummary,
    },
    Launched {
        app_id: String,
        pid: u32,
        existing: bool,
    },
    Stopped {
        app_id: String,
        was_running: bool,
    },
    Removed {
        app_id: String,
        removed: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleEventKind {
    Installed,
    Running,
    Stopped,
    Crashed,
    CrashLimited,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleEvent {
    pub event: LifecycleEventKind,
    pub app_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
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
    Event {
        schema: u32,
        #[serde(flatten)]
        event: LifecycleEvent,
    },
}

impl ServerMessage {
    pub fn success(request_id: impl Into<String>, result: ResponseResult) -> Self {
        Self::Response {
            schema: APPD_WIRE_SCHEMA_V1,
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
            schema: APPD_WIRE_SCHEMA_V1,
            request_id: request_id.into(),
            ok: false,
            result: None,
            error: Some(WireError {
                code: code.into(),
                message: message.into(),
            }),
        }
    }

    pub fn event(event: LifecycleEvent) -> Self {
        Self::Event {
            schema: APPD_WIRE_SCHEMA_V1,
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
    if request.schema() != APPD_WIRE_SCHEMA_V1 {
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
        decode_request, encode_message, AppSummary, ClientRequest, LifecycleEvent,
        LifecycleEventKind, ProtocolError, ResponseResult, ServerMessage, APPD_WIRE_SCHEMA_V1,
        MAX_WIRE_MESSAGE_BYTES,
    };

    #[test]
    fn decodes_each_v1_command() {
        let fixtures = [
            r#"{"schema":1,"request_id":"r1","command":"list"}"#,
            r#"{"schema":1,"request_id":"r2","command":"install","package_path":"/tmp/demo"}"#,
            r#"{"schema":1,"request_id":"r3","command":"launch","app_id":"org.saaios.demo"}"#,
            r#"{"schema":1,"request_id":"r4","command":"stop","app_id":"org.saaios.demo"}"#,
            r#"{"schema":1,"request_id":"r5","command":"remove","app_id":"org.saaios.demo"}"#,
        ];
        for fixture in fixtures {
            assert_eq!(decode_request(fixture.as_bytes()).unwrap().schema(), 1);
        }
        assert!(matches!(
            decode_request(fixtures[0].as_bytes()).unwrap(),
            ClientRequest::List { .. }
        ));
    }

    #[test]
    fn rejects_unknown_schema_field_command_and_bad_request_id() {
        assert!(matches!(
            decode_request(br#"{"schema":2,"request_id":"r","command":"list"}"#),
            Err(ProtocolError::UnsupportedSchema(2))
        ));
        for fixture in [
            br#"{"schema":1,"request_id":"r","command":"list","extra":true}"#.as_slice(),
            br#"{"schema":1,"request_id":"r","command":"purge"}"#.as_slice(),
            br#"{"schema":1,"request_id":"bad id","command":"list"}"#.as_slice(),
        ] {
            assert!(decode_request(fixture).is_err());
        }
    }

    #[test]
    fn rejects_oversized_message_before_json_parse() {
        let oversized = vec![b' '; MAX_WIRE_MESSAGE_BYTES + 1];
        assert!(matches!(
            decode_request(&oversized),
            Err(ProtocolError::MessageTooLarge)
        ));
    }

    #[test]
    fn response_and_event_encode_as_one_json_line() {
        let response = ServerMessage::success(
            "r1",
            ResponseResult::List {
                apps: vec![AppSummary {
                    id: "org.saaios.demo".into(),
                    name: "Demo".into(),
                    version: "0.1.0".into(),
                    state: "installed".into(),
                    pids: vec![],
                }],
            },
        );
        let encoded = encode_message(&response).unwrap();
        assert_eq!(encoded.last(), Some(&b'\n'));
        let decoded: ServerMessage = serde_json::from_slice(&encoded[..encoded.len() - 1]).unwrap();
        assert_eq!(decoded, response);

        let event = ServerMessage::event(LifecycleEvent {
            event: LifecycleEventKind::Running,
            app_id: "org.saaios.demo".into(),
            pid: Some(42),
            exit_code: None,
        });
        let encoded = encode_message(&event).unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(&encoded[..encoded.len() - 1]).unwrap();
        assert_eq!(json["schema"], APPD_WIRE_SCHEMA_V1);
        assert_eq!(json["type"], "event");
        assert_eq!(json["event"], "running");
    }
}
