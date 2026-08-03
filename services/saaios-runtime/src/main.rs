use ai_runtime::{diagnose_slow_with_mock_planner, AiRuntime, HandleOutcome, ResourceBudgets};
use anyhow::{anyhow, Context, Result};
use audit_log::AuditLog;
use automation_engine::AutomationEngine;
use clap::Parser;
use event_bus::EventBus;
use memory_store::{install_memory_tools, MemoryFact, MemoryStore};
use model_provider::{build_provider, ProviderKind};
use policy_engine::PolicyEngine;
use protocol::{ConfirmScope, Envelope, MessageKind};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use system_tools::{install_system_tools, ToolsMode};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tool_registry::ToolRegistry;
use tracing::{error, info};
use uuid::Uuid;

#[derive(Debug, Parser)]
#[command(name = "saaios-runtime", about = "SaaiOS Platform runtime")]
struct Args {
    /// Use mock tools + mock model provider (alias for --provider mock)
    #[arg(long, env = "SAAIOS_MOCK")]
    mock: bool,

    /// Model provider: mock | remote | local | auto
    #[arg(long, env = "SAAIOS_PROVIDER", default_value = "auto")]
    provider: String,

    /// Use deterministic mock planner (no model loop)
    #[arg(long)]
    mock_planner: bool,

    /// Unix domain socket path
    #[arg(long, default_value = "/tmp/saaios.sock", env = "SAAIOS_SOCK")]
    sock: PathBuf,

    /// Audit log path
    #[arg(long, default_value = "saaios-audit.jsonl", env = "SAAIOS_AUDIT")]
    audit: PathBuf,

    /// Local memory / facts JSONL path
    #[arg(long, default_value = "saaios-memory.jsonl", env = "SAAIOS_MEMORY")]
    memory: PathBuf,

    /// Disable memory store and memory.* tools
    #[arg(long)]
    no_memory: bool,

    /// Use real Linux /proc adapters instead of mock fixtures
    #[arg(long)]
    real_linux: bool,

    /// Dry-run replay of an audit correlation id (no tool side effects)
    #[arg(long)]
    replay: Option<Uuid>,

    /// Max concurrent AI requests
    #[arg(long, default_value_t = 1, env = "SAAIOS_MAX_CONCURRENT")]
    max_concurrent: usize,

    /// AI request timeout in seconds
    #[arg(long, default_value_t = 30, env = "SAAIOS_REQUEST_TIMEOUT_SECS")]
    request_timeout_secs: u64,

    /// Disable event-driven automation worker
    #[arg(long)]
    no_automation: bool,
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
        #[serde(default = "default_once")]
        scope: ConfirmScope,
        /// Backward-compatible boolean confirm; ignored when `scope` is set explicitly.
        #[serde(default)]
        confirmed: Option<bool>,
    },
    AuditTail {
        #[serde(default = "default_tail")]
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
        #[serde(default = "default_tail")]
        limit: usize,
    },
    MemoryForget {
        key: String,
    },
    SessionGrants,
    ClearSessionGrants,
    Ping,
}

fn default_once() -> ConfirmScope {
    ConfirmScope::Once
}

