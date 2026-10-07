use anyhow::Result;
use audit_log::AuditLog;
use chrono::Utc;
use event_bus::EventBus;
use protocol::{Envelope, MessageKind, ToolCallResult};
use saai_observation::{
    is_cellular_iface, observations_from_cellular, observations_from_system_metrics,
    CellularReading, IfaceSample, MetricsOrigin, ObservationCache, WorldSnapshot,
};
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
/// WORLD-02: the same sample also fills an in-process ObservationCache.
/// High-frequency rows stay out of entity-store.
pub struct TelemetrySampler {
    tools: Arc<ToolRegistry>,
    bus: EventBus,
    audit: Arc<AuditLog>,
    interval: Duration,
    samples: AtomicU64,
    started: Instant,
    cache: Option<Arc<ObservationCache>>,
    origin: MetricsOrigin,
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
            cache: None,
            origin: MetricsOrigin::Mock,
        }
    }

    pub fn with_cache(mut self, cache: Arc<ObservationCache>, origin: MetricsOrigin) -> Self {
        self.cache = Some(cache);
        self.origin = origin;
        self
    }

    pub fn observation_snapshot(&self) -> Option<WorldSnapshot> {
        self.cache.as_ref().map(|cache| cache.snapshot(Utc::now()))
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
                    space_id: None,
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
        let sequence = self.samples.fetch_add(1, Ordering::Relaxed) + 1;
        if result.ok {
            if let Some(cache) = &self.cache {
                let rows = observations_from_system_metrics(
                    &result.output,
                    self.origin,
                    Utc::now(),
                    sequence,
                );
                cache.apply(rows);
            }
        }
        if self.origin == MetricsOrigin::Procfs {
            if let Some(cache) = &self.cache {
                let sequence = self.samples.load(Ordering::Relaxed);
                cache.apply(observations_from_cellular(
                    &read_cellular_reading(),
                    Utc::now(),
                    sequence,
                ));
            }
        }
        debug!(%correlation_id, "telemetry sample published");
        Ok(env)
    }
}

fn read_cellular_reading() -> CellularReading {
    let cp_text = std::fs::read_to_string("/sys/devices/platform/cpif/modem_state").ok();
    let owner_log = read_log_tail("/data/saaios/var/modem-rfs-camp-combined-owner.log");
    let supervisor_log = read_log_tail("/run/modem-boot.log");
    let ifaces = std::fs::read_dir("/sys/class/net")
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| is_cellular_iface(name))
        .map(|name| IfaceSample {
            has_ipv4: iface_has_ipv4(&name),
            rx: iface_counter(&name, "rx_bytes"),
            tx: iface_counter(&name, "tx_bytes"),
            name,
        })
        .collect();
    CellularReading {
        cp_text,
        owner_log,
        supervisor_log,
        owner_running: camp_owner_running(),
        ifaces,
    }
}

fn camp_owner_running() -> bool {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return false;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !name.bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }
        let cmdline = std::fs::read(entry.path().join("cmdline")).unwrap_or_default();
        let argv0 = cmdline.split(|byte| *byte == 0).next().unwrap_or(b"");
        if String::from_utf8_lossy(argv0).ends_with("/modem-rfs-camp-combined-owner") {
            return true;
        }
    }
    false
}

fn read_log_tail(path: &str) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    let max = 256 * 1024u64;
    if len > max {
        use std::io::Seek;
        file.seek(std::io::SeekFrom::End(-(max as i64))).ok()?;
    }
    let mut text = String::new();
    use std::io::Read;
    file.read_to_string(&mut text).ok()?;
    Some(text)
}

fn iface_counter(name: &str, which: &str) -> u64 {
    if name.contains('/') {
        return 0;
    }
    std::fs::read_to_string(format!("/sys/class/net/{name}/statistics/{which}"))
        .ok()
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or(0)
}

fn iface_has_ipv4(name: &str) -> bool {
    if name.contains('/') {
        return false;
    }
    let Ok(out) = std::process::Command::new("/saaios/ip")
        .args(["-4", "-o", "addr", "show", "dev", name])
        .output()
    else {
        return false;
    };
    out.status.success() && out.stdout.windows(5).any(|window| window == b"inet ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use system_tools::{install_system_tools, system_identity, ToolsMode};
    use tempfile::tempdir;
    use tool_registry::ToolRegistry;

    #[tokio::test]
    async fn sample_publishes_tool_result() {
        let dir = tempdir().unwrap();
        let audit = Arc::new(AuditLog::open(dir.path().join("a.jsonl")).unwrap());
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();
        let mut reg = ToolRegistry::new();
        install_system_tools(&mut reg, ToolsMode::Mock, system_identity(ToolsMode::Mock));
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
        assert!(sampler.observation_snapshot().is_none());
    }

    #[tokio::test]
    async fn sample_fills_observation_cache_without_breaking_tool_result() {
        let dir = tempdir().unwrap();
        let audit = Arc::new(AuditLog::open(dir.path().join("a.jsonl")).unwrap());
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();
        let mut reg = ToolRegistry::new();
        install_system_tools(&mut reg, ToolsMode::Mock, system_identity(ToolsMode::Mock));
        let cache = Arc::new(ObservationCache::new());
        let sampler = Arc::new(
            TelemetrySampler::new(Arc::new(reg), bus, audit, Duration::from_secs(60))
                .with_cache(cache.clone(), MetricsOrigin::Mock),
        );
        let env = sampler.sample_once().await.unwrap();
        assert_eq!(env.kind, MessageKind::ToolResult);
        assert_eq!(env.payload["tool"], "system.metrics");
        assert!(env.payload["ok"].as_bool().unwrap());
        let got = rx.recv().await.unwrap();
        assert_eq!(got.msg_id, env.msg_id);
        let snap = sampler.observation_snapshot().expect("cache attached");
        assert_eq!(snap.revision, 1);
        assert!(!snap.observations.is_empty());
        assert!(snap
            .observations
            .iter()
            .all(|row| !row.source.source_id.is_empty()));
        assert_eq!(cache.snapshot(Utc::now()).revision, 1);
    }
}
