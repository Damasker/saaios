use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Full SaaiOS runtime configuration (TOML).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct SaaiOsConfig {
    pub runtime: RuntimeConfig,
    pub provider: ProviderConfig,
    pub budgets: BudgetsConfig,
    pub memory: MemoryConfig,
    pub automation: AutomationConfig,
    pub telemetry: TelemetryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct RuntimeConfig {
    pub sock: PathBuf,
    pub audit: PathBuf,
    pub real_linux: bool,
    pub mock_planner: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            sock: PathBuf::from("/tmp/saaios.sock"),
            audit: PathBuf::from("saaios-audit.jsonl"),
            real_linux: false,
            mock_planner: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ProviderConfig {
    /// mock | remote | local | auto
    pub kind: String,
    pub api_base: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub local_base: Option<String>,
    pub local_model: Option<String>,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            kind: "auto".into(),
            api_base: None,
            api_key: None,
            model: Some("gpt-4o-mini".into()),
            local_base: Some("http://127.0.0.1:11434/v1".into()),
            local_model: Some("llama3.2".into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct BudgetsConfig {
    pub max_concurrent: usize,
    pub request_timeout_secs: u64,
    pub max_tool_iters: usize,
}

impl Default for BudgetsConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 1,
            request_timeout_secs: 30,
            max_tool_iters: 6,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct MemoryConfig {
    pub enabled: bool,
    pub path: PathBuf,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            path: PathBuf::from("saaios-memory.jsonl"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AutomationConfig {
    pub enabled: bool,
}

impl Default for AutomationConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryMode {
    /// On when `runtime.real_linux` is true.
    #[default]
    Auto,
    On,
    Off,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct TelemetryConfig {
    pub mode: TelemetryMode,
    pub interval_secs: u64,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            mode: TelemetryMode::Auto,
            interval_secs: 30,
        }
    }
}

/// Resolved settings after defaults → file → CLI/env merge.
#[derive(Debug, Clone)]
pub struct ResolvedSettings {
    pub config_path: Option<PathBuf>,
    pub sock: PathBuf,
    pub audit: PathBuf,
    pub real_linux: bool,
    pub mock_planner: bool,
    pub provider_kind: String,
    pub api_base: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub local_base: Option<String>,
    pub local_model: Option<String>,
    pub max_concurrent: usize,
    pub request_timeout_secs: u64,
    pub max_tool_iters: usize,
    pub memory_enabled: bool,
    pub memory_path: PathBuf,
    pub automation_enabled: bool,
    pub telemetry_enabled: bool,
    pub telemetry_interval_secs: u64,
}

/// CLI/env overlays. `None` means "leave config/default".
#[derive(Debug, Clone, Default)]
pub struct CliOverrides {
    pub config: Option<PathBuf>,
    pub mock: bool,
    pub mock_planner: bool,
    pub provider: Option<String>,
    pub sock: Option<PathBuf>,
    pub audit: Option<PathBuf>,
    pub memory: Option<PathBuf>,
    pub no_memory: bool,
    pub real_linux: bool,
    pub max_concurrent: Option<usize>,
    pub request_timeout_secs: Option<u64>,
    pub max_tool_iters: Option<usize>,
    pub no_automation: bool,
    pub telemetry: bool,
    pub no_telemetry: bool,
    pub telemetry_interval_secs: Option<u64>,
}

impl SaaiOsConfig {
    pub fn load_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("read config {}", path.display()))?;
        let cfg: Self =
            toml::from_str(&text).with_context(|| format!("parse config {}", path.display()))?;
        Ok(cfg)
    }

    /// Merge another config on top (non-default-ish fields still fully replace sections).
    pub fn merge_from(&mut self, other: Self) {
        *self = other;
    }
}

/// Discover config path: explicit → SAAIOS_CONFIG → ./saaios.toml → /etc/saaios/saaios.toml.
pub fn discover_config_path(explicit: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(p) = explicit {
        return Some(p);
    }
    if let Ok(p) = std::env::var("SAAIOS_CONFIG") {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }
    let candidates = [
        PathBuf::from("saaios.toml"),
        PathBuf::from("/etc/saaios/saaios.toml"),
    ];
    candidates.into_iter().find(|p| p.exists())
}

pub fn resolve(overrides: CliOverrides) -> Result<ResolvedSettings> {
    let config_path = discover_config_path(overrides.config.clone());
    let mut cfg = SaaiOsConfig::default();
    if let Some(ref path) = config_path {
        cfg = SaaiOsConfig::load_file(path)?;
    }

    // Env overlays for provider secrets/endpoints (env wins over file).
    if let Ok(v) = std::env::var("SAAIOS_API_BASE") {
        cfg.provider.api_base = Some(v);
    }
    if let Ok(v) = std::env::var("SAAIOS_API_KEY") {
        cfg.provider.api_key = Some(v);
    }
    if let Ok(v) = std::env::var("SAAIOS_MODEL") {
        cfg.provider.model = Some(v);
    }
    if let Ok(v) = std::env::var("SAAIOS_LOCAL_BASE") {
        cfg.provider.local_base = Some(v);
    }
    if let Ok(v) = std::env::var("SAAIOS_LOCAL_MODEL") {
        cfg.provider.local_model = Some(v);
    }
    if let Ok(v) = std::env::var("SAAIOS_PROVIDER") {
        if overrides.provider.is_none() {
            cfg.provider.kind = v;
        }
    }
    if std::env::var("SAAIOS_MODE").ok().as_deref() == Some("mock") {
        cfg.provider.kind = "mock".into();
        cfg.runtime.mock_planner = true;
    }

    let real_linux = cfg.runtime.real_linux || overrides.real_linux;
    let mut mock_planner = cfg.runtime.mock_planner || overrides.mock_planner;
    let mut provider_kind = overrides
        .provider
        .clone()
        .unwrap_or_else(|| cfg.provider.kind.clone());

    if overrides.mock || std::env::var("SAAIOS_MODE").ok().as_deref() == Some("mock") {
        provider_kind = "mock".into();
    }
    if overrides.mock_planner || std::env::var("SAAIOS_MODE").ok().as_deref() == Some("mock") {
        mock_planner = true;
        provider_kind = "mock".into();
    }

    let sock = overrides.sock.unwrap_or(cfg.runtime.sock);
    let audit = overrides.audit.unwrap_or(cfg.runtime.audit);
    let memory_path = overrides.memory.unwrap_or(cfg.memory.path);
    let memory_enabled = cfg.memory.enabled && !overrides.no_memory;
    let automation_enabled = cfg.automation.enabled && !overrides.no_automation;

    let mut telemetry_enabled = match cfg.telemetry.mode {
        TelemetryMode::On => true,
        TelemetryMode::Off => false,
        TelemetryMode::Auto => real_linux,
    };
    if overrides.telemetry {
        telemetry_enabled = true;
    }
    if overrides.no_telemetry {
        telemetry_enabled = false;
    }

    if !overrides.telemetry && !overrides.no_telemetry {
        if let Ok(v) = std::env::var("SAAIOS_TELEMETRY") {
            let on = matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "on");
            let off = matches!(v.as_str(), "0" | "false" | "FALSE" | "no" | "off");
            if on {
                telemetry_enabled = true;
            } else if off {
                telemetry_enabled = false;
            }
        }
    }

    let max_concurrent = overrides
        .max_concurrent
        .unwrap_or(cfg.budgets.max_concurrent)
        .max(1);
    let request_timeout_secs = overrides
        .request_timeout_secs
        .unwrap_or(cfg.budgets.request_timeout_secs)
        .max(1);
    let max_tool_iters = overrides
        .max_tool_iters
        .unwrap_or(cfg.budgets.max_tool_iters)
        .max(1);
    let telemetry_interval_secs = overrides
        .telemetry_interval_secs
        .unwrap_or(cfg.telemetry.interval_secs)
        .max(1);

    Ok(ResolvedSettings {
        config_path,
        sock,
        audit,
        real_linux,
        mock_planner,
        provider_kind,
        api_base: cfg.provider.api_base,
        api_key: cfg.provider.api_key,
        model: cfg.provider.model,
        local_base: cfg.provider.local_base,
        local_model: cfg.provider.local_model,
        max_concurrent,
        request_timeout_secs,
        max_tool_iters,
        memory_enabled,
        memory_path,
        automation_enabled,
        telemetry_enabled,
        telemetry_interval_secs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn parse_example_toml() {
        let raw = r#"
[runtime]
sock = "/run/saaios/saaios.sock"
audit = "/var/lib/saaios/audit.jsonl"
real_linux = true

[provider]
kind = "local"
local_model = "llama3.2"

[budgets]
max_concurrent = 2
request_timeout_secs = 45
max_tool_iters = 8

[memory]
enabled = true
path = "/var/lib/saaios/memory.jsonl"

[automation]
enabled = true

[telemetry]
mode = "on"
interval_secs = 15
"#;
        let cfg: SaaiOsConfig = toml::from_str(raw).unwrap();
        assert!(cfg.runtime.real_linux);
        assert_eq!(cfg.provider.kind, "local");
        assert_eq!(cfg.budgets.max_concurrent, 2);
        assert_eq!(cfg.telemetry.mode, TelemetryMode::On);
        assert_eq!(cfg.telemetry.interval_secs, 15);
    }

    #[test]
    fn resolve_cli_overrides_file() {
        let mut f = NamedTempFile::new().unwrap();
        write!(
            f,
            r#"
[provider]
kind = "remote"
[runtime]
real_linux = false
[telemetry]
mode = "off"
"#
        )
        .unwrap();

        let settings = resolve(CliOverrides {
            config: Some(f.path().to_path_buf()),
            provider: Some("mock".into()),
            real_linux: true,
            telemetry: true,
            ..Default::default()
        })
        .unwrap();

        assert_eq!(settings.provider_kind, "mock");
        assert!(settings.real_linux);
        assert!(settings.telemetry_enabled);
        assert_eq!(settings.config_path.as_deref(), Some(f.path()));
    }

    #[test]
    fn telemetry_auto_follows_real_linux() {
        let settings = resolve(CliOverrides {
            real_linux: true,
            ..Default::default()
        })
        .unwrap();
        assert!(settings.telemetry_enabled);

        let settings = resolve(CliOverrides::default()).unwrap();
        assert!(!settings.telemetry_enabled);
    }

    #[test]
    fn no_memory_disables() {
        let settings = resolve(CliOverrides {
            no_memory: true,
            ..Default::default()
        })
        .unwrap();
        assert!(!settings.memory_enabled);
    }
}
