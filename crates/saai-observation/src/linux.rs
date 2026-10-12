//! WORLD-04: one place reads Linux /proc for host metrics.
//! `system-tools` and Observation conversion share these helpers.
//! No daemon. No Attention. Not saai-deviced.

use crate::metrics::{observations_from_system_metrics, MetricsOrigin};
use crate::model::Observation;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::time::Duration;

/// Synced host sample. Same numbers feed ToolResult JSON and Observations.
#[derive(Debug, Clone, PartialEq)]
pub struct HostMetricsSample {
    pub cpu_usage: f64,
    pub load_average: f64,
    pub mem_used_mb: f64,
    pub mem_total_mb: f64,
    pub mem_used_pct: f64,
}

pub fn read_load_average() -> Option<f64> {
    let content = std::fs::read_to_string("/proc/loadavg").ok()?;
    content
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
}

/// Returns `(used_mb, total_mb)` from MemTotal / MemAvailable.
pub fn read_meminfo_mb() -> Option<(f64, f64)> {
    let content = std::fs::read_to_string("/proc/meminfo").ok()?;
    let mut total_kb = None;
    let mut available_kb = None;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            total_kb = parse_kb(rest);
        } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
            available_kb = parse_kb(rest);
        }
    }
    let total_kb = total_kb?;
    let available_kb = available_kb.unwrap_or(0.0);
    let total_mb = total_kb / 1024.0;
    let used_mb = ((total_kb - available_kb) / 1024.0).max(0.0);
    Some((used_mb, total_mb))
}

fn parse_kb(v: &str) -> Option<f64> {
    v.split_whitespace().next()?.parse().ok()
}

/// `(total_jiffies, idle_jiffies)` from the aggregate `cpu ` line.
pub fn read_cpu_times() -> Option<(u64, u64)> {
    let content = std::fs::read_to_string("/proc/stat").ok()?;
    let line = content.lines().next()?;
    let parts: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .filter_map(|p| p.parse().ok())
        .collect();
    if parts.len() < 4 {
        return None;
    }
    let idle = parts[3] + parts.get(4).copied().unwrap_or(0);
    let total: u64 = parts.iter().sum();
    Some((total, idle))
}

pub fn cpu_usage_between(a: (u64, u64), b: (u64, u64)) -> f64 {
    let idle_delta = b.1.saturating_sub(a.1) as f64;
    let total_delta = b.0.saturating_sub(a.0).max(1) as f64;
    ((1.0 - idle_delta / total_delta) * 100.0).clamp(0.0, 100.0)
}

/// Blocking two-sample CPU read. Prefer the async path in system-tools
/// for production dispatch; this exists so Observation and tools share
/// the same /proc parsers without copying fields.
pub fn sample_host_metrics_blocking(sample_ms: u64) -> Option<HostMetricsSample> {
    let a = read_cpu_times()?;
    std::thread::sleep(Duration::from_millis(sample_ms.max(1)));
    let b = read_cpu_times()?;
    let mem = read_meminfo_mb().unwrap_or((0.0, 0.0));
    let mem_used_pct = if mem.1 > 0.0 {
        (mem.0 / mem.1) * 100.0
    } else {
        0.0
    };
    Some(HostMetricsSample {
        cpu_usage: cpu_usage_between(a, b),
        load_average: read_load_average().unwrap_or(0.0),
        mem_used_mb: mem.0,
        mem_total_mb: mem.1,
        mem_used_pct: (mem_used_pct * 10.0).round() / 10.0,
    })
}

impl HostMetricsSample {
    pub fn to_tool_json(&self) -> Value {
        json!({
            "cpu_usage": self.cpu_usage,
            "load_average": self.load_average,
            "mem_used_mb": self.mem_used_mb,
            "mem_total_mb": self.mem_total_mb,
            "mem_used_pct": self.mem_used_pct,
            "disk_read_mb_s": 0.0,
        })
    }

    pub fn to_observations(&self, observed_at: DateTime<Utc>, sequence: u64) -> Vec<Observation> {
        observations_from_system_metrics(
            &self.to_tool_json(),
            MetricsOrigin::Procfs,
            observed_at,
            sequence,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::{KEY_CPU_USAGE, KEY_LOAD_AVERAGE, KEY_MEMORY_USED_PERCENT};

    #[test]
    fn host_parsers_agree_with_proc_when_present() {
        if !std::path::Path::new("/proc/loadavg").exists() {
            return;
        }
        let load = read_load_average().expect("loadavg");
        assert!(load >= 0.0);
        let mem = read_meminfo_mb().expect("meminfo");
        assert!(mem.1 > 0.0);
        assert!(mem.0 <= mem.1 + 1.0);
        let a = read_cpu_times().expect("cpu a");
        let b = read_cpu_times().expect("cpu b");
        let usage = cpu_usage_between(a, b);
        assert!((0.0..=100.0).contains(&usage));
    }

    #[test]
    fn sample_converts_to_observations_without_second_proc_shape() {
        let sample = HostMetricsSample {
            cpu_usage: 12.5,
            load_average: 0.4,
            mem_used_mb: 1000.0,
            mem_total_mb: 4000.0,
            mem_used_pct: 25.0,
        };
        let obs = sample.to_observations(Utc::now(), 1);
        let keys: Vec<_> = obs.iter().map(|o| o.key.as_str()).collect();
        assert!(keys.contains(&KEY_CPU_USAGE));
        assert!(keys.contains(&KEY_LOAD_AVERAGE));
        assert!(keys.contains(&KEY_MEMORY_USED_PERCENT));
        assert_eq!(sample.to_tool_json()["cpu_usage"], 12.5);
    }
}
