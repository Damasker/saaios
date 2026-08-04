//! SaaiOS Stage — local web console (AI-native surface over the UDS runtime API).
use anyhow::{anyhow, Context, Result};
use axum::body::Body;
use axum::extract::{Query, Request, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::{from_fn_with_state, Next};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_server::tls_rustls::RustlsConfig;
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

    /// Listen address (overridden by --lan).
    #[arg(long, default_value = "127.0.0.1:7420", env = "SAAIOS_STAGE_BIND")]
    bind: String,

    /// Bind 0.0.0.0:7420 for LAN/kiosk access (requires --token unless --allow-open).
    #[arg(long, env = "SAAIOS_STAGE_LAN")]
    lan: bool,

    /// Port used with --lan (default 7420).
    #[arg(long, default_value_t = 7420, env = "SAAIOS_STAGE_PORT")]
    port: u16,

    /// Shared access token for LAN / remote browsers (Authorization: Bearer … or ?token=).
    #[arg(long, env = "SAAIOS_STAGE_TOKEN")]
    token: Option<String>,

    /// Allow --lan without a token (insecure; for lab networks only).
    #[arg(long, env = "SAAIOS_STAGE_ALLOW_OPEN")]
    allow_open: bool,

    /// TLS certificate PEM (enables HTTPS when paired with --tls-key).
    #[arg(long, env = "SAAIOS_STAGE_TLS_CERT")]
    tls_cert: Option<PathBuf>,

    /// TLS private key PEM.
    #[arg(long, env = "SAAIOS_STAGE_TLS_KEY")]
    tls_key: Option<PathBuf>,

    /// Serve the UI in kiosk layout by default (also available as /kiosk).
    #[arg(long, env = "SAAIOS_STAGE_KIOSK")]
    kiosk: bool,
}

#[derive(Clone)]
struct AppState {
    sock: PathBuf,
    /// Browser session → runtime chat session.
    chat_session: Arc<Mutex<Option<Uuid>>>,
    token: Option<String>,
    kiosk_default: bool,
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

#[derive(Debug, Deserialize)]
struct PageQuery {
    #[serde(default)]
    kiosk: Option<String>,
    #[serde(default)]
    token: Option<String>,
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
    let bind = resolve_bind(&args)?;
    let tls = match (&args.tls_cert, &args.tls_key) {
        (Some(cert), Some(key)) => Some((cert.clone(), key.clone())),
        (None, None) => None,
        _ => {
            return Err(anyhow!(
                "both --tls-cert and --tls-key are required for HTTPS"
            ))
        }
    };

    if args.lan {
        if args.token.is_none() && !args.allow_open {
            return Err(anyhow!(
                "--lan requires --token (or SAAIOS_STAGE_TOKEN), or pass --allow-open for lab use"
            ));
        }
        if args.token.is_none() {
            warn!("LAN bind without token (--allow-open): anyone on the network can control SaaiOS");
        }
        if tls.is_none() {
            warn!("LAN bind without TLS — prefer --tls-cert/--tls-key on untrusted networks");
        }
    }

    let state = AppState {
        sock: args.sock.clone(),
        chat_session: Arc::new(Mutex::new(None)),
        token: args.token.clone(),
        kiosk_default: args.kiosk,
    };

    match uds_request(&args.sock, &ClientRequest::Ping).await {
        Ok(_) => info!(sock = %args.sock.display(), "runtime reachable"),
        Err(e) => warn!(error = %e, "runtime not reachable yet — start with `just run-mock`"),
    }

    let app = Router::new()
        .route("/", get(index))
        .route("/kiosk", get(index_kiosk))
        .route("/app.css", get(app_css))
        .route("/app.js", get(app_js))
        .route("/api/status", get(api_status))
        .route("/api/events", get(api_events))
        .route("/api/grants", get(api_grants))
        .route("/api/grants/clear", post(api_clear_grants))
        .route("/api/chat/reset", post(api_chat_reset))
        .route("/api/diagnose", post(api_diagnose_stream))
        .route("/api/confirm", post(api_confirm))
        .layer(from_fn_with_state(state.clone(), require_token))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let scheme = if tls.is_some() { "https" } else { "http" };
    info!(%bind, %scheme, kiosk = args.kiosk, "SaaiOS Stage listening");

