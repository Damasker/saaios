//! Convert existing `system.metrics` JSON into Observations.
//! Does not re-read /proc; that stays in `system-tools` until WORLD-04.

use crate::model::{
    Observation, ObservationQuality, ObservationSource, ObservationSourceKind, ObservationSubject,
    SCHEMA_VERSION,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

pub const KEY_CPU_USAGE: &str = "system.cpu.usage";
pub const KEY_LOAD_AVERAGE: &str = "system.load.average";
pub const KEY_MEMORY_USED_PERCENT: &str = "system.memory.used_percent";

/// CPU/load TTL in the 2–5s band from ADR-122. Memory uses the same until
/// ObservationSpec exists.
pub const METRICS_CPU_TTL: u64 = 5_000;
pub const METRICS_LOAD_TTL: u64 = 5_000;
pub const METRICS_MEMORY_TTL: u64 = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricsOrigin {
    Mock,
    Procfs,
}

#[derive(Debug, Deserialize)]
struct MetricsJson {
    cpu_usage: Option<f64>,
    load_average: Option<f64>,
    mem_used_pct: Option<f64>,
    mem_used_mb: Option<f64>,
    mem_total_mb: Option<f64>,
}

pub fn observations_from_system_metrics(
    value: &Value,
    origin: MetricsOrigin,
    observed_at: DateTime<Utc>,
    sequence: u64,
) -> Vec<Observation> {
    let parsed: MetricsJson = match serde_json::from_value(value.clone()) {
        Ok(parsed) => parsed,
        Err(_) => return Vec::new(),
    };

    let mut out = Vec::new();
    if let Some(cpu) = finite(parsed.cpu_usage) {
        out.push(observation(
            Draft {
                key: KEY_CPU_USAGE,
                number: cpu,
                unit: Some("percent"),
                source_id: cpu_source(origin),
                quality: ObservationQuality::Direct,
                kind: ObservationSourceKind::Direct,
                valid_for_ms: METRICS_CPU_TTL,
            },
            observed_at,
            sequence,
        ));
    }
    if let Some(load) = finite(parsed.load_average) {
        out.push(observation(
            Draft {
                key: KEY_LOAD_AVERAGE,
                number: load,
                unit: None,
                source_id: load_source(origin),
                quality: ObservationQuality::Direct,
                kind: ObservationSourceKind::Direct,
                valid_for_ms: METRICS_LOAD_TTL,
            },
            observed_at,
            sequence,
        ));
    }
    let mem_pct = finite(parsed.mem_used_pct).or_else(|| {
        match (finite(parsed.mem_used_mb), finite(parsed.mem_total_mb)) {
            (Some(used), Some(total)) if total > 0.0 => Some((used / total) * 100.0),
            _ => None,
        }
    });
    if let Some(pct) = mem_pct {
        out.push(observation(
            Draft {
                key: KEY_MEMORY_USED_PERCENT,
                number: pct,
                unit: Some("percent"),
                source_id: "derived.memory_percentage",
                quality: ObservationQuality::Derived,
                kind: ObservationSourceKind::Derived,
                valid_for_ms: METRICS_MEMORY_TTL,
            },
            observed_at,
            sequence,
        ));
    }
    out
}

fn cpu_source(origin: MetricsOrigin) -> &'static str {
    match origin {
        MetricsOrigin::Mock => "mock.system.metrics",
        MetricsOrigin::Procfs => "procfs.cpu",
    }
}

fn load_source(origin: MetricsOrigin) -> &'static str {
    match origin {
        MetricsOrigin::Mock => "mock.system.metrics",
        MetricsOrigin::Procfs => "procfs.load",
    }
}

fn finite(value: Option<f64>) -> Option<f64> {
    value.filter(|n| n.is_finite())
}

