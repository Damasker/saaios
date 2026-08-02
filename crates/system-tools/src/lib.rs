use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tool_registry::{
    RiskLevel, ToolContext, ToolError, ToolExecutor, ToolOutput, ToolRegistry, ToolSpec,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu: f64,
    pub mem_mb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_usage: f64,
    pub load_average: f64,
    pub mem_used_mb: f64,
    pub mem_total_mb: f64,
    pub disk_read_mb_s: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolsMode {
    Mock,
    RealLinux,
}

pub fn install_system_tools(registry: &mut ToolRegistry, mode: ToolsMode) {
    let backend: Arc<dyn SystemBackend> = match mode {
        ToolsMode::Mock => Arc::new(MockBackend),
        ToolsMode::RealLinux => Arc::new(LinuxBackend),
    };

    registry.register(Arc::new(MetricsTool {
        backend: backend.clone(),
        spec: ToolSpec {
            name: "system.metrics".into(),
            description: "Collect CPU, memory and load metrics".into(),
            risk: RiskLevel::Low,
            timeout_ms: 2000,
            input_schema: json!({"type":"object","properties":{}}),
            output_schema: json!({"type":"object"}),
            requires_confirmation: false,
        },
    }));

    registry.register(Arc::new(ProcessListTool {
        backend: backend.clone(),
        spec: ToolSpec {
            name: "process.list".into(),
            description: "List running processes with CPU usage".into(),
            risk: RiskLevel::Low,
            timeout_ms: 3000,
            input_schema: json!({"type":"object","properties":{}}),
            output_schema: json!({"type":"object"}),
            requires_confirmation: false,
        },
    }));

    registry.register(Arc::new(NetworkStatusTool {
        backend: backend.clone(),
        spec: ToolSpec {
            name: "network.status".into(),
            description: "Basic network status".into(),
            risk: RiskLevel::Low,
            timeout_ms: 2000,
            input_schema: json!({"type":"object","properties":{}}),
            output_schema: json!({"type":"object"}),
            requires_confirmation: false,
        },
    }));

    registry.register(Arc::new(KillRequestTool {
        backend,
        spec: ToolSpec {
            name: "process.kill_request".into(),
            description: "Request termination of a process".into(),
            risk: RiskLevel::High,
            timeout_ms: 2000,
            input_schema: json!({
                "type":"object",
                "properties":{"pid":{"type":"integer"}},
                "required":["pid"]
            }),
            output_schema: json!({"type":"object"}),
            requires_confirmation: true,
        },
    }));
}

#[async_trait]
trait SystemBackend: Send + Sync {
    async fn metrics(&self) -> Result<SystemMetrics, ToolError>;
    async fn processes(&self) -> Result<Vec<ProcessInfo>, ToolError>;
    async fn network_status(&self) -> Result<Value, ToolError>;
    async fn kill(&self, pid: u32) -> Result<Value, ToolError>;
}

struct MockBackend;

#[async_trait]
impl SystemBackend for MockBackend {
    async fn metrics(&self) -> Result<SystemMetrics, ToolError> {
        Ok(SystemMetrics {
            cpu_usage: 97.0,
            load_average: 8.4,
            mem_used_mb: 5200.0,
            mem_total_mb: 8192.0,
            disk_read_mb_s: 120.0,
        })
    }

    async fn processes(&self) -> Result<Vec<ProcessInfo>, ToolError> {
        Ok(vec![
            ProcessInfo {
                pid: 4312,
                name: "runaway-worker".into(),
                cpu: 88.0,
                mem_mb: 900.0,
            },
            ProcessInfo {
                pid: 120,
                name: "systemd".into(),
                cpu: 0.3,
                mem_mb: 40.0,
            },
            ProcessInfo {
                pid: 880,
                name: "sshd".into(),
                cpu: 0.1,
                mem_mb: 20.0,
            },
        ])
    }

    async fn network_status(&self) -> Result<Value, ToolError> {
        Ok(json!({
            "online": true,
            "interface": "eth0",
            "ipv4": "192.168.1.50"
        }))
    }

    async fn kill(&self, pid: u32) -> Result<Value, ToolError> {
        Ok(json!({
            "killed": true,
            "pid": pid,
            "mode": "mock"
        }))
    }
}

struct LinuxBackend;

#[async_trait]
impl SystemBackend for LinuxBackend {
    async fn metrics(&self) -> Result<SystemMetrics, ToolError> {
        let load = std::fs::read_to_string("/proc/loadavg")
            .map_err(|e| ToolError::Execution(e.to_string()))?;
        let load_average = load
            .split_whitespace()
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        let mem = read_meminfo().unwrap_or((0.0, 0.0));
        let cpu_usage = estimate_cpu_usage().unwrap_or(0.0);
        Ok(SystemMetrics {
            cpu_usage,
            load_average,
            mem_used_mb: mem.0,
            mem_total_mb: mem.1,
            disk_read_mb_s: 0.0,
        })
    }

    async fn processes(&self) -> Result<Vec<ProcessInfo>, ToolError> {
        let mut procs = Vec::new();
        let read = std::fs::read_dir("/proc").map_err(|e| ToolError::Execution(e.to_string()))?;
        for entry in read.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !name.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            let pid: u32 = match name.parse() {
                Ok(p) => p,
                Err(_) => continue,
            };
            let comm_path = format!("/proc/{pid}/comm");
            let comm = std::fs::read_to_string(comm_path)
                .unwrap_or_else(|_| "unknown".into())
                .trim()
                .to_string();
            procs.push(ProcessInfo {
                pid,
                name: comm,
                cpu: 0.0,
                mem_mb: 0.0,
            });
            if procs.len() >= 64 {
                break;
            }
        }
        Ok(procs)
    }

    async fn network_status(&self) -> Result<Value, ToolError> {
        Ok(json!({
            "online": std::path::Path::new("/sys/class/net").exists(),
            "source": "linux"
        }))
    }

    async fn kill(&self, pid: u32) -> Result<Value, ToolError> {
        // Platform 0.1: do not actually signal processes in real mode from AI path.
        // Return a dry-run style acknowledgement; dangerous side effects stay gated.
        Ok(json!({
            "killed": false,
            "dry_run": true,
            "pid": pid,
            "mode": "linux"
        }))
    }
}

