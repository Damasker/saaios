//! SaaiOS Stage — local web console (AI-native surface over the UDS runtime API).
use anyhow::{Context, Result};
use axum::body::Body;
use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use clap::Parser;
use futures::stream;
use protocol::ConfirmScope;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::sync::Mutex;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, Parser)]
#[command(name = "saaios-stage", about = "SaaiOS Stage web console")]
struct Args {
    #[arg(long, default_value = "/tmp/saaios.sock", env = "SAAIOS_SOCK")]
    sock: PathBuf,

    #[arg(long, default_value = "127.0.0.1:7420", env = "SAAIOS_STAGE_BIND")]
    bind: String,
}

#[derive(Clone)]
struct AppState {
    sock: PathBuf,
    /// Browser session → runtime chat session.
    chat_session: Arc<Mutex<Option<Uuid>>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum ClientRequest {
    Diagnose {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        session_id: Option<Uuid>,
        stream: bool,
    },
    Confirm {
        correlation_id: Uuid,
        call_id: Uuid,
        tool: String,
        arguments: serde_json::Value,
        scope: ConfirmScope,
        #[serde(skip_serializing_if = "Option::is_none")]
        session_id: Option<Uuid>,
    },
    ChatReset {
        session_id: Uuid,
    },
    Status,
    EventsTail {
        limit: usize,
    },
    SessionGrants,
    ClearSessionGrants,
    Ping,
}

#[derive(Debug, Deserialize)]
struct DiagnoseBody {
    text: String,
}

#[derive(Debug, Deserialize)]
struct ConfirmBody {
    correlation_id: Uuid,
    call_id: Uuid,
    tool: String,
    arguments: serde_json::Value,
    scope: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with_target(false)
        .init();

    let args = Args::parse();
    let state = AppState {
        sock: args.sock.clone(),
        chat_session: Arc::new(Mutex::new(None)),
    };

    match uds_request(&args.sock, &ClientRequest::Ping).await {
        Ok(_) => info!(sock = %args.sock.display(), "runtime reachable"),
        Err(e) => warn!(error = %e, "runtime not reachable yet — start with `just run-mock`"),
    }

    let app = Router::new()
        .route("/", get(index))
        .route("/app.css", get(app_css))
        .route("/app.js", get(app_js))
        .route("/api/status", get(api_status))
        .route("/api/events", get(api_events))
        .route("/api/grants", get(api_grants))
        .route("/api/grants/clear", post(api_clear_grants))
        .route("/api/chat/reset", post(api_chat_reset))
        .route("/api/diagnose", post(api_diagnose_stream))
        .route("/api/confirm", post(api_confirm))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr: SocketAddr = args.bind.parse().context("parse bind address")?;
    info!(%addr, "SaaiOS Stage listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

async fn app_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("../static/app.css"),
    )
}

async fn app_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/javascript; charset=utf-8")],
        include_str!("../static/app.js"),
    )
}

async fn api_status(State(state): State<AppState>) -> Response {
    match uds_request(&state.sock, &ClientRequest::Status).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_json(StatusCode::BAD_GATEWAY, e),
    }
}

async fn api_events(State(state): State<AppState>) -> Response {
    match uds_request(&state.sock, &ClientRequest::EventsTail { limit: 24 }).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_json(StatusCode::BAD_GATEWAY, e),
    }
}

async fn api_grants(State(state): State<AppState>) -> Response {
    match uds_request(&state.sock, &ClientRequest::SessionGrants).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_json(StatusCode::BAD_GATEWAY, e),
    }
}

async fn api_clear_grants(State(state): State<AppState>) -> Response {
    match uds_request(&state.sock, &ClientRequest::ClearSessionGrants).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_json(StatusCode::BAD_GATEWAY, e),
    }
}

async fn api_chat_reset(State(state): State<AppState>) -> Response {
    let mut guard = state.chat_session.lock().await;
    if let Some(sid) = *guard {
        let _ = uds_request(&state.sock, &ClientRequest::ChatReset { session_id: sid }).await;
    }
    *guard = None;
    Json(serde_json::json!({ "ok": true })).into_response()
}