    if let Some((cert, key)) = tls {
        let config = RustlsConfig::from_pem_file(cert, key)
            .await
            .context("load TLS cert/key")?;
        axum_server::bind_rustls(bind, config)
            .serve(app.into_make_service())
            .await?;
    } else {
        let listener = tokio::net::TcpListener::bind(bind).await?;
        axum::serve(listener, app).await?;
    }
    Ok(())
}

fn resolve_bind(args: &Args) -> Result<SocketAddr> {
    if args.lan {
        return Ok(SocketAddr::from(([0, 0, 0, 0], args.port)));
    }
    args.bind.parse().context("parse --bind address")
}

async fn require_token(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let Some(expected) = state.token.as_deref() else {
        return Ok(next.run(req).await);
    };
    if token_matches(expected, req.headers(), req.uri().query()) {
        return Ok(next.run(req).await);
    }
    Err(StatusCode::UNAUTHORIZED)
}

fn token_matches(expected: &str, headers: &HeaderMap, query: Option<&str>) -> bool {
    if let Some(auth) = headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()) {
        if let Some(rest) = auth.strip_prefix("Bearer ") {
            if rest == expected {
                return true;
            }
        }
    }
    if let Some(t) = headers
        .get("x-saaios-token")
        .and_then(|v| v.to_str().ok())
    {
        if t == expected {
            return true;
        }
    }
    if let Some(q) = query {
        for pair in q.split('&') {
            if let Some(v) = pair.strip_prefix("token=") {
                if v == expected {
                    return true;
                }
            }
        }
    }
    if let Some(cookie) = headers.get(header::COOKIE).and_then(|v| v.to_str().ok()) {
        for part in cookie.split(';') {
            let part = part.trim();
            if let Some(v) = part.strip_prefix("saaios_token=") {
                if v == expected {
                    return true;
                }
            }
        }
    }
    false
}

fn render_index(kiosk: bool) -> Response {
    let html = include_str!("../static/index.html");
    let html = if kiosk {
        html.replace("<body>", r#"<body class="kiosk">"#)
    } else {
        html.to_string()
    };
    let mut res = Html(html).into_response();
    res.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    res
}

async fn index(State(state): State<AppState>, Query(q): Query<PageQuery>) -> Response {
    let kiosk = state.kiosk_default || q.kiosk.as_deref().is_some_and(|v| v != "0" && v != "false");
    let mut res = render_index(kiosk);
    if let (Some(expected), Some(provided)) = (state.token.as_deref(), q.token.as_deref()) {
        if provided == expected {
            let cookie = format!("saaios_token={expected}; Path=/; SameSite=Strict; HttpOnly");
            if let Ok(val) = header::HeaderValue::from_str(&cookie) {
                res.headers_mut().append(header::SET_COOKIE, val);
            }
        }
    }
    res
}

async fn index_kiosk(State(state): State<AppState>, Query(q): Query<PageQuery>) -> Response {
    let mut res = render_index(true);
    if let (Some(expected), Some(provided)) = (state.token.as_deref(), q.token.as_deref()) {
        if provided == expected {
            let cookie = format!("saaios_token={expected}; Path=/; SameSite=Strict; HttpOnly");
            if let Ok(val) = header::HeaderValue::from_str(&cookie) {
                res.headers_mut().append(header::SET_COOKIE, val);
            }
        }
    }
    res
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
        return error_json(StatusCode::BAD_REQUEST, anyhow!("empty text"));
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
                anyhow!("failed to build stream body"),
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn token_from_bearer_and_query() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer secret"),
        );
        assert!(token_matches("secret", &headers, None));
        assert!(!token_matches("nope", &headers, None));
        assert!(token_matches("q", &HeaderMap::new(), Some("token=q&x=1")));
    }

    #[test]
    fn lan_bind_uses_all_interfaces() {
        let args = Args {
            sock: PathBuf::from("/tmp/x.sock"),
            bind: "127.0.0.1:1".into(),
            lan: true,
            port: 7420,
            token: Some("t".into()),
            allow_open: false,
            tls_cert: None,
            tls_key: None,
            kiosk: false,
        };
        let addr = resolve_bind(&args).unwrap();
        assert_eq!(addr, SocketAddr::from(([0, 0, 0, 0], 7420)));
    }
}
