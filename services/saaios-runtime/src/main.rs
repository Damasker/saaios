use ai_runtime::{diagnose_slow_with_mock_planner, AiRuntime, HandleOutcome};
use anyhow::{anyhow, Context, Result};
use audit_log::AuditLog;
use clap::Parser;
use event_bus::EventBus;
use model_provider::{MockModelProvider, OpenAiCompatConfig, OpenAiCompatProvider};
use policy_engine::PolicyEngine;
use protocol::{Envelope, MessageKind};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use system_tools::{install_system_tools, ToolsMode};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tool_registry::ToolRegistry;
use tracing::{error, info};
use uuid::Uuid;

#[derive(Debug, Parser)]
#[command(name = "saaios-runtime", about = "SaaiOS Platform runtime")]
struct Args {
    /// Use mock tools + mock model provider
    #[arg(long, env = "SAAIOS_MOCK")]
    mock: bool,

    /// Use deterministic mock planner (no model loop)
    #[arg(long)]
    mock_planner: bool,

    /// Unix domain socket path
    #[arg(long, default_value = "/tmp/saaios.sock", env = "SAAIOS_SOCK")]
    sock: PathBuf,

    /// Audit log path
    #[arg(long, default_value = "saaios-audit.jsonl", env = "SAAIOS_AUDIT")]
    audit: PathBuf,

    /// Use real Linux /proc adapters instead of mock fixtures
    #[arg(long)]
    real_linux: bool,
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
        confirmed: bool,
    },
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
}

#[derive(Debug, Serialize, Deserialize)]
struct PendingDto {
    call_id: Uuid,
    tool: String,
    arguments: serde_json::Value,
    summary: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let mut args = Args::parse();
    if std::env::var("SAAIOS_MODE").ok().as_deref() == Some("mock") {
        args.mock = true;
        args.mock_planner = true;
    }

    let tools_mode = if args.real_linux {
        ToolsMode::RealLinux
    } else {
        ToolsMode::Mock
    };

    let mut registry = ToolRegistry::new();
    install_system_tools(&mut registry, tools_mode);
    let tools = Arc::new(registry);
    let policy = Arc::new(PolicyEngine::new());
    let audit = Arc::new(AuditLog::open(&args.audit)?);
    let bus = EventBus::new(64);

    if args.mock_planner {
        info!("running one-shot mock planner demo");
        let outcome =
            diagnose_slow_with_mock_planner(tools.clone(), policy.clone(), audit.clone()).await?;
        print_outcome(&outcome);
        info!("audit written to {}", args.audit.display());
        info!(
            "starting UDS server for interactive confirms at {}",
            args.sock.display()
        );
    }

    let provider: Arc<dyn model_provider::ModelProvider> = if args.mock || args.mock_planner {
        Arc::new(MockModelProvider)
    } else {
        let api_base =
            std::env::var("SAAIOS_API_BASE").context("SAAIOS_API_BASE required unless --mock")?;
        let api_key =
            std::env::var("SAAIOS_API_KEY").context("SAAIOS_API_KEY required unless --mock")?;
        let model = std::env::var("SAAIOS_MODEL").unwrap_or_else(|_| "gpt-4o-mini".into());
        Arc::new(OpenAiCompatProvider::new(OpenAiCompatConfig {
            api_base,
            api_key,
            model,
        }))
    };

    let runtime = Arc::new(AiRuntime::new(tools, policy, audit.clone(), bus, provider));

    // Store last pending in a simple map for TUI confirmations.
    let last_pending = Arc::new(tokio::sync::Mutex::new(None::<HandleOutcome>));

    if args.sock.exists() {
        let _ = std::fs::remove_file(&args.sock);
    }
    let listener =
        UnixListener::bind(&args.sock).with_context(|| format!("bind {}", args.sock.display()))?;
    info!("SaaiOS runtime listening on {}", args.sock.display());

    loop {
        let (stream, _) = listener.accept().await?;
        let runtime = runtime.clone();
        let last_pending = last_pending.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream, runtime, last_pending).await {
                error!("client error: {e:#}");
            }
        });
    }
}

async fn handle_client(
    mut stream: UnixStream,
    runtime: Arc<AiRuntime>,
    last_pending: Arc<tokio::sync::Mutex<Option<HandleOutcome>>>,
) -> Result<()> {
    let mut buf = vec![0u8; 64 * 1024];
    let n = stream.read(&mut buf).await?;
    if n == 0 {
        return Ok(());
    }
    let req: ClientRequest = serde_json::from_slice(&buf[..n])?;

    let response = match req {
        ClientRequest::Ping => ClientResponse {
            ok: true,
            correlation_id: None,
            diagnose: None,
            pending: None,
            error: None,
            tool_result: None,
        },
        ClientRequest::Diagnose { text } => match runtime.handle_user_text(&text).await {
            Ok(outcome) => {
                let pending = outcome.pending_confirmation.as_ref().map(|p| PendingDto {
                    call_id: p.call_id,
                    tool: p.tool.clone(),
                    arguments: p.arguments.clone(),
                    summary: p.summary.clone(),
                });
                let resp = ClientResponse {
                    ok: true,
                    correlation_id: Some(outcome.correlation_id),
                    diagnose: Some(outcome.diagnose.clone()),
                    pending,
                    error: None,
                    tool_result: None,
                };
                *last_pending.lock().await = Some(outcome);
                resp
            }
            Err(e) => ClientResponse {
                ok: false,
                correlation_id: None,
                diagnose: None,
                pending: None,
                error: Some(e.to_string()),
                tool_result: None,
            },
        },
        ClientRequest::Confirm {
            correlation_id,
            call_id,
            tool,
            arguments,
            confirmed,
        } => match runtime
            .confirm(correlation_id, call_id, &tool, arguments, confirmed)
            .await
        {
            Ok(result) => ClientResponse {
                ok: true,
                correlation_id: Some(correlation_id),
                diagnose: None,
                pending: None,
                error: None,
                tool_result: Some(result),
            },
            Err(e) => ClientResponse {
                ok: false,
                correlation_id: Some(correlation_id),
                diagnose: None,
                pending: None,
                error: Some(e.to_string()),
                tool_result: None,
            },
        },
    };

    let bytes = serde_json::to_vec(&response)?;
    stream.write_all(&bytes).await?;
    stream.shutdown().await?;
    Ok(())
}

fn print_outcome(outcome: &HandleOutcome) {
    println!("correlation_id={}", outcome.correlation_id);
    println!("{}", outcome.diagnose.summary);
    if let Some(action) = &outcome.diagnose.proposed_action {
        println!("proposed: {} ({})", action.summary, action.tool);
    }
}

#[allow(dead_code)]
fn encode_env(env: &Envelope) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    ciborium::into_writer(env, &mut buf).map_err(|e| anyhow!(e.to_string()))?;
    Ok(buf)
}

#[allow(dead_code)]
fn demo_event(correlation_id: Uuid) -> Envelope {
    Envelope::new(
        MessageKind::Event,
        correlation_id,
        None,
        json!({"hello": "saaios"}),
    )
}