fn observation(draft: Draft, observed_at: DateTime<Utc>, sequence: u64) -> Observation {
    Observation {
        schema: SCHEMA_VERSION,
        id: Uuid::new_v4(),
        subject: ObservationSubject::LocalDevice,
        key: draft.key.into(),
        value: Value::from(draft.number),
        unit: draft.unit.map(str::to_string),
        source: ObservationSource {
            source_id: draft.source_id.into(),
            kind: draft.kind,
        },
        quality: draft.quality,
        observed_at,
        valid_for_ms: draft.valid_for_ms,
        sequence,
    }
}

struct Draft {
    key: &'static str,
    number: f64,
    unit: Option<&'static str>,
    source_id: &'static str,
    quality: ObservationQuality,
    kind: ObservationSourceKind,
    valid_for_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{freshness_at, Freshness};
    use chrono::Duration as ChronoDuration;
    use serde_json::json;

    fn mock_metrics() -> Value {
        json!({
            "cpu_usage": 97.0,
            "load_average": 8.4,
            "mem_used_mb": 5200.0,
            "mem_total_mb": 8192.0,
            "mem_used_pct": 63.5,
            "disk_read_mb_s": 120.0
        })
    }

    fn by_key<'a>(rows: &'a [Observation], key: &str) -> &'a Observation {
        rows.iter().find(|o| o.key == key).expect("key present")
    }

    #[test]
    fn mock_metrics_become_observations() {
        let now = Utc::now();
        let rows = observations_from_system_metrics(&mock_metrics(), MetricsOrigin::Mock, now, 7);
        assert_eq!(rows.len(), 3);
        let cpu = by_key(&rows, KEY_CPU_USAGE);
        assert_eq!(cpu.value, json!(97.0));
        assert_eq!(cpu.unit.as_deref(), Some("percent"));
        assert_eq!(cpu.source.source_id, "mock.system.metrics");
        assert_eq!(cpu.quality, ObservationQuality::Direct);
        assert_eq!(cpu.observed_at, now);
        assert_eq!(cpu.sequence, 7);
        assert_eq!(cpu.subject, ObservationSubject::LocalDevice);
        assert_eq!(by_key(&rows, KEY_LOAD_AVERAGE).value, json!(8.4));
        let mem = by_key(&rows, KEY_MEMORY_USED_PERCENT);
        assert_eq!(mem.value, json!(63.5));
        assert_eq!(mem.quality, ObservationQuality::Derived);
        assert_eq!(mem.source.source_id, "derived.memory_percentage");
    }

    #[test]
    fn stale_after_metrics_ttl() {
        let t0 = Utc::now();
        let rows = observations_from_system_metrics(&mock_metrics(), MetricsOrigin::Mock, t0, 1);
        let cpu = by_key(&rows, KEY_CPU_USAGE);
        assert_eq!(freshness_at(cpu, t0), Freshness::Fresh);
        assert_eq!(
            freshness_at(
                cpu,
                t0 + ChronoDuration::milliseconds(METRICS_CPU_TTL as i64 + 1)
            ),
            Freshness::Stale
        );
    }

    #[test]
    fn nan_cpu_is_dropped() {
        let rows = observations_from_system_metrics(
            &json!({
                "cpu_usage": null,
                "load_average": 1.0,
                "mem_used_pct": 10.0
            }),
            MetricsOrigin::Mock,
            Utc::now(),
            1,
        );
        assert!(rows.iter().all(|o| o.key != KEY_CPU_USAGE));
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn derives_memory_percent_when_pct_missing() {
        let rows = observations_from_system_metrics(
            &json!({
                "cpu_usage": 1.0,
                "load_average": 0.2,
                "mem_used_mb": 25.0,
                "mem_total_mb": 100.0
            }),
            MetricsOrigin::Procfs,
            Utc::now(),
            1,
        );
        let mem = by_key(&rows, KEY_MEMORY_USED_PERCENT);
        assert_eq!(mem.value, json!(25.0));
        assert_eq!(mem.source.source_id, "derived.memory_percentage");
        assert_eq!(by_key(&rows, KEY_CPU_USAGE).source.source_id, "procfs.cpu");
    }
}
