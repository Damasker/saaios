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
    },
    Confirm {
        correlation_id: Uuid,
        call_id: Uuid,
        tool: String,
        arguments: serde_json::Value,
        scope: ConfirmScope,
    },
    AuditTail {
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
    diagnose: Option<protocol::DiagnoseResult>,
    pending: Option<PendingDto>,
    error: Option<String>,
    tool_result: Option<protocol::ToolCallResult>,
    #[serde(default)]
    audit_tail: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    session_grants: Option<Vec<String>>,
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
}

impl App {
    fn new(sock: PathBuf) -> Self {
        Self {
            sock,
            input: String::new(),
            lines: vec![
                "SaaiOS Console 0.2".into(),
                "Type a question and press Enter. Example: Почему система тормозит?".into(),
                "Confirm: y=once, s=session, n=cancel | a=audit tail | g=grants | c=clear grants | q=quit"
                    .into(),
            ],
            pending: None,
            status: "disconnected".into(),
            last_correlation: None,
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
                        diagnose(app, &text).await?;
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
    match request(
        &app.sock,
        &ClientRequest::Diagnose {
            text: text.to_string(),
        },
    )
    .await
    {
        Ok(resp) => {
            if let Some(err) = resp.error {
                app.lines.push(format!("error: {err}"));
                return Ok(());
            }
            if let Some(corr) = resp.correlation_id {
                app.last_correlation = Some(corr);
                app.lines.push(format!("correlation_id={corr}"));
            }
            if let Some(d) = resp.diagnose {
                app.lines.push(d.summary);
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
            if let Some(grants) = resp.session_grants {
                app.status = format!("ok | grants={}", grants.join(","));
            } else {
                app.status = "ok".into();
            }
        }
        Err(e) => app.lines.push(format!("request failed: {e:#}")),
    }
    Ok(())
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