fn read_meminfo() -> Option<(f64, f64)> {
    let content = std::fs::read_to_string("/proc/meminfo").ok()?;
    let mut total_kb = 0.0;
    let mut available_kb = 0.0;
    for line in content.lines() {
        if let Some(v) = line.strip_prefix("MemTotal:") {
            total_kb = parse_kb(v)?;
        } else if let Some(v) = line.strip_prefix("MemAvailable:") {
            available_kb = parse_kb(v)?;
        }
    }
    let used = (total_kb - available_kb) / 1024.0;
    Some((used, total_kb / 1024.0))
}

fn parse_kb(v: &str) -> Option<f64> {
    v.split_whitespace().next()?.parse().ok()
}

fn estimate_cpu_usage() -> Option<f64> {
    // Best-effort single snapshot; not a precise delta sampler.
    let load = std::fs::read_to_string("/proc/loadavg").ok()?;
    let one: f64 = load.split_whitespace().next()?.parse().ok()?;
    let cpus = std::thread::available_parallelism()
        .map(|n| n.get() as f64)
        .unwrap_or(1.0);
    Some(((one / cpus) * 100.0).clamp(0.0, 100.0))
}

struct MetricsTool {
    backend: Arc<dyn SystemBackend>,
    spec: ToolSpec,
}

#[async_trait]
impl ToolExecutor for MetricsTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, _args: Value, _ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
        let metrics = self.backend.metrics().await?;
        Ok(ToolOutput {
            ok: true,
            value: serde_json::to_value(metrics).unwrap_or(json!({})),
            error: None,
        })
    }
}

struct ProcessListTool {
    backend: Arc<dyn SystemBackend>,
    spec: ToolSpec,
}

#[async_trait]
impl ToolExecutor for ProcessListTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, _args: Value, _ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
        let processes = self.backend.processes().await?;
        Ok(ToolOutput {
            ok: true,
            value: json!({ "processes": processes }),
            error: None,
        })
    }
}

struct NetworkStatusTool {
    backend: Arc<dyn SystemBackend>,
    spec: ToolSpec,
}

#[async_trait]
impl ToolExecutor for NetworkStatusTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, _args: Value, _ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
        let status = self.backend.network_status().await?;
        Ok(ToolOutput {
            ok: true,
            value: status,
            error: None,
        })
    }
}

struct KillRequestTool {
    backend: Arc<dyn SystemBackend>,
    spec: ToolSpec,
}

#[async_trait]
impl ToolExecutor for KillRequestTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
        let pid = args
            .get("pid")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| ToolError::InvalidArgs("pid required".into()))? as u32;
        let value = self.backend.kill(pid).await?;
        Ok(ToolOutput {
            ok: true,
            value,
            error: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn mock_metrics_fixture() {
        let mut reg = ToolRegistry::new();
        install_system_tools(&mut reg, ToolsMode::Mock);
        let out = reg
            .execute(
                "system.metrics",
                json!({}),
                &ToolContext {
                    correlation_id: Uuid::new_v4(),
                    call_id: Uuid::new_v4(),
                },
            )
            .await
            .unwrap();
        assert_eq!(out.value["cpu_usage"], 97.0);
        assert_eq!(out.value["load_average"], 8.4);
    }
}
