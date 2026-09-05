use ai_runtime::{diagnose_slow_with_mock_planner, AiRuntime, HandleOutcome, ResourceBudgets};
use anyhow::{anyhow, Context, Result};
use audit_log::AuditLog;
use automation_engine::AutomationEngine;
use chrono::Utc;
use clap::Parser;
use config::{resolve, CliOverrides};
use event_bus::EventBus;
use memory_store::{install_memory_tools, MemoryFact, MemoryStore};
use model_provider::{build_provider, ProviderKind};
use policy_engine::PolicyEngine;
use protocol::{ConfirmScope, Envelope, MessageKind};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use system_tools::{install_system_tools, ToolsMode};
use telemetry::TelemetrySampler;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, UnixListener};
use tool_registry::ToolRegistry;
use tracing::{error, info, warn};
use uuid::Uuid;

mod event_feed;
mod ab_status;
use event_feed::{EventFeed, FeedItem};
use ab_status::{ab_root_from_env, read_ab_status, AbStatus};

#[derive(Debug, Parser)]
#[command(name = "saaios-runtime", about = "SaaiOS Platform runtime")]
struct Args {
    /// Path to saaios.toml (also: SAAIOS_CONFIG, ./saaios.toml, /etc/saaios/saaios.toml)
    #[arg(long, env = "SAAIOS_CONFIG")]
    config: Option<PathBuf>,

    /// Use mock tools + mock model provider (alias for --provider mock)
    #[arg(long, env = "SAAIOS_MOCK")]
    mock: bool,

    /// Model provider: mock | remote | local | auto
    #[arg(long, env = "SAAIOS_PROVIDER")]
    provider: Option<String>,

    /// Use deterministic mock planner (no model loop)
    #[arg(long)]
    mock_planner: bool,

    /// Unix domain socket path
    #[arg(long, env = "SAAIOS_SOCK")]
    sock: Option<PathBuf>,

    /// TCP listen address (for hosts such as Android where filesystem UDS bind is denied)
    #[arg(long, env = "SAAIOS_TCP")]
    tcp: Option<String>,

    /// Audit log path
    #[arg(long, env = "SAAIOS_AUDIT")]
    audit: Option<PathBuf>,

    /// Local memory / facts JSONL path
    #[arg(long, env = "SAAIOS_MEMORY")]
    memory: Option<PathBuf>,

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
    #[arg(long, env = "SAAIOS_MAX_CONCURRENT")]
    max_concurrent: Option<usize>,

    /// AI request timeout in seconds
    #[arg(long, env = "SAAIOS_REQUEST_TIMEOUT_SECS")]
    request_timeout_secs: Option<u64>,

    /// Max tool-loop iterations per request
    #[arg(long, env = "SAAIOS_MAX_TOOL_ITERS")]
    max_tool_iters: Option<usize>,

    /// Disable event-driven automation worker
    #[arg(long)]
    no_automation: bool,

    /// Enable auto-diagnose worker (runs diagnose on AutomationAutoDiagnose)
    #[arg(long)]
    auto_diagnose: bool,

    /// Disable auto-diagnose even if enabled in config
    #[arg(long)]
    no_auto_diagnose: bool,

    /// Enable background telemetry sampler
    #[arg(long)]
    telemetry: bool,

    /// Disable background telemetry sampler
    #[arg(long)]
    no_telemetry: bool,

