use anyhow::{Context, Result};
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use protocol::ConfirmScope;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use serde::{Deserialize, Serialize};
use std::io::stdout;
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use uuid::Uuid;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, default_value = "/tmp/saaios.sock", env = "SAAIOS_SOCK")]
    sock: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum ClientRequest {
    Diagnose {
        text: String,
        #[serde(default)]
        session_id: Option<Uuid>,
        #[serde(default)]
        stream: bool,
    },
    Confirm {
        correlation_id: Uuid,
        call_id: Uuid,
        tool: String,
        arguments: serde_json::Value,
        scope: ConfirmScope,
        #[serde(default)]
        session_id: Option<Uuid>,
    },
    ChatReset {
        session_id: Uuid,
    },
    AuditTail {
        limit: usize,
    },
    MemoryRemember {
        key: String,
        value: String,
        #[serde(default)]
        tags: Vec<String>,
    },
    MemoryRecall {
        #[serde(default)]
        query: String,
    },
    MemoryTail {
        limit: usize,
    },
    MemoryForget {
        key: String,
    },
    Status,
    EventsTail {
        limit: usize,
    },
    SessionGrants,
    ClearSessionGrants,
    Ping,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClientResponse {
    ok: bool,
    correlation_id: Option<Uuid>,
    #[serde(default)]
    session_id: Option<Uuid>,
    diagnose: Option<protocol::DiagnoseResult>,
    pending: Option<PendingDto>,
    error: Option<String>,
    tool_result: Option<protocol::ToolCallResult>,
    #[serde(default)]
    audit_tail: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    session_grants: Option<Vec<String>>,
    #[serde(default)]
    memory_facts: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    status: Option<serde_json::Value>,
    #[serde(default)]
    events: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    progress: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PendingDto {
    call_id: Uuid,
    tool: String,
    arguments: serde_json::Value,
    summary: String,
}

struct App {
    sock: PathBuf,
    input: String,
    lines: Vec<String>,
    pending: Option<(Uuid, PendingDto)>,
    status: String,
    last_correlation: Option<Uuid>,
    chat_session: Option<Uuid>,
}

impl App {
    fn new(sock: PathBuf) -> Self {
        Self {
            sock,
            input: String::new(),
            lines: vec![
                "SaaiOS Console 0.5".into(),
                "Type a question and press Enter. Example: Почему система тормозит?".into(),
                "Confirm: y=once, s=session, n=cancel | h=status | e=events | a=audit | m=memory | g=grants | c=clear | q=quit"
                    .into(),
                "Chat: multi-turn session kept across prompts | /new resets | /remember key=value"
                    .into(),
            ],
            pending: None,
            status: "disconnected".into(),
            last_correlation: None,
            chat_session: None,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let mut app = App::new(args.sock.clone());

    match request(&args.sock, &ClientRequest::Ping).await {
        Ok(_) => app.status = format!("connected {}", args.sock.display()),
        Err(e) => {
            app.status = format!("runtime unavailable at {}: {e}", args.sock.display());
            app.lines
                .push("Start runtime first: `just run-mock` (keeps server listening).".into());
        }
    }

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let result = run_loop(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    result
}

async fn run_loop(terminal: &mut Terminal<impl Backend>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') if app.pending.is_none() && app.input.is_empty() => break,
                    KeyCode::Char('y') | KeyCode::Char('Y') if app.pending.is_some() => {
                        confirm(app, ConfirmScope::Once).await?;
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') if app.pending.is_some() => {
                        confirm(app, ConfirmScope::Session).await?;
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') if app.pending.is_some() => {
                        confirm(app, ConfirmScope::Cancel).await?;
                    }
                    KeyCode::Char('a') if app.pending.is_none() && app.input.is_empty() => {
                        show_audit(app).await?;
                    }
                    KeyCode::Char('h') if app.pending.is_none() && app.input.is_empty() => {
                        show_status(app).await?;
                    }
                    KeyCode::Char('e') if app.pending.is_none() && app.input.is_empty() => {
                        show_events(app).await?;
                    }
                    KeyCode::Char('m') if app.pending.is_none() && app.input.is_empty() => {
                        show_memory(app).await?;
                    }
                    KeyCode::Char('g') if app.pending.is_none() && app.input.is_empty() => {
                        show_grants(app).await?;
                    }
                    KeyCode::Char('c') if app.pending.is_none() && app.input.is_empty() => {
                        clear_grants(app).await?;
                    }
                    KeyCode::Enter => {
                        if app.pending.is_some() {
                            continue;
                        }
                        let text = app.input.trim().to_string();
                        if text.is_empty() {
                            continue;
                        }
                        app.input.clear();
                        app.lines.push(format!("> {text}"));
                        if text.starts_with('/') {
                            handle_slash(app, &text).await?;
                        } else {
                            diagnose(app, &text).await?;
                        }
                    }
                    KeyCode::Backspace => {
                        app.input.pop();
                    }
                    KeyCode::Char(c) if app.pending.is_none() => {
                        app.input.push(c);
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

async fn diagnose(app: &mut App, text: &str) -> Result<()> {
    match request_stream(
        &app.sock,
        &ClientRequest::Diagnose {
            text: text.to_string(),
            session_id: app.chat_session,
            stream: true,
        },
        |line| {
            if let Some(kind) = line.get("type").and_then(|v| v.as_str()) {
                if kind == "progress" {
                    if let Some(ev) = line.get("event") {
                        return Some(format_progress(ev));
                    }
                }
            }
            None
        },
    )
    .await
    {
        Ok((resp, progress_lines)) => {
            for line in progress_lines {
                if !line.is_empty() {
                    app.lines.push(line);
                }
            }
            if let Some(err) = resp.error {
                app.lines.push(format!("error: {err}"));
                return Ok(());
            }
            if let Some(sid) = resp.session_id {
                app.chat_session = Some(sid);
            }
            if let Some(corr) = resp.correlation_id {
                app.last_correlation = Some(corr);
            }
            if let Some(d) = resp.diagnose {
                // Avoid duplicating assistant text already shown via progress.
                if app.lines.last().map(|l| l.as_str()) != Some(d.summary.as_str()) {
                    app.lines.push(d.summary);
                }
                if let Some(action) = d.proposed_action {
                    app.lines
                        .push(format!("Предлагаемое действие: {}", action.summary));
                }
            }
            if let (Some(corr), Some(pending)) = (resp.correlation_id, resp.pending) {
                app.lines.push(format!(
                    "[Confirm] {}  y=once / s=session / n=cancel",
                    pending.summary
                ));
                app.pending = Some((corr, pending));
            }
            let sid = app
                .chat_session
                .map(|s| s.to_string())
                .unwrap_or_else(|| "-".into());
            if let Some(grants) = resp.session_grants {
                app.status = format!("ok | session={} | grants={}", &sid[..8.min(sid.len())], grants.join(","));
            } else {
                app.status = format!("ok | session={}", &sid[..8.min(sid.len())]);
            }
        }
        Err(e) => app.lines.push(format!("request failed: {e:#}")),
    }
    Ok(())
}

fn format_progress(ev: &serde_json::Value) -> String {
    let kind = ev.get("type").and_then(|v| v.as_str()).unwrap_or("?");
    match kind {
        "tool_call" => {
            let tool = ev.get("tool").and_then(|v| v.as_str()).unwrap_or("?");
            format!("… tool {tool}")
        }
        "tool_result" => {
            let tool = ev.get("tool").and_then(|v| v.as_str()).unwrap_or("?");
            let ok = ev.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
            format!("… {tool} → {}", if ok { "ok" } else { "fail" })
        }
        "policy" => {
            let tool = ev.get("tool").and_then(|v| v.as_str()).unwrap_or("?");
            let verdict = ev.get("verdict").and_then(|v| v.as_str()).unwrap_or("?");
            format!("… policy {tool}={verdict}")
        }
        "assistant_delta" => String::new(),
        "assistant" => ev
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        "confirmation" => String::new(),
        "completed" => String::new(),
        "error" => format!(
            "error: {}",
            ev.get("message").and_then(|v| v.as_str()).unwrap_or("?")
        ),
        _ => format!("… {kind}"),
    }
}

async fn confirm(app: &mut App, scope: ConfirmScope) -> Result<()> {
    let Some((correlation_id, pending)) = app.pending.take() else {
        return Ok(());
    };
    match request(
        &app.sock,
        &ClientRequest::Confirm {
            correlation_id,
            call_id: pending.call_id,
            tool: pending.tool,
            arguments: pending.arguments,
            scope,
            session_id: app.chat_session,
        },
    )
    .await
    {
        Ok(resp) => {
            if let Some(err) = resp.error {
                app.lines.push(format!("confirm error: {err}"));
            } else if let Some(result) = resp.tool_result {
                app.lines.push(format!(
                    "tool {} scope={:?} ok={} output={}",
                    result.tool, scope, result.ok, result.output
                ));
            } else {
                app.lines.push(format!("confirm scope={scope:?}"));
            }
            if let Some(grants) = resp.session_grants {
                app.status = format!("ok | grants={}", grants.join(","));
            }
        }
        Err(e) => app.lines.push(format!("confirm failed: {e:#}")),
    }
    Ok(())
}

async fn show_audit(app: &mut App) -> Result<()> {
    match request(&app.sock, &ClientRequest::AuditTail { limit: 12 }).await {
        Ok(resp) => {
            if let Some(err) = resp.error {
                app.lines.push(format!("audit error: {err}"));
            } else if let Some(tail) = resp.audit_tail {
                app.lines.push("--- audit tail ---".into());
                for item in tail {
                    let kind = item.get("kind").cloned().unwrap_or_default();
                    let corr = item.get("correlation_id").cloned().unwrap_or_default();
                    app.lines.push(format!("{kind} corr={corr}"));
                }
            }
        }
        Err(e) => app.lines.push(format!("audit failed: {e:#}")),
    }
    Ok(())
}

async fn show_status(app: &mut App) -> Result<()> {
    match request(&app.sock, &ClientRequest::Status).await {
        Ok(resp) => {
            if let Some(err) = resp.error {
                app.lines.push(format!("status error: {err}"));
            } else if let Some(st) = resp.status {
                app.lines.push("--- runtime status ---".into());
                let provider = st.get("provider").and_then(|v| v.as_str()).unwrap_or("?");
                let kind = st
                    .get("provider_kind")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let uptime = st.get("uptime_secs").and_then(|v| v.as_u64()).unwrap_or(0);
                let tools = st.get("tool_count").and_then(|v| v.as_u64()).unwrap_or(0);
                let mem = st
                    .get("memory_enabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let auto = st
                    .get("automation")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let tel = st
                    .get("telemetry")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let samples = st
                    .get("telemetry_samples")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let adiag = st
                    .get("auto_diagnose")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let chats = st
                    .get("chat_sessions")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                app.lines.push(format!(
                    "provider={provider} ({kind}) uptime={uptime}s tools={tools}"
                ));
                app.lines.push(format!(
                    "memory={mem} automation={auto} auto_diagnose={adiag} telemetry={tel} samples={samples} chat_sessions={chats}"
                ));
                if let Some(ab) = st.get("ab") {
                    let enabled = ab.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
                    if enabled {
                        let current = ab
                            .get("current")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?");
                        let mut slot_bits = Vec::new();
                        if let Some(slots) = ab.get("slots").and_then(|v| v.as_array()) {
                            for s in slots {
                                let name = s.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                                let bin = s
                                    .get("binary_present")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false);
                                let ok = s
                                    .get("boot_ok")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false);
                                slot_bits.push(format!(
                                    "{name}[bin={},ok={}]",
                                    if bin { "y" } else { "n" },
                                    if ok { "y" } else { "n" }
                                ));
                            }
                        }
                        app.lines.push(format!(
                            "ab current={current} {}",
                            slot_bits.join(" ")
                        ));
                    } else {
                        app.lines.push("ab=disabled (no layout)".into());
                    }
                }
                app.status = format!("provider={provider} up={uptime}s");
            }
        }
        Err(e) => app.lines.push(format!("status failed: {e:#}")),
    }
    Ok(())
}

async fn show_events(app: &mut App) -> Result<()> {
    match request(&app.sock, &ClientRequest::EventsTail { limit: 16 }).await {
        Ok(resp) => {
            if let Some(err) = resp.error {
                app.lines.push(format!("events error: {err}"));
            } else if let Some(events) = resp.events {
                app.lines.push("--- events ---".into());
                if events.is_empty() {
                    app.lines.push("(empty)".into());
                }
                for item in events {
                    let ev = item.get("event").and_then(|v| v.as_str()).unwrap_or("?");
                    let summary = item.get("summary").and_then(|v| v.as_str()).unwrap_or("");
                    app.lines.push(format!("{ev}: {summary}"));
                }
            }
        }
        Err(e) => app.lines.push(format!("events failed: {e:#}")),
    }
    Ok(())
}

async fn show_memory(app: &mut App) -> Result<()> {
    match request(&app.sock, &ClientRequest::MemoryTail { limit: 20 }).await {
        Ok(resp) => {
            if let Some(err) = resp.error {
                app.lines.push(format!("memory error: {err}"));
            } else if let Some(facts) = resp.memory_facts {
                app.lines.push("--- memory ---".into());
                if facts.is_empty() {
                    app.lines.push("(empty)".into());
                }
                for item in facts {
                    let key = item.get("key").and_then(|v| v.as_str()).unwrap_or("?");
                    let value = item.get("value").and_then(|v| v.as_str()).unwrap_or("?");
                    app.lines.push(format!("{key} = {value}"));
                }
            }
        }
        Err(e) => app.lines.push(format!("memory failed: {e:#}")),
    }
    Ok(())
}

async fn handle_slash(app: &mut App, text: &str) -> Result<()> {
    let mut parts = text.splitn(2, char::is_whitespace);
    let cmd = parts.next().unwrap_or("");
    let rest = parts.next().unwrap_or("").trim();
    match cmd {
        "/new" | "/reset" => {
            if let Some(sid) = app.chat_session {
                let _ = request(&app.sock, &ClientRequest::ChatReset { session_id: sid }).await;
            }
            app.chat_session = None;
            app.pending = None;
            app.lines.push("chat session reset".into());
            app.status = "session=new".into();
        }
        "/remember" => {
            let Some((key, value)) = rest.split_once('=') else {
                app.lines.push("usage: /remember key=value".into());
                return Ok(());
            };
            match request(
                &app.sock,
                &ClientRequest::MemoryRemember {
                    key: key.trim().to_string(),
                    value: value.trim().to_string(),
                    tags: vec![],
                },
            )
            .await
            {
                Ok(resp) => {
                    if let Some(err) = resp.error {
                        app.lines.push(format!("remember error: {err}"));
                    } else {
                        app.lines
                            .push(format!("remembered {}={}", key.trim(), value.trim()));
                    }
                }
                Err(e) => app.lines.push(format!("remember failed: {e:#}")),
            }
        }
        "/recall" => {
            match request(
                &app.sock,
                &ClientRequest::MemoryRecall {
                    query: rest.to_string(),
                },
            )
            .await
            {
                Ok(resp) => {
                    if let Some(err) = resp.error {
                        app.lines.push(format!("recall error: {err}"));
                    } else if let Some(facts) = resp.memory_facts {
                        if facts.is_empty() {
                            app.lines.push("no matches".into());
                        }
                        for item in facts {
                            let key = item.get("key").and_then(|v| v.as_str()).unwrap_or("?");
                            let value = item.get("value").and_then(|v| v.as_str()).unwrap_or("?");
                            app.lines.push(format!("{key} = {value}"));
                        }
                    }
                }
                Err(e) => app.lines.push(format!("recall failed: {e:#}")),
            }
        }
        "/forget" => {
            if rest.is_empty() {
                app.lines.push("usage: /forget key".into());
                return Ok(());
            }
            match request(
                &app.sock,
                &ClientRequest::MemoryForget {
                    key: rest.to_string(),
                },
            )
            .await
            {
                Ok(resp) => {
                    if let Some(err) = resp.error {
                        app.lines.push(format!("forget error: {err}"));
                    } else {
                        app.lines.push(format!("forgot {rest}"));
                    }
                }
                Err(e) => app.lines.push(format!("forget failed: {e:#}")),
            }
        }
        _ => app.lines.push(format!("unknown command: {cmd}")),
    }
    Ok(())
}

async fn show_grants(app: &mut App) -> Result<()> {
    match request(&app.sock, &ClientRequest::SessionGrants).await {
        Ok(resp) => {
            let grants = resp.session_grants.unwrap_or_default();
            app.lines.push(format!(
                "session grants: {}",
                if grants.is_empty() {
                    "(none)".into()
                } else {
                    grants.join(", ")
                }
            ));
            app.status = format!("grants={}", grants.join(","));
        }
        Err(e) => app.lines.push(format!("grants failed: {e:#}")),
    }
    Ok(())
}

async fn clear_grants(app: &mut App) -> Result<()> {
    match request(&app.sock, &ClientRequest::ClearSessionGrants).await {
        Ok(_) => {
            app.lines.push("session grants cleared".into());
            app.status = "grants=".into();
        }
        Err(e) => app.lines.push(format!("clear grants failed: {e:#}")),
    }
    Ok(())
}

async fn request(sock: &PathBuf, req: &ClientRequest) -> Result<ClientResponse> {
    let mut stream = UnixStream::connect(sock)
        .await
        .with_context(|| format!("connect {}", sock.display()))?;
    let bytes = serde_json::to_vec(req)?;
    stream.write_all(&bytes).await?;
    stream.shutdown().await?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await?;
    let resp: ClientResponse = serde_json::from_slice(&buf)?;
    Ok(resp)
}

async fn request_stream<F>(
    sock: &PathBuf,
    req: &ClientRequest,
    mut on_progress: F,
) -> Result<(ClientResponse, Vec<String>)>
where
    F: FnMut(&serde_json::Value) -> Option<String>,
{
    let mut stream = UnixStream::connect(sock)
        .await
        .with_context(|| format!("connect {}", sock.display()))?;
    let bytes = serde_json::to_vec(req)?;
    stream.write_all(&bytes).await?;
    stream.shutdown().await?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await?;
    let text = String::from_utf8_lossy(&buf);
    let mut progress_lines = Vec::new();
    let mut done: Option<ClientResponse> = None;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line)
            .with_context(|| format!("parse stream line: {line}"))?;
        match value.get("type").and_then(|v| v.as_str()) {
            Some("progress") => {
                if let Some(s) = on_progress(&value) {
                    progress_lines.push(s);
                }
            }
            Some("done") | None => {
                done = Some(serde_json::from_value(value)?);
            }
            Some(_) => {
                // Unknown frame; ignore.
            }
        }
    }
    let resp = done.ok_or_else(|| anyhow::anyhow!("stream ended without done frame"))?;
    Ok((resp, progress_lines))
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(f.area());

    let status = Paragraph::new(app.status.clone())
        .block(Block::default().borders(Borders::ALL).title("Status"));
    f.render_widget(status, chunks[0]);

    let text = app.lines.join("\n");
    let transcript = Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .block(Block::default().borders(Borders::ALL).title("SaaiOS"));
    f.render_widget(transcript, chunks[1]);

    let input = Paragraph::new(app.input.clone())
        .block(Block::default().borders(Borders::ALL).title("Input"));
    f.render_widget(input, chunks[2]);
}
