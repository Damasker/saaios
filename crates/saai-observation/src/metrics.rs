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
pub const KEY_MEMORY_USED_MB: &str = "system.memory.used_mb";
pub const KEY_MEMORY_TOTAL_MB: &str = "system.memory.total_mb";
pub const KEY_STORAGE_USED_MB: &str = "system.storage.used_mb";
pub const KEY_STORAGE_TOTAL_MB: &str = "system.storage.total_mb";

/// CPU/load TTL in the 2–5s band from ADR-122. Memory uses the same until
/// ObservationSpec exists.
pub const METRICS_CPU_TTL: u64 = 5_000;
pub const METRICS_LOAD_TTL: u64 = 5_000;
pub const METRICS_MEMORY_TTL: u64 = 5_000;
/// Storage moves slowly and `df` is a subprocess, so it outlives several
/// telemetry ticks. A dead sampler still goes Stale after two minutes.
pub const METRICS_STORAGE_TTL: u64 = 120_000;

/// Data partition first (the user-visible capacity on a phone), then `/`.
const STORAGE_MOUNT_PREFERENCE: [&str; 2] = ["/data", "/"];

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
    if let Some((used, total)) = used_of_total(parsed.mem_used_mb, parsed.mem_total_mb) {
        let source_id = memory_source(origin);
        for (key, number) in [(KEY_MEMORY_USED_MB, used), (KEY_MEMORY_TOTAL_MB, total)] {
            out.push(observation(
                Draft {
                    key,
                    number,
                    unit: Some("megabyte"),
                    source_id,
                    quality: ObservationQuality::Direct,
                    kind: ObservationSourceKind::Direct,
                    valid_for_ms: METRICS_MEMORY_TTL,
                },
                observed_at,
                sequence,
            ));
        }
    }
    out
}

#[derive(Debug, Deserialize)]
struct DiskJson {
    #[serde(default)]
    mounts: Vec<MountJson>,
}

#[derive(Debug, Deserialize)]
struct MountJson {
    path: String,
    used_mb: Option<f64>,
    total_mb: Option<f64>,
}

/// Convert the existing `system.disk` tool JSON into storage Observations.
/// One mount only: `/data` when the device has it, otherwise `/`. Missing,
/// non-finite, or impossible numbers (used > total) yield no rows.
pub fn observations_from_system_disk(
    value: &Value,
    origin: MetricsOrigin,
    observed_at: DateTime<Utc>,
    sequence: u64,
) -> Vec<Observation> {
    let parsed: DiskJson = match serde_json::from_value(value.clone()) {
        Ok(parsed) => parsed,
        Err(_) => return Vec::new(),
    };
    let Some(mount) = STORAGE_MOUNT_PREFERENCE
        .iter()
        .find_map(|path| parsed.mounts.iter().find(|m| m.path == *path))
    else {
        return Vec::new();
    };
    let Some((used, total)) = used_of_total(mount.used_mb, mount.total_mb) else {
        return Vec::new();
    };
    let source_id = storage_source(origin);
    [(KEY_STORAGE_USED_MB, used), (KEY_STORAGE_TOTAL_MB, total)]
        .into_iter()
        .map(|(key, number)| {
            observation(
                Draft {
                    key,
                    number,
                    unit: Some("megabyte"),
                    source_id,
                    quality: ObservationQuality::Direct,
                    kind: ObservationSourceKind::Direct,
                    valid_for_ms: METRICS_STORAGE_TTL,
                },
                observed_at,
                sequence,
            )
        })
        .collect()
}

fn used_of_total(used: Option<f64>, total: Option<f64>) -> Option<(f64, f64)> {
    match (finite(used), finite(total)) {
        (Some(used), Some(total)) if total > 0.0 && (0.0..=total).contains(&used) => {
            Some((used, total))
        }
        _ => None,
    }
}

fn memory_source(origin: MetricsOrigin) -> &'static str {
    match origin {
        MetricsOrigin::Mock => "mock.system.metrics",
        MetricsOrigin::Procfs => "procfs.meminfo",
    }
}