fn default_tail() -> usize {
    20
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
    audit_tail: Option<Vec<audit_log::AuditRecord>>,
    #[serde(default)]
    session_grants: Option<Vec<String>>,
    #[serde(default)]
    memory_facts: Option<Vec<MemoryFact>>,
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
        .with_target(false)
        .init();

    let mut args = Args::parse();
    if std::env::var("SAAIOS_MODE").ok().as_deref() == Some("mock") {
        args.mock = true;
        args.mock_planner = true;
        args.provider = "mock".into();
    }
    if args.mock {
        args.provider = "mock".into();
    }

    if let Some(correlation_id) = args.replay {
        let audit = AuditLog::open(&args.audit)?;
        let report = audit.replay(correlation_id)?;
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    let tools_mode = if args.real_linux {
        ToolsMode::RealLinux
    } else {
        ToolsMode::Mock
    };

    let mut registry = ToolRegistry::new();
    install_system_tools(&mut registry, tools_mode);

    let memory = if args.no_memory {
        None
    } else {
        let store = Arc::new(MemoryStore::open(&args.memory)?);
        install_memory_tools(&mut registry, store.clone());
        info!(path = %args.memory.display(), "memory store ready");
        Some(store)
    };

    let tools = Arc::new(registry);
    let policy = Arc::new(PolicyEngine::new());
    let audit = Arc::new(AuditLog::open(&args.audit)?);
    let bus = EventBus::new(64);

    if args.mock_planner {
        info!("running one-shot mock planner demo");
        let outcome =
            diagnose_slow_with_mock_planner(tools.clone(), policy.clone(), audit.clone()).await?;
        print_outcome(&outcome);
        info!(correlation_id = %outcome.correlation_id, "audit written");
        info!(sock = %args.sock.display(), "starting UDS server");
    }

    let kind = if args.mock_planner {
        ProviderKind::Mock
    } else {
        ProviderKind::parse(&args.provider).ok_or_else(|| {
            anyhow!(
                "unknown provider {:?}; expected mock|remote|local|auto",
                args.provider
            )
        })?
    };
    let provider = build_provider(
        kind,
        std::env::var("SAAIOS_API_BASE").ok(),
        std::env::var("SAAIOS_API_KEY").ok(),
        std::env::var("SAAIOS_MODEL").ok(),
        std::env::var("SAAIOS_LOCAL_BASE").ok(),
        std::env::var("SAAIOS_LOCAL_MODEL").ok(),
    )
    .await
    .context("build model provider")?;
    info!(provider = provider.name(), kind = ?kind, "model provider ready");

    let budgets = ResourceBudgets {
        max_concurrent_requests: args.max_concurrent.max(1),
        request_timeout: Duration::from_secs(args.request_timeout_secs.max(1)),
        max_tool_iters: 6,
    };
    info!(
        max_concurrent = budgets.max_concurrent_requests,
        timeout_secs = budgets.request_timeout.as_secs(),
        "AI resource budgets"
    );

    let mut runtime =
        AiRuntime::with_budgets(tools, policy, audit.clone(), bus.clone(), provider, budgets);
    if let Some(mem) = memory.clone() {
        runtime = runtime.with_memory(mem);
    }
    let runtime = Arc::new(runtime);

    if !args.no_automation {
        let automation = Arc::new(AutomationEngine::new(
            bus.clone(),
            audit.clone(),
            AutomationEngine::default_rules(),
        ));
        let _automation_worker = automation.spawn();
        info!("automation engine started");
    }

    let last_pending = Arc::new(tokio::sync::Mutex::new(None::<HandleOutcome>));

    if args.sock.exists() {
        let _ = std::fs::remove_file(&args.sock);
    }
    let listener =
        UnixListener::bind(&args.sock).with_context(|| format!("bind {}", args.sock.display()))?;
    info!(sock = %args.sock.display(), "SaaiOS runtime listening");

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
            audit_tail: None,
            session_grants: None,
            memory_facts: None,
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
                    audit_tail: None,
                    session_grants: Some(runtime.session_grants()),
                    memory_facts: None,
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
                audit_tail: None,
                session_grants: None,
                memory_facts: None,
            },
        },
        ClientRequest::Confirm {
            correlation_id,
            call_id,
            tool,
            arguments,
            scope,
            confirmed,
        } => {
            let scope = match confirmed {
                Some(false) => ConfirmScope::Cancel,
                Some(true) if matches!(scope, ConfirmScope::Once) => ConfirmScope::Once,
                _ => scope,
            };
            match runtime
                .confirm(correlation_id, call_id, &tool, arguments, scope)
                .await
            {
                Ok(result) => ClientResponse {
                    ok: true,
                    correlation_id: Some(correlation_id),
                    diagnose: None,
                    pending: None,
                    error: None,
                    tool_result: Some(result),
                    audit_tail: None,
                    session_grants: Some(runtime.session_grants()),
                    memory_facts: None,
                },
                Err(e) => ClientResponse {
                    ok: false,
                    correlation_id: Some(correlation_id),
                    diagnose: None,
                    pending: None,
                    error: Some(e.to_string()),
                    tool_result: None,
                    audit_tail: None,
                    session_grants: Some(runtime.session_grants()),
                    memory_facts: None,
                },
            }
        }
        ClientRequest::AuditTail { limit } => match runtime.audit_tail(limit) {
            Ok(tail) => ClientResponse {
                ok: true,
                correlation_id: None,
                diagnose: None,
                pending: None,
                error: None,
                tool_result: None,
                audit_tail: Some(tail),
                session_grants: None,
                memory_facts: None,
            },
            Err(e) => ClientResponse {
                ok: false,
                correlation_id: None,
                diagnose: None,
                pending: None,
                error: Some(e.to_string()),
                tool_result: None,
                audit_tail: None,
                session_grants: None,
                memory_facts: None,
            },
        },
        ClientRequest::MemoryRemember { key, value, tags } => match runtime.memory() {
            Some(store) => {
                let mut fact = MemoryFact::new(key, value);
                fact.tags = tags;
                fact.source = Some("console".into());
                match store.remember(fact) {
                    Ok(fact) => ClientResponse {
                        ok: true,
                        correlation_id: None,
                        diagnose: None,
                        pending: None,
                        error: None,
                        tool_result: None,
                        audit_tail: None,
                        session_grants: None,
                        memory_facts: Some(vec![fact]),
                    },
                    Err(e) => ClientResponse {
                        ok: false,
                        correlation_id: None,
                        diagnose: None,
                        pending: None,
                        error: Some(e.to_string()),
                        tool_result: None,
                        audit_tail: None,
                        session_grants: None,
                        memory_facts: None,
                    },
                }
            }
            None => ClientResponse {
                ok: false,
                correlation_id: None,
                diagnose: None,
                pending: None,
                error: Some("memory disabled (--no-memory)".into()),
                tool_result: None,
                audit_tail: None,
                session_grants: None,
                memory_facts: None,
            },
        },
        ClientRequest::MemoryRecall { query } => match runtime.memory() {
            Some(store) => match store.recall(&query) {
                Ok(facts) => ClientResponse {
                    ok: true,
                    correlation_id: None,
                    diagnose: None,
                    pending: None,
                    error: None,
                    tool_result: None,
                    audit_tail: None,
                    session_grants: None,
                    memory_facts: Some(facts),
                },
                Err(e) => ClientResponse {
                    ok: false,
                    correlation_id: None,
                    diagnose: None,
                    pending: None,
                    error: Some(e.to_string()),
                    tool_result: None,
                    audit_tail: None,
                    session_grants: None,
                    memory_facts: None,
                },
            },
            None => ClientResponse {
                ok: false,
                correlation_id: None,
                diagnose: None,
                pending: None,
                error: Some("memory disabled (--no-memory)".into()),
                tool_result: None,
                audit_tail: None,
                session_grants: None,
                memory_facts: None,
            },
        },
        ClientRequest::MemoryTail { limit } => match runtime.memory() {
            Some(store) => match store.list_recent(limit) {
                Ok(facts) => ClientResponse {
                    ok: true,
                    correlation_id: None,
                    diagnose: None,
                    pending: None,
                    error: None,
                    tool_result: None,
                    audit_tail: None,
                    session_grants: None,
                    memory_facts: Some(facts),
                },
                Err(e) => ClientResponse {
                    ok: false,
                    correlation_id: None,
                    diagnose: None,
                    pending: None,
                    error: Some(e.to_string()),
                    tool_result: None,
                    audit_tail: None,
                    session_grants: None,
                    memory_facts: None,
                },
            },
            None => ClientResponse {
                ok: false,
                correlation_id: None,
                diagnose: None,
                pending: None,
                error: Some("memory disabled (--no-memory)".into()),
                tool_result: None,
                audit_tail: None,
                session_grants: None,
                memory_facts: None,
            },
        },
        ClientRequest::MemoryForget { key } => match runtime.memory() {
            Some(store) => match store.forget(&key) {
                Ok(Some(_)) => ClientResponse {
                    ok: true,
                    correlation_id: None,
                    diagnose: None,
                    pending: None,
                    error: None,
                    tool_result: None,
                    audit_tail: None,
                    session_grants: None,
                    memory_facts: Some(vec![]),
                },
                Ok(None) => ClientResponse {
                    ok: false,
                    correlation_id: None,
                    diagnose: None,
                    pending: None,
                    error: Some(format!("no fact for key={key}")),
                    tool_result: None,
                    audit_tail: None,
                    session_grants: None,
                    memory_facts: None,
                },
                Err(e) => ClientResponse {
                    ok: false,
                    correlation_id: None,
                    diagnose: None,
                    pending: None,
                    error: Some(e.to_string()),
                    tool_result: None,
                    audit_tail: None,
                    session_grants: None,
                    memory_facts: None,
                },
            },
            None => ClientResponse {
                ok: false,
                correlation_id: None,
                diagnose: None,
                pending: None,
                error: Some("memory disabled (--no-memory)".into()),
                tool_result: None,
                audit_tail: None,
                session_grants: None,
                memory_facts: None,
            },
        },
        ClientRequest::SessionGrants => ClientResponse {
            ok: true,
            correlation_id: None,
            diagnose: None,
            pending: None,
            error: None,
            tool_result: None,
            audit_tail: None,
            session_grants: Some(runtime.session_grants()),
            memory_facts: None,
        },
        ClientRequest::ClearSessionGrants => {
            runtime.clear_session_grants();
            ClientResponse {
                ok: true,
                correlation_id: None,
                diagnose: None,
                pending: None,
                error: None,
                tool_result: None,
                audit_tail: None,
                session_grants: Some(vec![]),
                memory_facts: None,
            }
        }
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