async fn api_confirm(State(state): State<AppState>, Json(body): Json<ConfirmBody>) -> Response {
    let scope = match body.scope.as_str() {
        "session" | "s" => ConfirmScope::Session,
        "cancel" | "n" => ConfirmScope::Cancel,
        _ => ConfirmScope::Once,
    };
    let session_id = *state.chat_session.lock().await;
    match uds_request(
        &state.sock,
        &ClientRequest::Confirm {
            correlation_id: body.correlation_id,
            call_id: body.call_id,
            tool: body.tool,
            arguments: body.arguments,
            scope,
            session_id,
        },
    )
    .await
    {
        Ok(v) => Json(v).into_response(),
        Err(e) => error_json(StatusCode::BAD_GATEWAY, e),
    }
}

/// NDJSON stream: progress frames + final done (mirrors UDS stream=true).
async fn api_diagnose_stream(
    State(state): State<AppState>,
    Json(body): Json<DiagnoseBody>,
) -> Response {
    let text = body.text.trim().to_string();
    if text.is_empty() {
        return error_json(StatusCode::BAD_REQUEST, anyhow::anyhow!("empty text"));
    }
    let sock = state.sock.clone();
    let session_slot = state.chat_session.clone();
    let session_id = *session_slot.lock().await;

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<bytes::Bytes, std::io::Error>>(64);
    tokio::spawn(async move {
        if let Err(e) = forward_diagnose_stream(&sock, &text, session_id, &session_slot, &tx).await
        {
            let err = serde_json::json!({
                "type": "done",
                "ok": false,
                "error": e.to_string(),
            });
            let line = serde_json::to_string(&err).unwrap_or_default() + "\n";
            let _ = tx.send(Ok(bytes::Bytes::from(line))).await;
        }
    });

    let body_stream = stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|item| (item, rx))
    });

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/x-ndjson; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-cache")
        .body(Body::from_stream(body_stream))
        .unwrap_or_else(|_| {
            error_json(
                StatusCode::INTERNAL_SERVER_ERROR,
                anyhow::anyhow!("failed to build stream body"),
            )
        })
}

fn error_json(status: StatusCode, err: anyhow::Error) -> Response {
    (
        status,
        Json(serde_json::json!({ "ok": false, "error": err.to_string() })),
    )
        .into_response()
}

async fn uds_request(sock: &PathBuf, req: &ClientRequest) -> Result<serde_json::Value> {
    let mut stream = UnixStream::connect(sock)
        .await
        .with_context(|| format!("connect {}", sock.display()))?;
    let bytes = serde_json::to_vec(req)?;
    stream.write_all(&bytes).await?;
    stream.shutdown().await?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await?;
    let value: serde_json::Value = serde_json::from_slice(&buf)?;
    Ok(value)
}

async fn forward_diagnose_stream(
    sock: &PathBuf,
    text: &str,
    session_id: Option<Uuid>,
    session_slot: &Arc<Mutex<Option<Uuid>>>,
    tx: &tokio::sync::mpsc::Sender<Result<bytes::Bytes, std::io::Error>>,
) -> Result<()> {
    let mut stream = UnixStream::connect(sock)
        .await
        .with_context(|| format!("connect {}", sock.display()))?;
    let req = ClientRequest::Diagnose {
        text: text.to_string(),
        session_id,
        stream: true,
    };
    let bytes = serde_json::to_vec(&req)?;
    stream.write_all(&bytes).await?;
    stream.shutdown().await?;

    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    let mut saw_frame = false;
    loop {
        let n = stream.read(&mut tmp).await?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        while let Some(pos) = buf.iter().position(|b| *b == b'\n') {
            let line_bytes = buf.drain(..=pos).collect::<Vec<_>>();
            let line = String::from_utf8_lossy(&line_bytes);
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            saw_frame = true;
            if let Ok(frame) = serde_json::from_str::<serde_json::Value>(line) {
                if frame.get("type").and_then(|v| v.as_str()) == Some("done") {
                    if let Some(sid) = frame
                        .get("session_id")
                        .and_then(|v| v.as_str())
                        .and_then(|s| Uuid::parse_str(s).ok())
                    {
                        *session_slot.lock().await = Some(sid);
                    }
                }
            }
            let out = format!("{line}\n");
            if tx.send(Ok(bytes::Bytes::from(out))).await.is_err() {
                return Ok(());
            }
        }
    }
    if !buf.is_empty() {
        let line = String::from_utf8_lossy(&buf);
        let line = line.trim();
        if !line.is_empty() {
            saw_frame = true;
            let out = format!("{line}\n");
            let _ = tx.send(Ok(bytes::Bytes::from(out))).await;
        }
    }
    if !saw_frame {
        anyhow::bail!("empty stream from runtime");
    }
    Ok(())
}
