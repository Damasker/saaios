use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
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
    /// Derived: used/total * 100
    #[serde(default)]
    pub mem_used_pct: f64,
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

    registry.register(Arc::new(DiskTool {
        backend: backend.clone(),
        spec: ToolSpec {
            name: "system.disk".into(),
            description: "Disk usage for root and key mounts".into(),
            risk: RiskLevel::Low,
            timeout_ms: 2000,
            input_schema: json!({"type":"object","properties":{}}),
            output_schema: json!({"type":"object"}),
            requires_confirmation: false,
        },
    }));

    registry.register(Arc::new(TemperatureTool {
        backend: backend.clone(),
        spec: ToolSpec {
            name: "system.temperature".into(),
            description: "Read thermal sensors (CPU/SoC zones when available)".into(),
            risk: RiskLevel::Low,
            timeout_ms: 2000,
            input_schema: json!({"type":"object","properties":{}}),
            output_schema: json!({"type":"object"}),
            requires_confirmation: false,
        },
    }));

    registry.register(Arc::new(JournalTool {
        backend: backend.clone(),
        spec: ToolSpec {
            name: "system.journal".into(),
            description: "Tail recent system journal / log lines".into(),
            risk: RiskLevel::Low,
            timeout_ms: 4000,
            input_schema: json!({
                "type":"object",
                "properties":{
                    "lines":{"type":"integer","minimum":1,"maximum":200},
                    "unit":{"type":"string"}
                }
            }),
            output_schema: json!({"type":"object"}),
            requires_confirmation: false,
        },
    }));

    registry.register(Arc::new(KillRequestTool {
        backend,
        spec: ToolSpec {
            name: "process.kill_request".into(),
            description: "Send a signal to a process (default SIGTERM). Requires confirmation."
                .into(),
            risk: RiskLevel::High,
            timeout_ms: 2000,
            input_schema: json!({
                "type":"object",
                "properties":{
                    "pid":{"type":"integer"},
                    "signal":{"type":"string","description":"TERM|KILL|HUP|INT (default TERM)"}
                },
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
    async fn disk(&self) -> Result<Value, ToolError>;
    async fn temperature(&self) -> Result<Value, ToolError>;
    async fn journal(&self, lines: usize, unit: Option<String>) -> Result<Value, ToolError>;
    async fn kill(&self, pid: u32, signal: &str) -> Result<Value, ToolError>;
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
            mem_used_pct: 63.5,
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
            "source": "mock",
            "interfaces": [{
                "name": "eth0",
                "operstate": "up",
                "ipv4": ["192.168.1.50/24"],
                "rx_bytes": 1_250_000,
                "tx_bytes": 420_000
            }],
            "default_route": "192.168.1.1"
        }))
    }

    async fn disk(&self) -> Result<Value, ToolError> {
        Ok(json!({
            "source": "mock",
            "mounts": [{
                "path": "/",
                "total_mb": 32000.0,
                "used_mb": 22400.0,
                "avail_mb": 9600.0,
                "used_pct": 70.0
            }],
            "root_used_pct": 70.0
        }))
    }

    async fn temperature(&self) -> Result<Value, ToolError> {
        Ok(json!({
            "source": "mock",
            "celsius": 62.5,
            "zones": [{"name":"cpu-thermal","celsius":62.5}]
        }))
    }

    async fn journal(&self, lines: usize, unit: Option<String>) -> Result<Value, ToolError> {
        let mut entries = vec![
            "2026-08-03T10:00:01 saaios-runtime[1]: listening on /tmp/saaios.sock".into(),
            "2026-08-03T10:00:05 kernel: CPU soft lockup suspected on runaway-worker".into(),
            "2026-08-03T10:00:08 systemd[1]: Started Session 42 of user pi".into(),
        ];
        if let Some(u) = unit {
            entries.retain(|e: &String| e.contains(&u));
            if entries.is_empty() {
                entries.push(format!("(no mock lines for unit={u})"));
            }
        }
        entries.truncate(lines.max(1));
        Ok(json!({
            "source": "mock",
            "lines": entries,
            "count": entries.len()
        }))
    }

    async fn kill(&self, pid: u32, signal: &str) -> Result<Value, ToolError> {
        if pid <= 1 {
            return Err(ToolError::InvalidArgs("refusing to signal pid <= 1".into()));
        }
        Ok(json!({
            "killed": true,
            "pid": pid,
            "signal": signal,
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
        let cpu_usage = sample_cpu_usage().await.unwrap_or(0.0);
        let mem_used_pct = if mem.1 > 0.0 {
            (mem.0 / mem.1) * 100.0
        } else {
            0.0
        };
        Ok(SystemMetrics {
            cpu_usage,
            load_average,
            mem_used_mb: mem.0,
            mem_total_mb: mem.1,
            mem_used_pct: (mem_used_pct * 10.0).round() / 10.0,
            disk_read_mb_s: 0.0,
        })
    }

    async fn processes(&self) -> Result<Vec<ProcessInfo>, ToolError> {
        let total1 = read_total_jiffies().unwrap_or(0);
        let sample1 = snapshot_process_cpu()?;
        sleep(Duration::from_millis(120)).await;
        let total2 = read_total_jiffies().unwrap_or(total1);
        let sample2 = snapshot_process_cpu()?;
        let total_delta = total2.saturating_sub(total1).max(1) as f64;
        let ncpus = std::thread::available_parallelism()
            .map(|n| n.get() as f64)
            .unwrap_or(1.0);

        let mut procs = Vec::new();
        for (pid, (name, j1, mem_mb)) in sample1 {
            if let Some((_, j2, mem2)) = sample2.get(&pid) {
                let delta = j2.saturating_sub(j1) as f64;
                // Share of total CPU time across all cores, scaled to percent of one core * ncpus.
                let cpu = ((delta / total_delta) * ncpus * 100.0).clamp(0.0, 100.0 * ncpus);
                procs.push(ProcessInfo {
                    pid,
                    name,
                    cpu: (cpu * 10.0).round() / 10.0,
                    mem_mb: (*mem2).max(mem_mb),
                });
            }
        }
        procs.sort_by(|a, b| {
            b.cpu
                .partial_cmp(&a.cpu)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        procs.truncate(32);
        Ok(procs)
    }

    async fn network_status(&self) -> Result<Value, ToolError> {
        Ok(read_network_status())
    }

    async fn disk(&self) -> Result<Value, ToolError> {
        Ok(read_disk_usage())
    }

    async fn temperature(&self) -> Result<Value, ToolError> {
        Ok(read_temperature())
    }

    async fn journal(&self, lines: usize, unit: Option<String>) -> Result<Value, ToolError> {
        Ok(read_journal(lines, unit.as_deref()))
    }

    async fn kill(&self, pid: u32, signal: &str) -> Result<Value, ToolError> {
        real_kill(pid, signal)
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

fn read_disk_usage() -> Value {
    // Prefer libc-free approach: parse `df -Bk` for / and a few common mounts.
    let output = std::process::Command::new("df")
        .args(["-Bk", "/", "/home", "/var", "/tmp"])
        .output();
    let Ok(out) = output else {
        return json!({
            "source": "linux",
            "mounts": [],
            "root_used_pct": null,
            "error": "df unavailable"
        });
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let mut mounts = Vec::new();
    let mut root_used_pct = None;
    let mut seen = std::collections::HashSet::new();
    for line in text.lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 6 {
            continue;
        }
        let path = cols[5].to_string();
        if !seen.insert(path.clone()) {
            continue;
        }
        let total_kb = cols[1].trim_end_matches('K').parse::<f64>().unwrap_or(0.0);
        let used_kb = cols[2].trim_end_matches('K').parse::<f64>().unwrap_or(0.0);
        let avail_kb = cols[3].trim_end_matches('K').parse::<f64>().unwrap_or(0.0);
        let used_pct = cols[4]
            .trim_end_matches('%')
            .parse::<f64>()
            .unwrap_or_else(|_| {
                if total_kb > 0.0 {
                    (used_kb / total_kb) * 100.0
                } else {
                    0.0
                }
            });
        if path == "/" {
            root_used_pct = Some(used_pct);
        }
        mounts.push(json!({
            "path": path,
            "total_mb": (total_kb / 1024.0 * 10.0).round() / 10.0,
            "used_mb": (used_kb / 1024.0 * 10.0).round() / 10.0,
            "avail_mb": (avail_kb / 1024.0 * 10.0).round() / 10.0,
            "used_pct": used_pct
        }));
    }
    json!({
        "source": "linux",
        "mounts": mounts,
        "root_used_pct": root_used_pct
    })
}

fn read_network_status() -> Value {
    let mut interfaces = Vec::new();
    let net_dir = Path::new("/sys/class/net");
    if net_dir.exists() {
        if let Ok(read) = std::fs::read_dir(net_dir) {
            for entry in read.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let path = entry.path();
                let oper = std::fs::read_to_string(path.join("operstate"))
                    .unwrap_or_else(|_| "unknown".into())
                    .trim()
                    .to_string();
                let rx_bytes = std::fs::read_to_string(path.join("statistics/rx_bytes"))
                    .ok()
                    .and_then(|s| s.trim().parse::<u64>().ok());
                let tx_bytes = std::fs::read_to_string(path.join("statistics/tx_bytes"))
                    .ok()
                    .and_then(|s| s.trim().parse::<u64>().ok());
                let ipv4 = read_iface_ipv4(&name);
                interfaces.push(json!({
                    "name": name,
                    "operstate": oper,
                    "ipv4": ipv4,
                    "rx_bytes": rx_bytes,
                    "tx_bytes": tx_bytes
                }));
            }
        }
    }
    let online = interfaces.iter().any(|i| i["operstate"] == "up");
    let default_route = read_default_gateway();
    json!({
        "online": online,
        "source": "linux",
        "interfaces": interfaces,
        "default_route": default_route
    })
}

fn read_iface_ipv4(name: &str) -> Vec<String> {
    let output = std::process::Command::new("ip")
        .args(["-4", "-o", "addr", "show", "dev", name])
        .output();
    let Ok(out) = output else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let mut addrs = Vec::new();
    for line in text.lines() {
        // ... inet 192.168.1.10/24 ...
        let mut parts = line.split_whitespace();
        while let Some(tok) = parts.next() {
            if tok == "inet" {
                if let Some(addr) = parts.next() {
                    addrs.push(addr.to_string());
                }
                break;
            }
        }
    }
    addrs
}

fn read_default_gateway() -> Option<String> {
    let content = std::fs::read_to_string("/proc/net/route").ok()?;
    for line in content.lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 3 {
            continue;
        }
        // Destination 00000000 means default route.
        if cols[1] == "00000000" {
            let gw = cols[2];
            if let Ok(n) = u32::from_str_radix(gw, 16) {
                let b = n.to_le_bytes();
                return Some(format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3]));
            }
        }
    }
    None
}

fn read_temperature() -> Value {
    let mut zones = Vec::new();
    let thermal = Path::new("/sys/class/thermal");
    if thermal.exists() {
        if let Ok(read) = std::fs::read_dir(thermal) {
            for entry in read.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.starts_with("thermal_zone") {
                    continue;
                }
                let temp_path = entry.path().join("temp");
                let type_path = entry.path().join("type");
                let Some(raw) = std::fs::read_to_string(temp_path)
                    .ok()
                    .and_then(|s| s.trim().parse::<f64>().ok())
                else {
                    continue;
                };
                // Usually millidegrees C.
                let celsius = if raw > 1000.0 { raw / 1000.0 } else { raw };
                let zname = std::fs::read_to_string(type_path)
                    .unwrap_or(name)
                    .trim()
                    .to_string();
                zones.push(json!({
                    "name": zname,
                    "celsius": (celsius * 10.0).round() / 10.0
                }));
            }
        }
    }
    // Fallback: hwmon
    if zones.is_empty() {
        let hwmon = Path::new("/sys/class/hwmon");
        if hwmon.exists() {
            if let Ok(read) = std::fs::read_dir(hwmon) {
                for entry in read.flatten() {
                    let path = entry.path();
                    let label = std::fs::read_to_string(path.join("name"))
                        .unwrap_or_else(|_| "hwmon".into())
                        .trim()
                        .to_string();
                    if let Ok(raw) = std::fs::read_to_string(path.join("temp1_input")) {
                        if let Ok(v) = raw.trim().parse::<f64>() {
                            let celsius = if v > 1000.0 { v / 1000.0 } else { v };
                            zones.push(json!({
                                "name": label,
                                "celsius": (celsius * 10.0).round() / 10.0
                            }));
                        }
                    }
                }
            }
        }
    }
    let max_c = zones
        .iter()
        .filter_map(|z| z.get("celsius").and_then(|v| v.as_f64()))
        .fold(None, |acc: Option<f64>, v| {
            Some(acc.map(|a| a.max(v)).unwrap_or(v))
        });
    json!({
        "source": "linux",
        "celsius": max_c,
        "zones": zones
    })
}

fn read_journal(lines: usize, unit: Option<&str>) -> Value {
    let lines = lines.clamp(1, 200);
    let mut cmd = std::process::Command::new("journalctl");
    cmd.args(["-n", &lines.to_string(), "--no-pager", "-o", "short-iso"]);
    if let Some(u) = unit {
        if !u.is_empty() {
            cmd.args(["-u", u]);
        }
    }
    match cmd.output() {
        Ok(out) if out.status.success() || !out.stdout.is_empty() => {
            let text = String::from_utf8_lossy(&out.stdout);
            let entries: Vec<String> = text
                .lines()
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
                .collect();
            json!({
                "source": "linux",
                "lines": entries,
                "count": entries.len(),
                "unit": unit
            })
        }
        Ok(out) => {
            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
            // Fallback: dmesg tail
            if let Ok(dmesg) = std::process::Command::new("dmesg").args(["-T"]).output() {
                let text = String::from_utf8_lossy(&dmesg.stdout);
                let mut entries: Vec<String> = text.lines().map(|s| s.to_string()).collect();
                if entries.len() > lines {
                    entries = entries.split_off(entries.len() - lines);
                }
                return json!({
                    "source": "dmesg",
                    "lines": entries,
                    "count": entries.len(),
                    "journalctl_error": err
                });
            }
            json!({
                "source": "linux",
                "lines": [],
                "count": 0,
                "error": err
            })
        }
        Err(e) => json!({
            "source": "linux",
            "lines": [],
            "count": 0,
            "error": e.to_string()
        }),
    }
}

fn normalize_signal(signal: &str) -> Result<String, ToolError> {
    let s = signal.trim().trim_start_matches("SIG").to_uppercase();
    match s.as_str() {
        "TERM" | "KILL" | "HUP" | "INT" | "QUIT" | "USR1" | "USR2" => Ok(s),
        "" => Ok("TERM".into()),
        other => Err(ToolError::InvalidArgs(format!(
            "unsupported signal {other}; allowed TERM|KILL|HUP|INT|QUIT|USR1|USR2"
        ))),
    }
}

fn real_kill(pid: u32, signal: &str) -> Result<Value, ToolError> {
    if pid <= 1 {
        return Err(ToolError::InvalidArgs("refusing to signal pid <= 1".into()));
    }
    if pid == std::process::id() {
        return Err(ToolError::InvalidArgs(
            "refusing to signal the saaios-runtime process".into(),
        ));
    }
    let signal = normalize_signal(signal)?;
    // Verify pid exists.
    if !Path::new(&format!("/proc/{pid}")).exists() {
        return Err(ToolError::Execution(format!("pid {pid} not found")));
    }
    let status = std::process::Command::new("kill")
        .arg(format!("-{signal}"))
        .arg(pid.to_string())
        .status()
        .map_err(|e| ToolError::Execution(e.to_string()))?;
    if status.success() {
        Ok(json!({
            "killed": true,
            "dry_run": false,
            "pid": pid,
            "signal": signal,
            "mode": "linux"
        }))
    } else {
        Err(ToolError::Execution(format!(
            "kill -{signal} {pid} failed with {status}"
        )))
    }
}

fn parse_kb(v: &str) -> Option<f64> {
    v.split_whitespace().next()?.parse().ok()
}

fn read_total_jiffies() -> Option<u64> {
    let content = std::fs::read_to_string("/proc/stat").ok()?;
    let line = content.lines().next()?;
    // cpu user nice system idle iowait irq softirq steal ...
    let mut sum = 0u64;
    for (i, part) in line.split_whitespace().enumerate() {
        if i == 0 {
            continue;
        }
        sum = sum.saturating_add(part.parse::<u64>().ok()?);
    }
    Some(sum)
}

async fn sample_cpu_usage() -> Option<f64> {
    let a = read_cpu_times()?;
    sleep(Duration::from_millis(100)).await;
    let b = read_cpu_times()?;
    let idle_delta = b.1.saturating_sub(a.1) as f64;
    let total_delta = b.0.saturating_sub(a.0).max(1) as f64;
    Some(((1.0 - idle_delta / total_delta) * 100.0).clamp(0.0, 100.0))
}

fn read_cpu_times() -> Option<(u64, u64)> {
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

fn snapshot_process_cpu() -> Result<HashMap<u32, (String, u64, f64)>, ToolError> {
    let mut map = HashMap::new();
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
        let stat = match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let Some(jiffies) = parse_proc_stat_jiffies(&stat) else {
            continue;
        };
        let comm = std::fs::read_to_string(format!("/proc/{pid}/comm"))
            .unwrap_or_else(|_| "unknown".into())
            .trim()
            .to_string();
        let mem_mb = read_vm_rss_mb(pid).unwrap_or(0.0);
        map.insert(pid, (comm, jiffies, mem_mb));
        if map.len() >= 256 {
            break;
        }
    }
    Ok(map)
}

fn parse_proc_stat_jiffies(stat: &str) -> Option<u64> {
    // Format: pid (comm) state ... utime(14) stime(15) — 1-indexed fields after comm.
    let close = stat.rfind(')')?;
    let after = stat.get(close + 2..)?;
    let parts: Vec<&str> = after.split_whitespace().collect();
    // After ") ": state is parts[0], utime is parts[11], stime parts[12]
    let utime: u64 = parts.get(11)?.parse().ok()?;
    let stime: u64 = parts.get(12)?.parse().ok()?;
    Some(utime.saturating_add(stime))
}

fn read_vm_rss_mb(pid: u32) -> Option<f64> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let kb: f64 = rest.split_whitespace().next()?.parse().ok()?;
            return Some(kb / 1024.0);
        }
    }
    None
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

struct DiskTool {
    backend: Arc<dyn SystemBackend>,
    spec: ToolSpec,
}

#[async_trait]
impl ToolExecutor for DiskTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, _args: Value, _ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
        let disk = self.backend.disk().await?;
        Ok(ToolOutput {
            ok: true,
            value: disk,
            error: None,
        })
    }
}