    /// Telemetry sample interval in seconds
    #[arg(long, env = "SAAIOS_TELEMETRY_INTERVAL_SECS")]
    telemetry_interval_secs: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum ClientRequest {
    Diagnose {
        text: String,
        #[serde(default)]
        session_id: Option<Uuid>,
        /// When true, respond with NDJSON progress frames then a final `done` object.
        #[serde(default)]
        stream: bool,
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
        #[serde(default)]
        session_id: Option<Uuid>,
    },
    ChatReset {
        session_id: Uuid,
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
    Status,
    EventsTail {
        #[serde(default = "default_tail")]
        limit: usize,
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

#[derive(Debug, Serialize, Deserialize, Default)]
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
    audit_tail: Option<Vec<audit_log::AuditRecord>>,
    #[serde(default)]
    session_grants: Option<Vec<String>>,
    #[serde(default)]
    memory_facts: Option<Vec<MemoryFact>>,
    #[serde(default)]
    status: Option<RuntimeStatusDto>,
    #[serde(default)]
    events: Option<Vec<FeedItem>>,
    #[serde(default)]
    progress: Option<Vec<ai_runtime::RuntimeEvent>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PendingDto {
    call_id: Uuid,
    tool: String,
    arguments: serde_json::Value,
    summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuntimeStatusDto {
    ok: bool,
    version: String,
    uptime_secs: u64,
    sock: String,
    provider: String,
    provider_kind: String,
    tools_mode: String,
    tool_count: usize,
    tools: Vec<String>,
    memory_enabled: bool,
    memory_path: Option<String>,
    audit_path: String,
    automation: bool,
    telemetry: bool,
    telemetry_samples: u64,
    telemetry_interval_secs: u64,
    max_concurrent: usize,
    request_timeout_secs: u64,
    session_grants: Vec<String>,
    #[serde(default)]
    config_path: Option<String>,
    #[serde(default)]
    auto_diagnose: bool,
    #[serde(default)]
    chat_sessions: usize,
    #[serde(default)]
    ab: Option<AbStatus>,
}

struct RuntimeMeta {
    started: Instant,
    config_path: Option<PathBuf>,
    provider_name: String,
    provider_kind: String,
    tools_mode: String,
    tool_names: Vec<String>,
    memory_enabled: bool,
    memory_path: Option<PathBuf>,
    audit_path: PathBuf,
    sock: PathBuf,
    automation: bool,
    auto_diagnose: bool,
    telemetry: Option<Arc<TelemetrySampler>>,
    max_concurrent: usize,
    request_timeout_secs: u64,
}

impl RuntimeMeta {
    fn status(&self, runtime: &AiRuntime) -> RuntimeStatusDto {
        let tel = self.telemetry.as_ref().map(|t| t.stats());
        RuntimeStatusDto {
            ok: true,
            version: env!("CARGO_PKG_VERSION").into(),
            uptime_secs: self.started.elapsed().as_secs(),
            sock: self.sock.display().to_string(),
            provider: self.provider_name.clone(),
            provider_kind: self.provider_kind.clone(),
            tools_mode: self.tools_mode.clone(),
            tool_count: self.tool_names.len(),
            tools: self.tool_names.clone(),
            memory_enabled: self.memory_enabled,
            memory_path: self.memory_path.as_ref().map(|p| p.display().to_string()),
            audit_path: self.audit_path.display().to_string(),
            automation: self.automation,
            telemetry: self.telemetry.is_some(),
            telemetry_samples: tel.as_ref().map(|t| t.samples).unwrap_or(0),
            telemetry_interval_secs: tel.as_ref().map(|t| t.interval_secs).unwrap_or(0),
            max_concurrent: self.max_concurrent,
            request_timeout_secs: self.request_timeout_secs,
            session_grants: runtime.session_grants(),
            config_path: self.config_path.as_ref().map(|p| p.display().to_string()),
            auto_diagnose: self.auto_diagnose,
            chat_sessions: runtime.conversation_count(),
            ab: Some(read_ab_status(&ab_root_from_env())),
        }
    }
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
    let settings = resolve(CliOverrides {
        config: args.config.clone(),
        mock: args.mock,
        mock_planner: args.mock_planner,
        provider: args.provider.clone(),
        sock: args.sock.clone(),
        audit: args.audit.clone(),
        memory: args.memory.clone(),
        no_memory: args.no_memory,
        real_linux: args.real_linux,
        max_concurrent: args.max_concurrent,
        request_timeout_secs: args.request_timeout_secs,
        max_tool_iters: args.max_tool_iters,
        no_automation: args.no_automation,
        auto_diagnose: args.auto_diagnose,
        no_auto_diagnose: args.no_auto_diagnose,
        telemetry: args.telemetry,
        no_telemetry: args.no_telemetry,
        telemetry_interval_secs: args.telemetry_interval_secs,
    })?;

    if let Some(path) = &settings.config_path {
        info!(path = %path.display(), "loaded config");
    }

    if let Some(correlation_id) = args.replay {
        let audit = AuditLog::open(&settings.audit)?;
        let report = audit.replay(correlation_id)?;
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    let tools_mode = if settings.real_linux {
        ToolsMode::RealLinux
    } else {
        ToolsMode::Mock
    };

    let mut registry = ToolRegistry::new();
    install_system_tools(&mut registry, tools_mode);

    let memory = if !settings.memory_enabled {
        None
    } else {
        let store = Arc::new(MemoryStore::open(&settings.memory_path)?);
        install_memory_tools(&mut registry, store.clone());
        info!(path = %settings.memory_path.display(), "memory store ready");
        Some(store)
    };

    let tools = Arc::new(registry);
    let policy = Arc::new(PolicyEngine::new());
    let audit = Arc::new(AuditLog::open(&settings.audit)?);
    let bus = EventBus::new(64);

    if settings.mock_planner {
        info!("running one-shot mock planner demo");
        let outcome =
            diagnose_slow_with_mock_planner(tools.clone(), policy.clone(), audit.clone()).await?;
        print_outcome(&outcome);
        info!(correlation_id = %outcome.correlation_id, "audit written");
        info!(sock = %settings.sock.display(), "starting UDS server");
    }

    let kind = if settings.mock_planner {
        ProviderKind::Mock
    } else {
        ProviderKind::parse(&settings.provider_kind).ok_or_else(|| {
            anyhow!(
                "unknown provider {:?}; expected mock|remote|local|auto",
                settings.provider_kind
            )
        })?
    };
    let provider = build_provider(
        kind,
        settings.api_base.clone(),
        settings.api_key.clone(),
        settings.model.clone(),
        settings.local_base.clone(),
        settings.local_model.clone(),
    )
    .await
    .context("build model provider")?;
    info!(provider = provider.name(), kind = ?kind, "model provider ready");

    let budgets = ResourceBudgets {
        max_concurrent_requests: settings.max_concurrent,
        request_timeout: Duration::from_secs(settings.request_timeout_secs),
        max_tool_iters: settings.max_tool_iters,
    };
    info!(
        max_concurrent = budgets.max_concurrent_requests,
        timeout_secs = budgets.request_timeout.as_secs(),
        max_tool_iters = budgets.max_tool_iters,
        "AI resource budgets"
    );

    let provider_name = provider.name().to_string();
    let max_concurrent = budgets.max_concurrent_requests;
    let request_timeout_secs = budgets.request_timeout.as_secs();
    let mut runtime = AiRuntime::with_budgets(
        tools.clone(),
        policy,
        audit.clone(),
        bus.clone(),
        provider,
        budgets,
    );
    if let Some(mem) = memory.clone() {
        runtime = runtime.with_memory(mem);
    }
    let runtime = Arc::new(runtime);
    let feed = Arc::new(EventFeed::new(64));

    let automation_enabled = settings.automation_enabled;
    if automation_enabled {
        let mut engine = AutomationEngine::new(
            bus.clone(),
            audit.clone(),
            AutomationEngine::default_rules(),
        );
        engine.set_auto_diagnose_rule(settings.auto_diagnose);
        let automation = Arc::new(engine);
        let _automation_worker = automation.spawn();
        info!(
            auto_diagnose = settings.auto_diagnose,
            "automation engine started"
        );
    }

    let _feed_worker = spawn_event_feed_worker(bus.clone(), feed.clone());

    if settings.auto_diagnose {
        let _auto =
            spawn_auto_diagnose_worker(runtime.clone(), bus.clone(), audit.clone(), feed.clone());
        info!("auto-diagnose worker started");
    }

    let telemetry = if !settings.telemetry_enabled {
        None
    } else {
        let sampler = Arc::new(TelemetrySampler::new(
            tools.clone(),
            bus.clone(),
            audit.clone(),
            Duration::from_secs(settings.telemetry_interval_secs),
        ));
        let _tel = sampler.clone().spawn();
        info!(
            interval_secs = settings.telemetry_interval_secs,
            "telemetry sampler started"
        );
        Some(sampler)
    };

    let mut tool_names: Vec<_> = tools.list().into_iter().map(|t| t.name).collect();
    tool_names.sort();

    let meta = Arc::new(RuntimeMeta {
        started: Instant::now(),
        config_path: settings.config_path.clone(),
        provider_name,
        provider_kind: format!("{kind:?}").to_lowercase(),
        tools_mode: format!("{tools_mode:?}").to_lowercase(),
        tool_names,
        memory_enabled: memory.is_some(),
        memory_path: if memory.is_some() {
            Some(settings.memory_path.clone())
        } else {
            None
        },
        audit_path: settings.audit.clone(),
        sock: settings.sock.clone(),
        automation: automation_enabled,
        auto_diagnose: settings.auto_diagnose,
        telemetry,
        max_concurrent,
        request_timeout_secs,
    });

    let last_pending = Arc::new(tokio::sync::Mutex::new(None::<HandleOutcome>));

    if let Some(addr) = args.tcp.as_deref() {
        let listener = TcpListener::bind(addr)
            .await
            .with_context(|| format!("bind TCP {addr}"))?;
        info!(addr, "SaaiOS runtime listening on TCP");

        loop {
            let (stream, _) = listener.accept().await?;
            let runtime = runtime.clone();
            let last_pending = last_pending.clone();
            let meta = meta.clone();
            let feed = feed.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_client(stream, runtime, last_pending, meta, feed).await {
                    error!("client error: {e:#}");
                }
            });
        }
    } else {
        if settings.sock.exists() {
            let _ = std::fs::remove_file(&settings.sock);
        }
        let listener = UnixListener::bind(&settings.sock)
            .with_context(|| format!("bind {}", settings.sock.display()))?;
        info!(sock = %settings.sock.display(), "SaaiOS runtime listening");

        loop {
            let (stream, _) = listener.accept().await?;
            let runtime = runtime.clone();
            let last_pending = last_pending.clone();
            let meta = meta.clone();
            let feed = feed.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_client(stream, runtime, last_pending, meta, feed).await {
                    error!("client error: {e:#}");
                }
            });
        }
    }
}

fn spawn_event_feed_worker(bus: EventBus, feed: Arc<EventFeed>) -> tokio::task::JoinHandle<()> {
    let mut rx = bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(env) => {
                    if env.kind != MessageKind::Event {
                        continue;
                    }
                    let Some(name) = env.payload.get("event").and_then(|v| v.as_str()) else {
                        continue;
                    };
                    if !name.starts_with("Automation") {
                        continue;
                    }
                    let summary = env
                        .payload
                        .get("message")
                        .or_else(|| env.payload.get("prompt"))
                        .or_else(|| env.payload.get("summary"))
                        .and_then(|v| v.as_str())
                        .unwrap_or(name)
                        .to_string();
                    feed.push(FeedItem {
                        ts: Utc::now().to_rfc3339(),
                        event: name.to_string(),
                        correlation_id: Some(env.correlation_id),
                        summary,
                        payload: env.payload.clone(),
                    });
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    })
}

fn spawn_auto_diagnose_worker(
    runtime: Arc<AiRuntime>,
    bus: EventBus,
    audit: Arc<AuditLog>,
    feed: Arc<EventFeed>,
) -> tokio::task::JoinHandle<()> {
    let mut rx = bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(env) => {
                    if env.kind != MessageKind::Event {
                        continue;
                    }
                    let Some(name) = env.payload.get("event").and_then(|v| v.as_str()) else {
                        continue;
                    };
                    if name != "AutomationAutoDiagnose" {
                        continue;
                    }
                    let prompt = env
                        .payload
                        .get("prompt")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Почему система тормозит?")
                        .to_string();
                    info!(%prompt, "auto-diagnose triggered");
                    feed.push(FeedItem {
                        ts: Utc::now().to_rfc3339(),
                        event: "AutoDiagnoseStarted".into(),
                        correlation_id: Some(env.correlation_id),
                        summary: prompt.clone(),
                        payload: env.payload.clone(),
                    });
                    match runtime.handle_user_text(&prompt).await {
                        Ok(outcome) => {
                            let summary = if outcome.diagnose.summary.is_empty() {
                                "auto-diagnose completed".into()
                            } else {
                                outcome.diagnose.summary.clone()
                            };
                            let payload = json!({
                                "event": "AutomationDiagnoseCompleted",
                                "prompt": prompt,
                                "summary": summary,
                                "correlation_id": outcome.correlation_id,
                                "pending": outcome.pending_confirmation.is_some(),
                            });
                            let out = Envelope::new(
                                MessageKind::Event,
                                outcome.correlation_id,
                                Some(env.msg_id),
                                payload.clone(),
                            );
                            let _ = audit.append_envelope(&out);
                            bus.publish_envelope(out);
                            feed.push(FeedItem {
                                ts: Utc::now().to_rfc3339(),
                                event: "AutomationDiagnoseCompleted".into(),
                                correlation_id: Some(outcome.correlation_id),
                                summary: outcome.diagnose.summary,
                                payload,
                            });
                        }
                        Err(e) => {
                            warn!(error = %e, "auto-diagnose failed");
                            let payload = json!({
                                "event": "AutomationDiagnoseFailed",
                                "prompt": prompt,
                                "error": e.to_string(),
                            });
                            let out = Envelope::new(
                                MessageKind::Event,
                                env.correlation_id,
                                Some(env.msg_id),
                                payload.clone(),
                            );
                            let _ = audit.append_envelope(&out);
                            bus.publish_envelope(out);
                            feed.push(FeedItem {
                                ts: Utc::now().to_rfc3339(),
                                event: "AutomationDiagnoseFailed".into(),
                                correlation_id: Some(env.correlation_id),
                                summary: e.to_string(),
                                payload,
                            });
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    })
}

async fn handle_client<S>(
    mut stream: S,
    runtime: Arc<AiRuntime>,
    last_pending: Arc<tokio::sync::Mutex<Option<HandleOutcome>>>,
    meta: Arc<RuntimeMeta>,
    feed: Arc<EventFeed>,
) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut buf = vec![0u8; 64 * 1024];
    let n = stream.read(&mut buf).await?;
    if n == 0 {
        return Ok(());
    }
    let req: ClientRequest = serde_json::from_slice(&buf[..n])?;

    if let ClientRequest::Diagnose {
        text,
        session_id,
        stream: true,
    } = &req
    {
        handle_diagnose_stream(
            &mut stream,
            runtime,
            last_pending,
            text.clone(),
            *session_id,
        )
        .await?;
        return Ok(());
    }

    let response = match req {
        ClientRequest::Ping => ClientResponse {
            ok: true,
            ..Default::default()
        },
        ClientRequest::Status => ClientResponse {
            ok: true,
            status: Some(meta.status(&runtime)),
            ..Default::default()
        },
        ClientRequest::EventsTail { limit } => ClientResponse {
            ok: true,
            events: Some(feed.tail(limit)),
            ..Default::default()
        },
        ClientRequest::Diagnose {
            text,
            session_id,
            stream: _,
        } => match runtime
            .handle_user_text_in_session(&text, session_id, None)
            .await
        {
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
                    session_id: Some(outcome.session_id),
                    diagnose: Some(outcome.diagnose.clone()),
                    pending,
                    session_grants: Some(runtime.session_grants()),
                    progress: Some(outcome.events.clone()),
                    ..Default::default()
                };
                *last_pending.lock().await = Some(outcome);
                resp
            }
            Err(e) => ClientResponse {
                ok: false,
                session_id,
                error: Some(e.to_string()),
                ..Default::default()
            },
        },
        ClientRequest::ChatReset { session_id } => {
            let _ = runtime.reset_chat_session(session_id);
            ClientResponse {
                ok: true,
                session_id: Some(session_id),
                ..Default::default()
            }
        }
        ClientRequest::Confirm {
            correlation_id,
            call_id,
            tool,
            arguments,
            scope,
            confirmed,
            session_id,
        } => {
            let scope = match confirmed {
                Some(false) => ConfirmScope::Cancel,
                Some(true) if matches!(scope, ConfirmScope::Once) => ConfirmScope::Once,
                _ => scope,
            };
            match runtime
                .confirm_in_session(correlation_id, session_id, call_id, &tool, arguments, scope)
                .await
            {
                Ok(result) => ClientResponse {
                    ok: true,
                    correlation_id: Some(correlation_id),
                    session_id,
                    tool_result: Some(result),
                    session_grants: Some(runtime.session_grants()),
                    ..Default::default()
                },
                Err(e) => ClientResponse {
                    ok: false,
                    correlation_id: Some(correlation_id),
                    session_id,
                    error: Some(e.to_string()),
                    session_grants: Some(runtime.session_grants()),
                    ..Default::default()
                },
            }
        }
        ClientRequest::AuditTail { limit } => match runtime.audit_tail(limit) {
            Ok(tail) => ClientResponse {
                ok: true,
                audit_tail: Some(tail),
                ..Default::default()
            },
            Err(e) => ClientResponse {
                ok: false,
                error: Some(e.to_string()),
                ..Default::default()
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
                        memory_facts: Some(vec![fact]),
                        ..Default::default()
                    },
                    Err(e) => ClientResponse {
                        ok: false,
                        error: Some(e.to_string()),
                        ..Default::default()
                    },
                }
            }
            None => ClientResponse {
                ok: false,
                error: Some("memory disabled (--no-memory)".into()),
                ..Default::default()
            },
        },
        ClientRequest::MemoryRecall { query } => match runtime.memory() {
            Some(store) => match store.recall(&query) {
                Ok(facts) => ClientResponse {
                    ok: true,
                    memory_facts: Some(facts),
                    ..Default::default()
                },
                Err(e) => ClientResponse {
                    ok: false,
                    error: Some(e.to_string()),
                    ..Default::default()
                },
            },
            None => ClientResponse {
                ok: false,
                error: Some("memory disabled (--no-memory)".into()),
                ..Default::default()
            },
        },
        ClientRequest::MemoryTail { limit } => match runtime.memory() {
            Some(store) => match store.list_recent(limit) {
                Ok(facts) => ClientResponse {
                    ok: true,
                    memory_facts: Some(facts),
                    ..Default::default()
                },
                Err(e) => ClientResponse {
                    ok: false,
                    error: Some(e.to_string()),
                    ..Default::default()
                },
            },
            None => ClientResponse {
                ok: false,
                error: Some("memory disabled (--no-memory)".into()),
                ..Default::default()
            },
        },
        ClientRequest::MemoryForget { key } => match runtime.memory() {
            Some(store) => match store.forget(&key) {
                Ok(Some(_)) => ClientResponse {
                    ok: true,
                    memory_facts: Some(vec![]),
                    ..Default::default()
                },
                Ok(None) => ClientResponse {
                    ok: false,
                    error: Some(format!("no fact for key={key}")),
                    ..Default::default()
                },
                Err(e) => ClientResponse {
                    ok: false,
                    error: Some(e.to_string()),
                    ..Default::default()
                },
            },
            None => ClientResponse {
                ok: false,
                error: Some("memory disabled (--no-memory)".into()),
                ..Default::default()
            },
        },
        ClientRequest::SessionGrants => ClientResponse {
            ok: true,
            session_grants: Some(runtime.session_grants()),
            ..Default::default()
        },
        ClientRequest::ClearSessionGrants => {
            runtime.clear_session_grants();
            ClientResponse {
                ok: true,
                session_grants: Some(vec![]),
                ..Default::default()
            }
        }
    };

    let bytes = serde_json::to_vec(&response)?;
    stream.write_all(&bytes).await?;
    stream.shutdown().await?;
    Ok(())
}

