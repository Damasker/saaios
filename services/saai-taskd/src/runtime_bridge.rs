//! ADR-033: bridges a free-form `saaios.intent` to the already-deployed,
//! already-running `saaios-runtime` (Platform Track) over its existing
//! one-request-per-connection JSON protocol. Deliberately does not
//! depend on `protocol`/`ai-runtime`/`model-provider` -- only the wire
//! JSON shape is shared, per ADR-030's no-cross-runtime-dependency
//! principle. `saaios-runtime`'s own `main.rs` is the source of truth
//! this was read against, not guessed at.

use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use uuid::Uuid;

/// Slightly above panther's 60s runtime budget so a live timeout still
/// arrives as `ok:false` from `saaios-runtime`. Hung TCP cannot leave
/// a Task `Pending` forever (ADR-236).
pub const DIAGNOSE_TIMEOUT: Duration = Duration::from_secs(65);
/// WORLD-05: status is a local snapshot, not a model call.
const STATUS_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("saaios-runtime unreachable at {addr}: {source}")]
    Connect {
        addr: String,
        #[source]
        source: std::io::Error,
    },
    #[error("saaios-runtime io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("malformed saaios-runtime response: {0}")]
    Json(#[from] serde_json::Error),
    #[error("saaios-runtime diagnose timed out after {secs}s at {addr}")]
    Timeout { addr: String, secs: u64 },
}

impl BridgeError {
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout { .. })
    }

    /// Timeout and connect failures are retryable. Malformed JSON is not.
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Timeout { .. } | Self::Connect { .. })
    }
}

/// Runtime's own budget surfaces as `ok:false` with this substring
/// (`crates/ai-runtime` "request timed out after Ns").
pub fn runtime_error_is_timeout(message: &str) -> bool {
    message.contains("timed out after")
}

/// Mirrors `saaios-runtime`'s `PendingDto` -- a proposed, not yet
/// executed, tool call. Its presence is the entire `WorkflowStatus`
/// signal (ADR-033): `WaitingConfirmation` if this is `Some`, `Done`
/// immediately if `None`.
#[derive(Debug, Clone, Deserialize)]
pub struct Pending {
    pub call_id: Uuid,
    pub tool: String,
    pub arguments: Value,
    pub summary: String,
}

#[derive(Debug, Deserialize)]
struct DiagnoseDto {
    summary: String,
}

#[derive(Debug, Deserialize)]
struct ToolResultDto {
    ok: bool,
    #[serde(default)]
    output: Value,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LiveObservation {
    pub key: String,
    pub value: Value,
}

#[derive(Debug, Deserialize)]
struct StatusDto {
    #[serde(default)]
    observations: Vec<LiveObservation>,
}

/// The subset of `saaios-runtime`'s `ClientResponse` this bridge reads.
/// Extra fields on the wire (`session_grants`, `progress`, ...) are
/// silently ignored -- no `deny_unknown_fields` -- since this bridge
/// only ever needs the outcome, not the model's intermediate trace.
#[derive(Debug, Deserialize)]
pub struct RuntimeResponse {
    pub ok: bool,
    pub correlation_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    diagnose: Option<DiagnoseDto>,
    pub pending: Option<Pending>,
    pub error: Option<String>,
    tool_result: Option<ToolResultDto>,
    #[serde(default)]
    status: Option<StatusDto>,
}

impl RuntimeResponse {
    pub fn summary(&self) -> Option<&str> {
        self.diagnose.as_ref().map(|d| d.summary.as_str())
    }

    pub fn tool_output(&self) -> Option<(bool, &Value, Option<&str>)> {
        self.tool_result
            .as_ref()
            .map(|r| (r.ok, &r.output, r.error.as_deref()))
    }