fn storage_source(origin: MetricsOrigin) -> &'static str {
    match origin {
        MetricsOrigin::Mock => "mock.system.disk",
        MetricsOrigin::Procfs => "df.mount",
    }
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
        assert_eq!(rows.len(), 5);
        let used = by_key(&rows, KEY_MEMORY_USED_MB);
        assert_eq!(used.value, json!(5200.0));
        assert_eq!(used.unit.as_deref(), Some("megabyte"));
        assert_eq!(by_key(&rows, KEY_MEMORY_TOTAL_MB).value, json!(8192.0));
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

    fn disk_json() -> Value {
        json!({
            "source": "linux",
            "mounts": [
                { "path": "/", "total_mb": 1000.0, "used_mb": 900.0, "avail_mb": 100.0, "used_pct": 90.0 },
                { "path": "/data", "total_mb": 128000.0, "used_mb": 40000.0, "avail_mb": 88000.0, "used_pct": 31.0 }
            ],
            "root_used_pct": 90.0
        })
    }

    #[test]
    fn storage_prefers_data_mount_over_root() {
        let rows =
            observations_from_system_disk(&disk_json(), MetricsOrigin::Procfs, Utc::now(), 3);
        assert_eq!(rows.len(), 2);
        assert_eq!(by_key(&rows, KEY_STORAGE_USED_MB).value, json!(40000.0));
        let total = by_key(&rows, KEY_STORAGE_TOTAL_MB);
        assert_eq!(total.value, json!(128000.0));
        assert_eq!(total.unit.as_deref(), Some("megabyte"));
        assert_eq!(total.source.source_id, "df.mount");
        assert_eq!(total.valid_for_ms, METRICS_STORAGE_TTL);
    }

    #[test]
    fn storage_outlives_the_cpu_ttl_but_still_goes_stale() {
        let t0 = Utc::now();
        let rows = observations_from_system_disk(&disk_json(), MetricsOrigin::Procfs, t0, 1);
        let used = by_key(&rows, KEY_STORAGE_USED_MB);
        let after_cpu_ttl = t0 + ChronoDuration::milliseconds(METRICS_CPU_TTL as i64 + 1);
        assert_eq!(freshness_at(used, after_cpu_ttl), Freshness::Fresh);
        let after_storage_ttl = t0 + ChronoDuration::milliseconds(METRICS_STORAGE_TTL as i64 + 1);
        assert_eq!(freshness_at(used, after_storage_ttl), Freshness::Stale);
    }

    #[test]
    fn storage_falls_back_to_root_and_drops_unknown_mounts() {
        let only_root = json!({
            "mounts": [
                { "path": "/tmp", "total_mb": 10.0, "used_mb": 1.0 },
                { "path": "/", "total_mb": 32000.0, "used_mb": 22400.0 }
            ]
        });
        let rows = observations_from_system_disk(&only_root, MetricsOrigin::Mock, Utc::now(), 1);
        assert_eq!(by_key(&rows, KEY_STORAGE_USED_MB).value, json!(22400.0));
        assert_eq!(
            by_key(&rows, KEY_STORAGE_TOTAL_MB).source.source_id,
            "mock.system.disk"
        );
        let only_tmp = json!({ "mounts": [{ "path": "/tmp", "total_mb": 10.0, "used_mb": 1.0 }] });
        assert!(
            observations_from_system_disk(&only_tmp, MetricsOrigin::Mock, Utc::now(), 1).is_empty()
        );
    }

    #[test]
    fn impossible_capacity_numbers_are_dropped_not_clamped() {
        let now = Utc::now();
        for (used, total) in [(9.0, 8.0), (-1.0, 8.0), (1.0, 0.0)] {
            let disk = json!({ "mounts": [{ "path": "/", "used_mb": used, "total_mb": total }] });
            assert!(observations_from_system_disk(&disk, MetricsOrigin::Mock, now, 1).is_empty());
            let metrics = json!({ "mem_used_mb": used, "mem_total_mb": total });
            assert!(
                observations_from_system_metrics(&metrics, MetricsOrigin::Mock, now, 1)
                    .iter()
                    .all(|o| o.key != KEY_MEMORY_USED_MB && o.key != KEY_MEMORY_TOTAL_MB)
            );
        }
        assert!(observations_from_system_disk(
            &json!({ "error": "df unavailable", "mounts": [] }),
            MetricsOrigin::Procfs,
            now,
            1
        )
        .is_empty());
        assert!(
            observations_from_system_disk(&json!("junk"), MetricsOrigin::Procfs, now, 1).is_empty()
        );
    }
}