async fn handle_diagnose_stream<S>(
    stream: &mut S,
    runtime: Arc<AiRuntime>,
    last_pending: Arc<tokio::sync::Mutex<Option<HandleOutcome>>>,
    text: String,
    session_id: Option<Uuid>,
) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let (tx, mut rx) = tokio::sync::mpsc::channel::<ai_runtime::RuntimeEvent>(64);
    let runtime2 = runtime.clone();
    let join = tokio::spawn(async move {
        runtime2
            .handle_user_text_in_session(&text, session_id, Some(tx))
            .await
    });

    while let Some(event) = rx.recv().await {
        let frame = json!({
            "type": "progress",
            "event": event,
        });
        let mut line = serde_json::to_vec(&frame)?;
        line.push(b'\n');
        stream.write_all(&line).await?;
    }

    let response = match join.await? {
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
                session_id: Some(outcome.session_id),
                diagnose: Some(outcome.diagnose.clone()),
                pending,
                session_grants: Some(runtime.session_grants()),
                progress: Some(outcome.events.clone()),
                ..Default::default()
            };
            *last_pending.lock().await = Some(outcome);
            resp
        }
        Err(e) => ClientResponse {
            ok: false,
            session_id,
            error: Some(e.to_string()),
            ..Default::default()
        },
    };

    let mut done = serde_json::to_value(&response)?;
    if let Some(obj) = done.as_object_mut() {
        obj.insert("type".into(), json!("done"));
    }
    let mut line = serde_json::to_vec(&done)?;
    line.push(b'\n');
    stream.write_all(&line).await?;
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