    pub fn observations(&self) -> &[LiveObservation] {
        self.status
            .as_ref()
            .map(|status| status.observations.as_slice())
            .unwrap_or(&[])
    }
}

async fn call(addr: &str, request: &Value) -> Result<RuntimeResponse, BridgeError> {
    call_with_timeout(addr, request, DIAGNOSE_TIMEOUT).await
}

async fn call_with_timeout(
    addr: &str,
    request: &Value,
    timeout: Duration,
) -> Result<RuntimeResponse, BridgeError> {
    let fut = async {
        let mut stream = TcpStream::connect(addr)
            .await
            .map_err(|source| BridgeError::Connect {
                addr: addr.to_string(),
                source,
            })?;
        let bytes = serde_json::to_vec(request)?;
        stream.write_all(&bytes).await?;
        stream.shutdown().await?;
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await?;
        Ok(serde_json::from_slice(&buf)?)
    };
    match tokio::time::timeout(timeout, fut).await {
        Ok(inner) => inner,
        Err(_) => Err(BridgeError::Timeout {
            addr: addr.to_string(),
            secs: timeout.as_secs().max(1),
        }),
    }
}

/// `{"op":"diagnose", "text": ..., "stream": false}` -- the one call
/// that turns free-form Intent text into either a finished answer or a
/// paused proposal, physically confirmed against the real on-device
/// runtime in ADR-033.
/// `space_id` (ADR-038) is this daemon's own already-known space
/// (`--space`, ADR-030) -- the one caller of this bridge that ever
/// knows one at all. Threaded through so `saaios-runtime`'s memory
/// tools can keep a fact remembered from one space out of another's
/// recall.
pub async fn diagnose(
    addr: &str,
    text: &str,
    space_id: &str,
) -> Result<RuntimeResponse, BridgeError> {
    diagnose_with_timeout(addr, text, space_id, DIAGNOSE_TIMEOUT).await
}

pub async fn diagnose_with_timeout(
    addr: &str,
    text: &str,
    space_id: &str,
    timeout: Duration,
) -> Result<RuntimeResponse, BridgeError> {
    call_with_timeout(
        addr,
        &json!({ "op": "diagnose", "text": text, "stream": false, "space_id": space_id }),
        timeout,
    )
    .await
}

/// WORLD-05: Fresh Observation rows from `{"op":"status"}`. Stale rows
/// are already omitted by runtime (ADR-246). Connect failure is the
/// caller's problem — stay Verifying, do not invent evidence.
pub async fn status(addr: &str) -> Result<RuntimeResponse, BridgeError> {
    call_with_timeout(addr, &json!({ "op": "status" }), STATUS_TIMEOUT).await
}

/// `{"op":"confirm", ...}` -- resumes (or cancels) exactly the call a
/// prior `diagnose()` left pending. `confirmed: false` is sent on
/// decline too (not just silently dropped) so `saaios-runtime`'s own
/// pending state doesn't dangle -- physically confirmed in ADR-033's
/// spike to produce a clean `"user cancelled"` tool_result.
pub async fn confirm(
    addr: &str,
    correlation_id: Uuid,
    session_id: Option<Uuid>,
    call_id: Uuid,
    tool: &str,
    arguments: Value,
    confirmed: bool,
) -> Result<RuntimeResponse, BridgeError> {
    confirm_as_worker(
        addr,
        correlation_id,
        session_id,
        call_id,
        tool,
        arguments,
        confirmed,
        None,
    )
    .await
}

/// AUTH-06: `execution_id` is the Task that will present the worker
/// envelope. Absent keeps the owner grant path for shell/runtime
/// confirm without a Task.
pub async fn confirm_as_worker(
    addr: &str,
    correlation_id: Uuid,
    session_id: Option<Uuid>,
    call_id: Uuid,
    tool: &str,
    arguments: Value,
    confirmed: bool,
    execution_id: Option<Uuid>,
) -> Result<RuntimeResponse, BridgeError> {
    let scope = if confirmed { "once" } else { "cancel" };
    let mut body = json!({
        "op": "confirm",
        "correlation_id": correlation_id,
        "call_id": call_id,
        "tool": tool,
        "arguments": arguments,
        "scope": scope,
        "confirmed": confirmed,
        "session_id": session_id,
    });
    if let Some(execution_id) = execution_id {
        body["execution_id"] = json!(execution_id);
    }
    call(addr, &body).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    /// A one-shot fake `saaios-runtime`: accepts one connection, reads
    /// until the peer half-closes (matching `read_client_request`'s own
    /// "read until valid JSON" shape on the real server, simplified to
    /// "read until EOF" since these tests always send a complete,
    /// well-formed request in one write), then writes back a fixed
    /// response and closes.
    fn with_fake_runtime(response_json: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            stream.read_to_end(&mut request).unwrap();
            stream.write_all(response_json.as_bytes()).unwrap();
        });
        addr
    }

    #[tokio::test]
    async fn diagnose_parses_an_immediate_answer_with_no_pending() {
        let addr = with_fake_runtime(
            r#"{"ok":true,"correlation_id":"550e8400-e29b-41d4-a716-446655440000","session_id":null,"diagnose":{"summary":"42 GB free","culprit_pid":null,"culprit_name":null,"proposed_action":null},"pending":null,"error":null,"tool_result":null}"#,
        );
        let response = diagnose(&addr, "how much disk space?", "home")
            .await
            .unwrap();
        assert!(response.ok);
        assert!(response.pending.is_none());
        assert_eq!(response.summary(), Some("42 GB free"));
    }

    #[tokio::test]
    async fn diagnose_sends_its_space_id_on_the_wire() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            stream.read_to_end(&mut request).unwrap();
            tx.send(request).unwrap();
            stream
                .write_all(
                    br#"{"ok":true,"correlation_id":null,"session_id":null,"diagnose":null,"pending":null,"error":null,"tool_result":null}"#,
                )
                .unwrap();
        });
        diagnose(&addr, "hi", "work").await.unwrap();
        let sent = String::from_utf8(rx.recv().unwrap()).unwrap();
        let parsed: Value = serde_json::from_str(&sent).unwrap();
        assert_eq!(parsed["space_id"], "work");
    }

    #[tokio::test]
    async fn diagnose_parses_a_pending_dangerous_proposal() {
        let addr = with_fake_runtime(
            r#"{"ok":true,"correlation_id":"550e8400-e29b-41d4-a716-446655440000","session_id":"660e8400-e29b-41d4-a716-446655440000","diagnose":{"summary":"","culprit_pid":null,"culprit_name":null,"proposed_action":null},"pending":{"call_id":"770e8400-e29b-41d4-a716-446655440000","tool":"process.kill_request","arguments":{"pid":999},"summary":"Run process.kill_request?"},"error":null,"tool_result":null}"#,
        );
        let response = diagnose(&addr, "stop pid 999", "home").await.unwrap();
        let pending = response.pending.expect("expected a pending proposal");
        assert_eq!(pending.tool, "process.kill_request");
        assert_eq!(pending.arguments["pid"], json!(999));
    }

    #[tokio::test]
    async fn confirm_parses_a_cancelled_tool_result() {
        let addr = with_fake_runtime(
            r#"{"ok":true,"correlation_id":"550e8400-e29b-41d4-a716-446655440000","session_id":null,"diagnose":null,"pending":null,"error":null,"tool_result":{"call_id":"770e8400-e29b-41d4-a716-446655440000","tool":"process.kill_request","ok":false,"output":{},"error":"user cancelled"}}"#,
        );
        let response = confirm(
            &addr,
            Uuid::nil(),
            None,
            Uuid::nil(),
            "process.kill_request",
            json!({}),
            false,
        )
        .await
        .unwrap();
        let (ok, _output, error) = response.tool_output().expect("expected a tool_result");
        assert!(!ok);
        assert_eq!(error, Some("user cancelled"));
    }

    #[tokio::test]
    async fn confirm_as_worker_sends_execution_id_on_the_wire() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            stream.read_to_end(&mut request).unwrap();
            tx.send(request).unwrap();
            stream
                .write_all(
                    br#"{"ok":true,"correlation_id":null,"session_id":null,"diagnose":null,"pending":null,"error":null,"tool_result":null}"#,
                )
                .unwrap();
        });
        let execution_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        confirm_as_worker(
            &addr,
            Uuid::nil(),
            None,
            Uuid::nil(),
            "process.kill_request",
            json!({"pid": 4312}),
            true,
            Some(execution_id),
        )
        .await
        .unwrap();
        let sent = String::from_utf8(rx.recv().unwrap()).unwrap();
        let parsed: Value = serde_json::from_str(&sent).unwrap();
        assert_eq!(parsed["op"], "confirm");
        assert_eq!(parsed["execution_id"], execution_id.to_string());
        assert_eq!(parsed["scope"], "once");
    }

    #[tokio::test]
    async fn confirm_without_task_omits_execution_id() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            stream.read_to_end(&mut request).unwrap();
            tx.send(request).unwrap();
            stream
                .write_all(
                    br#"{"ok":true,"correlation_id":null,"session_id":null,"diagnose":null,"pending":null,"error":null,"tool_result":null}"#,
                )
                .unwrap();
        });
        confirm(
            &addr,
            Uuid::nil(),
            None,
            Uuid::nil(),
            "process.kill_request",
            json!({}),
            false,
        )
        .await
        .unwrap();
        let sent = String::from_utf8(rx.recv().unwrap()).unwrap();
        let parsed: Value = serde_json::from_str(&sent).unwrap();
        assert!(parsed.get("execution_id").is_none());
    }

    #[tokio::test]
    async fn unreachable_runtime_is_a_typed_connect_error() {
        // Bind to let the OS pick a free port, then drop the listener --
        // that exact port is refused immediately afterwards, without
        // depending on any real, potentially-present service being
        // absent (unlike guessing a "probably closed" fixed port).
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        drop(listener);

        let error = diagnose(&addr, "hello", "home").await.unwrap_err();
        assert!(matches!(error, BridgeError::Connect { .. }));
        assert!(error.is_retryable());
        assert!(!error.is_timeout());
    }

    #[tokio::test]
    async fn diagnose_times_out_instead_of_hanging() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        thread::spawn(move || {
            let (_stream, _) = listener.accept().unwrap();
            thread::sleep(Duration::from_secs(30));
        });
        let error = diagnose_with_timeout(&addr, "hi", "work", Duration::from_millis(80))
            .await
            .unwrap_err();
        assert!(error.is_timeout());
        assert!(error.is_retryable());
    }

    #[test]
    fn runtime_timeout_text_is_classified() {
        assert!(runtime_error_is_timeout(
            "request timed out after 60s (correlation_id=67d7c8ea-0000-0000-0000-000000000000)"
        ));
        assert!(!runtime_error_is_timeout("saaios-runtime unreachable"));
    }

    #[tokio::test]
    async fn status_parses_fresh_observation_rows() {
        let addr = with_fake_runtime(
            r#"{"ok":true,"status":{"observations":[{"key":"wifi.link","value":"up","source":"net","observed_at":"2026-09-21T00:00:00Z"}]}}"#,
        );
        let response = status(&addr).await.unwrap();
        assert!(response.ok);
        assert_eq!(response.observations().len(), 1);
        assert_eq!(response.observations()[0].key, "wifi.link");
        assert_eq!(response.observations()[0].value, json!("up"));
    }
}