struct TemperatureTool {
    backend: Arc<dyn SystemBackend>,
    spec: ToolSpec,
}

#[async_trait]
impl ToolExecutor for TemperatureTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, _args: Value, _ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
        let temp = self.backend.temperature().await?;
        Ok(ToolOutput {
            ok: true,
            value: temp,
            error: None,
        })
    }
}

struct JournalTool {
    backend: Arc<dyn SystemBackend>,
    spec: ToolSpec,
}

#[async_trait]
impl ToolExecutor for JournalTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
        let lines = args.get("lines").and_then(|v| v.as_u64()).unwrap_or(40) as usize;
        let unit = args
            .get("unit")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let journal = self.backend.journal(lines, unit).await?;
        Ok(ToolOutput {
            ok: true,
            value: journal,
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
        let signal = args
            .get("signal")
            .and_then(|v| v.as_str())
            .unwrap_or("TERM");
        let value = self.backend.kill(pid, signal).await?;
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

    #[tokio::test]
    async fn mock_disk_fixture() {
        let mut reg = ToolRegistry::new();
        install_system_tools(&mut reg, ToolsMode::Mock);
        let out = reg
            .execute(
                "system.disk",
                json!({}),
                &ToolContext {
                    correlation_id: Uuid::new_v4(),
                    call_id: Uuid::new_v4(),
                },
            )
            .await
            .unwrap();
        assert!(out.ok);
        assert_eq!(out.value["root_used_pct"], 70.0);
    }

    #[tokio::test]
    async fn mock_temperature_and_journal() {
        let mut reg = ToolRegistry::new();
        install_system_tools(&mut reg, ToolsMode::Mock);
        let ctx = ToolContext {
            correlation_id: Uuid::new_v4(),
            call_id: Uuid::new_v4(),
        };
        let temp = reg
            .execute("system.temperature", json!({}), &ctx)
            .await
            .unwrap();
        assert_eq!(temp.value["celsius"], 62.5);
        let journal = reg
            .execute("system.journal", json!({"lines": 2}), &ctx)
            .await
            .unwrap();
        assert_eq!(journal.value["count"], 2);
        let net = reg
            .execute("network.status", json!({}), &ctx)
            .await
            .unwrap();
        assert_eq!(net.value["source"], "mock");
        assert!(!net.value["interfaces"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn mock_kill_refuses_pid_one() {
        let mut reg = ToolRegistry::new();
        install_system_tools(&mut reg, ToolsMode::Mock);
        let err = reg
            .execute(
                "process.kill_request",
                json!({"pid": 1}),
                &ToolContext {
                    correlation_id: Uuid::new_v4(),
                    call_id: Uuid::new_v4(),
                },
            )
            .await
            .unwrap_err();
        assert!(err.to_string().contains("pid <= 1"));
    }

    #[tokio::test]
    async fn real_linux_metrics_smoke() {
        if !Path::new("/proc/stat").exists() {
            return;
        }
        let mut reg = ToolRegistry::new();
        install_system_tools(&mut reg, ToolsMode::RealLinux);
        let metrics = reg
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
        assert!(metrics.ok);
        assert!(metrics.value.get("cpu_usage").is_some());

        let procs = reg
            .execute(
                "process.list",
                json!({}),
                &ToolContext {
                    correlation_id: Uuid::new_v4(),
                    call_id: Uuid::new_v4(),
                },
            )
            .await
            .unwrap();
        assert!(procs.ok);
        assert!(!procs.value["processes"].as_array().unwrap().is_empty());

        let net = reg
            .execute(
                "network.status",
                json!({}),
                &ToolContext {
                    correlation_id: Uuid::new_v4(),
                    call_id: Uuid::new_v4(),
                },
            )
            .await
            .unwrap();
        assert!(net.ok);
        assert_eq!(net.value["source"], "linux");
    }

    #[test]
    fn parse_stat_jiffies_basic() {
        let sample = "1 (systemd) S 0 1 1 0 -1 4194560 123 0 0 0 10 20 0 0 20 0 1 0 12345 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0";
        assert_eq!(parse_proc_stat_jiffies(sample), Some(30));
    }
}
