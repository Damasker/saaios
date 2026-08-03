use anyhow::Result;
use audit_log::AuditLog;
use event_bus::EventBus;
use protocol::{Envelope, MessageKind, ToolCallResult};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tool_registry::{ToolContext, ToolRegistry};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Periodic host telemetry that publishes `system.metrics` tool results
/// onto the event bus so automation can fire without a user request.
pub struct TelemetrySampler {
    tools: Arc<ToolRegistry>,
    bus: EventBus,
    audit: Arc<AuditLog>,
    interval: Duration,
    samples: AtomicU64,
    started: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryStats {
    pub samples: u64,
    pub interval_secs: u64,
    pub uptime_secs: u64,
}

impl TelemetrySampler {
    pub fn new(
        tools: Arc<ToolRegistry>,
        bus: EventBus,
        audit: Arc<AuditLog>,
        interval: Duration,
    ) -> Self {
        Self {
            tools,
            bus,
            audit,
            interval: interval.max(Duration::from_secs(1)),
            samples: AtomicU64::new(0),
            started: Instant::now(),
        }
    }

    pub fn stats(&self) -> TelemetryStats {
        TelemetryStats {
            samples: self.samples.load(Ordering::Relaxed),
            interval_secs: self.interval.as_secs(),
            uptime_secs: self.started.elapsed().as_secs(),
        }
    }

    pub fn spawn(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!(
                interval_secs = self.interval.as_secs(),
                "telemetry sampler started"
            );
            let mut ticker = tokio::time::interval(self.interval);
            // First tick completes immediately; skip so we don't spike at boot.
            ticker.tick().await;
            loop {
                ticker.tick().await;
                if let Err(e) = self.sample_once().await {
                    warn!(error = %e, "telemetry sample failed");
                }
            }
        })
    }

    pub async fn sample_once(&self) -> Result<Envelope> {
        let correlation_id = Uuid::new_v4();
        let call_id = Uuid::new_v4();
        let output = self
            .tools
            .execute(
                "system.metrics",
                json!({}),
                &ToolContext {
                    correlation_id,
                    call_id,
                },
            )
            .await?;

        let result = ToolCallResult {
            call_id,
            tool: "system.metrics".into(),
            ok: output.ok,
            output: output.value,
            error: output.error,
        };
        let env = Envelope::new(
            MessageKind::ToolResult,
            correlation_id,
            None,
            serde_json::to_value(&result)?,
        );
        self.audit.append_envelope(&env)?;
        self.bus.publish_envelope(env.clone());
        self.samples.fetch_add(1, Ordering::Relaxed);
        debug!(%correlation_id, "telemetry sample published");
        Ok(env)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use system_tools::{install_system_tools, ToolsMode};
    use tempfile::tempdir;
    use tool_registry::ToolRegistry;

    #[tokio::test]
    async fn sample_publishes_tool_result() {
        let dir = tempdir().unwrap();
        let audit = Arc::new(AuditLog::open(dir.path().join("a.jsonl")).unwrap());
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();
        let mut reg = ToolRegistry::new();
        install_system_tools(&mut reg, ToolsMode::Mock);
        let sampler = Arc::new(TelemetrySampler::new(
            Arc::new(reg),
            bus,
            audit,
            Duration::from_secs(60),
        ));
        let env = sampler.sample_once().await.unwrap();
        assert_eq!(env.kind, MessageKind::ToolResult);
        assert_eq!(env.payload["tool"], "system.metrics");
        assert!(env.payload["ok"].as_bool().unwrap());
        let got = rx.recv().await.unwrap();
        assert_eq!(got.msg_id, env.msg_id);
        assert_eq!(sampler.stats().samples, 1);
    }
}
