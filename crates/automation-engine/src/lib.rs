use anyhow::Result;
use audit_log::AuditLog;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use event_bus::EventBus;
use protocol::{Envelope, MessageKind};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TriggerKind {
    /// Match MessageKind::Event with payload.event == name
    NamedEvent { name: String },
    /// Match tool_result for a tool with optional numeric threshold on a field
    ToolResultThreshold {
        tool: String,
        field: String,
        gte: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AutomationAction {
    Notify {
        message: String,
    },
    SuggestDiagnose {
        prompt: String,
    },
    /// Emit a request that the runtime may execute as a real diagnose.
    AutoDiagnose {
        prompt: String,
    },
    EmitEvent {
        name: String,
        payload: Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub trigger: TriggerKind,
    pub action: AutomationAction,
    /// Minimum seconds between firings of this rule.
    pub cooldown_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutomationFire {
    pub rule_id: String,
    pub rule_name: String,
    pub action: AutomationAction,
    pub correlation_id: Uuid,
    pub causation_id: Uuid,
    pub detail: String,
}

#[derive(Default)]
struct CooldownState {
    last_fired: HashMap<String, DateTime<Utc>>,
}

pub struct AutomationEngine {
    rules: Vec<AutomationRule>,
    bus: EventBus,
    audit: Arc<AuditLog>,
    cooldowns: Mutex<CooldownState>,
}

impl AutomationEngine {
    pub fn new(bus: EventBus, audit: Arc<AuditLog>, rules: Vec<AutomationRule>) -> Self {
        Self {
            rules,
            bus,
            audit,
            cooldowns: Mutex::new(CooldownState::default()),
        }
    }

    /// Enable/disable the built-in `high-cpu-auto-diagnose` rule.
    pub fn set_auto_diagnose_rule(&mut self, enabled: bool) {
        if let Some(rule) = self
            .rules
            .iter_mut()
            .find(|r| r.id == "high-cpu-auto-diagnose")
        {
            rule.enabled = enabled;
        }
    }

    pub fn rules(&self) -> &[AutomationRule] {
        &self.rules
    }

    pub fn default_rules() -> Vec<AutomationRule> {
        vec![
            AutomationRule {
                id: "high-cpu-suggest".into(),
                name: "High CPU suggests diagnose".into(),
                enabled: true,
                trigger: TriggerKind::ToolResultThreshold {
                    tool: "system.metrics".into(),
                    field: "cpu_usage".into(),
                    gte: 85.0,
                },
                action: AutomationAction::SuggestDiagnose {
                    prompt: "Почему система тормозит?".into(),
                },
                cooldown_secs: 30,
            },
            AutomationRule {
                id: "high-cpu-auto-diagnose".into(),
                name: "High CPU auto diagnose".into(),
                // Enabled at runtime when config automation.auto_diagnose=true
                // (see AutomationEngine::with_auto_diagnose_rule).
                enabled: false,
                trigger: TriggerKind::ToolResultThreshold {
                    tool: "system.metrics".into(),
                    field: "cpu_usage".into(),
                    gte: 85.0,
                },
                action: AutomationAction::AutoDiagnose {
                    prompt: "Почему система тормозит?".into(),
                },
                cooldown_secs: 120,
            },
            AutomationRule {
                id: "high-cpu-notify".into(),
                name: "High CPU notification".into(),
                enabled: true,
                trigger: TriggerKind::ToolResultThreshold {
                    tool: "system.metrics".into(),
                    field: "cpu_usage".into(),
                    gte: 85.0,
                },
                action: AutomationAction::Notify {
                    message: "CPU usage is critically high (>= 85%)".into(),
                },
                cooldown_secs: 30,
            },
            AutomationRule {
                id: "high-mem-notify".into(),
                name: "High memory notification".into(),
                enabled: true,
                trigger: TriggerKind::ToolResultThreshold {
                    tool: "system.metrics".into(),
                    field: "mem_used_pct".into(),
                    gte: 90.0,
                },
                action: AutomationAction::Notify {
                    message: "Memory usage is critically high (>= 90%)".into(),
                },
                cooldown_secs: 60,
            },
            AutomationRule {
                id: "high-temp-notify".into(),
                name: "High temperature notification".into(),
                enabled: true,
                trigger: TriggerKind::ToolResultThreshold {
                    tool: "system.temperature".into(),
                    field: "celsius".into(),
                    gte: 80.0,
                },
                action: AutomationAction::Notify {
                    message: "SoC/CPU temperature is high (>= 80C)".into(),
                },
                cooldown_secs: 60,
            },
            AutomationRule {
                id: "named-temp-warning".into(),
                name: "Temperature warning passthrough".into(),
                enabled: true,
                trigger: TriggerKind::NamedEvent {
                    name: "TemperatureWarning".into(),
                },
                action: AutomationAction::Notify {
                    message: "Temperature warning received".into(),
                },
                cooldown_secs: 10,
            },
        ]
    }

    pub fn evaluate(&self, env: &Envelope) -> Vec<AutomationFire> {
        let mut fires = Vec::new();
        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }
            if let Some(detail) = self.matches(rule, env) {
                if self.in_cooldown(&rule.id, rule.cooldown_secs) {
                    continue;
                }
                fires.push(AutomationFire {
                    rule_id: rule.id.clone(),
                    rule_name: rule.name.clone(),
                    action: rule.action.clone(),
                    correlation_id: env.correlation_id,
                    causation_id: env.msg_id,
                    detail,
                });
            }
        }
        fires
    }

    pub fn apply(&self, fire: &AutomationFire) -> Result<Envelope> {
        self.mark_fired(&fire.rule_id);
        let payload = match &fire.action {
            AutomationAction::Notify { message } => json!({
                "event": "AutomationNotify",
                "rule_id": fire.rule_id,
                "rule_name": fire.rule_name,
                "message": message,
                "detail": fire.detail,
            }),
            AutomationAction::SuggestDiagnose { prompt } => json!({
                "event": "AutomationSuggestDiagnose",
                "rule_id": fire.rule_id,
                "rule_name": fire.rule_name,
                "prompt": prompt,
                "detail": fire.detail,
            }),
            AutomationAction::AutoDiagnose { prompt } => json!({
                "event": "AutomationAutoDiagnose",
                "rule_id": fire.rule_id,
                "rule_name": fire.rule_name,
                "prompt": prompt,
                "detail": fire.detail,
            }),
            AutomationAction::EmitEvent { name, payload } => json!({
                "event": name,
                "rule_id": fire.rule_id,
                "rule_name": fire.rule_name,
                "detail": fire.detail,
                "data": payload,
            }),
        };

        let out = Envelope::new(
            MessageKind::Event,
            fire.correlation_id,
            Some(fire.causation_id),
            payload,
        );
        self.audit.append_envelope(&out)?;
        self.bus.publish_envelope(out.clone());
        info!(
            rule_id = %fire.rule_id,
            correlation_id = %fire.correlation_id,
            "automation rule fired"
        );
        Ok(out)
    }

    pub async fn run_loop(self: Arc<Self>) {
        let mut rx = self.bus.subscribe();
        // Yield so callers that spawn us can rely on subscription being active
        // after a short await / sleep in tests.
        tokio::task::yield_now().await;
        loop {
            match rx.recv().await {
                Ok(env) => {
                    // Avoid reacting to our own automation events forever.
                    if env.kind == MessageKind::Event {
                        if let Some(name) = env.payload.get("event").and_then(|v| v.as_str()) {
                            if name.starts_with("Automation") {
                                continue;
                            }
                        }
                    }
                    for fire in self.evaluate(&env) {
                        if let Err(e) = self.apply(&fire) {
                            warn!(error = %e, "failed to apply automation fire");
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    warn!(skipped = n, "automation bus lagged");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    warn!("automation bus closed");
                    break;
                }
            }
        }
    }

    /// Subscribe immediately and return a background task handle.
    pub fn spawn(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        let mut rx = self.bus.subscribe();
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(env) => {
                        if env.kind == MessageKind::Event {
                            if let Some(name) = env.payload.get("event").and_then(|v| v.as_str()) {
                                if name.starts_with("Automation") {
                                    continue;
                                }
                            }
                        }
                        for fire in self.evaluate(&env) {
                            if let Err(e) = self.apply(&fire) {
                                warn!(error = %e, "failed to apply automation fire");
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!(skipped = n, "automation bus lagged");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        })
    }

    fn matches(&self, rule: &AutomationRule, env: &Envelope) -> Option<String> {
        match &rule.trigger {
            TriggerKind::NamedEvent { name } => {
                if env.kind != MessageKind::Event {
                    return None;
                }
                let event_name = env.payload.get("event")?.as_str()?;
                if event_name == name {
                    Some(format!("named event `{name}`"))
                } else {
                    None
                }
            }
            TriggerKind::ToolResultThreshold { tool, field, gte } => {
                if env.kind != MessageKind::ToolResult {
                    return None;
                }
                if env.payload.get("tool")?.as_str()? != tool {
                    return None;
                }
                if !env.payload.get("ok")?.as_bool()? {
                    return None;
                }
                let value = env
                    .payload
                    .pointer(&format!("/output/{field}"))
                    .and_then(|v| v.as_f64())?;
                if value >= *gte {
                    Some(format!("{tool}.{field}={value} >= {gte}"))
                } else {
                    None
                }
            }
        }
    }

    fn in_cooldown(&self, rule_id: &str, cooldown_secs: u64) -> bool {
        let guard = self.cooldowns.lock().expect("cooldown lock");
        if let Some(last) = guard.last_fired.get(rule_id) {
            let elapsed = Utc::now().signed_duration_since(*last);
            if elapsed < ChronoDuration::seconds(cooldown_secs as i64) {
                return true;
            }
        }
        false
    }

    fn mark_fired(&self, rule_id: &str) {
        if let Ok(mut guard) = self.cooldowns.lock() {
            guard.last_fired.insert(rule_id.to_string(), Utc::now());
        }
    }
}

/// Pure matching helper for tests / offline evaluation.
pub fn evaluate_once(rules: Vec<AutomationRule>, env: &Envelope) -> Vec<AutomationFire> {
    let matcher = Matcher;
    let mut fires = Vec::new();
    for rule in rules {
        if !rule.enabled {
            continue;
        }
        if let Some(detail) = matcher.matches(&rule, env) {
            fires.push(AutomationFire {
                rule_id: rule.id,
                rule_name: rule.name,
                action: rule.action,
                correlation_id: env.correlation_id,
                causation_id: env.msg_id,
                detail,
            });
        }
    }
    fires
}

struct Matcher;

impl Matcher {
    fn matches(&self, rule: &AutomationRule, env: &Envelope) -> Option<String> {
        match &rule.trigger {
            TriggerKind::NamedEvent { name } => {
                if env.kind != MessageKind::Event {
                    return None;
                }
                let event_name = env.payload.get("event")?.as_str()?;
                (event_name == name).then(|| format!("named event `{name}`"))
            }
            TriggerKind::ToolResultThreshold { tool, field, gte } => {
                if env.kind != MessageKind::ToolResult {
                    return None;
                }
                if env.payload.get("tool")?.as_str()? != tool {
                    return None;
                }
                if env.payload.get("ok")?.as_bool() != Some(true) {
                    return None;
                }
                let value = env
                    .payload
                    .pointer(&format!("/output/{field}"))
                    .and_then(|v| v.as_f64())?;
                (value >= *gte).then(|| format!("{tool}.{field}={value} >= {gte}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[test]
    fn high_cpu_tool_result_triggers_rules() {
        let corr = Uuid::new_v4();
        let env = Envelope::new(
            MessageKind::ToolResult,
            corr,
            None,
            json!({
                "call_id": Uuid::new_v4(),
                "tool": "system.metrics",
                "ok": true,
                "output": {"cpu_usage": 97.0, "load_average": 8.4}
            }),
        );
        let fires = evaluate_once(AutomationEngine::default_rules(), &env);
        assert!(fires.iter().any(|f| f.rule_id == "high-cpu-suggest"));
        assert!(fires.iter().any(|f| f.rule_id == "high-cpu-notify"));
        assert!(!fires.iter().any(|f| f.rule_id == "high-cpu-auto-diagnose"));
    }

    #[test]
    fn auto_diagnose_rule_can_be_enabled() {
        let mut engine = AutomationEngine::new(
            EventBus::new(8),
            Arc::new(AuditLog::open(tempdir().unwrap().path().join("a.jsonl")).unwrap()),
            AutomationEngine::default_rules(),
        );
        assert!(
            !engine
                .rules()
                .iter()
                .find(|r| r.id == "high-cpu-auto-diagnose")
                .unwrap()
                .enabled
        );
        engine.set_auto_diagnose_rule(true);
        assert!(
            engine
                .rules()
                .iter()
                .find(|r| r.id == "high-cpu-auto-diagnose")
                .unwrap()
                .enabled
        );

        let env = Envelope::new(
            MessageKind::ToolResult,
            Uuid::new_v4(),
            None,
            json!({
                "tool": "system.metrics",
                "ok": true,
                "output": {"cpu_usage": 97.0}
            }),
        );
        let fires = engine.evaluate(&env);
        assert!(fires.iter().any(|f| f.rule_id == "high-cpu-auto-diagnose"));
        let applied = engine
            .apply(
                fires
                    .iter()
                    .find(|f| f.rule_id == "high-cpu-auto-diagnose")
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(
            applied.payload.get("event").and_then(|v| v.as_str()),
            Some("AutomationAutoDiagnose")
        );
    }

    #[test]
    fn low_cpu_does_not_trigger() {
        let env = Envelope::new(
            MessageKind::ToolResult,
            Uuid::new_v4(),
            None,
            json!({
                "tool": "system.metrics",
                "ok": true,
                "output": {"cpu_usage": 12.0}
            }),
        );
        let fires = evaluate_once(AutomationEngine::default_rules(), &env);
        assert!(fires.is_empty());
    }

    #[tokio::test]
    async fn apply_writes_audit_and_publishes() {
        let dir = tempdir().unwrap();
        let audit = Arc::new(AuditLog::open(dir.path().join("a.jsonl")).unwrap());
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();
        let engine = AutomationEngine::new(bus, audit.clone(), AutomationEngine::default_rules());
        let corr = Uuid::new_v4();
        let env = Envelope::new(
            MessageKind::ToolResult,
            corr,
            None,
            json!({
                "tool": "system.metrics",
                "ok": true,
                "output": {"cpu_usage": 90.0}
            }),
        );
        let fires = engine.evaluate(&env);
        assert!(!fires.is_empty());
        engine.apply(&fires[0]).unwrap();
        let got = rx.recv().await.unwrap();
        assert_eq!(got.kind, MessageKind::Event);
        let chain = audit.list_by_correlation(corr).unwrap();
        assert!(chain.iter().any(|r| r.kind == MessageKind::Event));
    }

    #[test]
    fn cooldown_suppresses_second_fire() {
        let dir = tempdir().unwrap();
        let audit = Arc::new(AuditLog::open(dir.path().join("a.jsonl")).unwrap());
        let bus = EventBus::new(8);
        let engine = AutomationEngine::new(
            bus,
            audit,
            vec![AutomationRule {
                id: "r1".into(),
                name: "r1".into(),
                enabled: true,
                trigger: TriggerKind::ToolResultThreshold {
                    tool: "system.metrics".into(),
                    field: "cpu_usage".into(),
                    gte: 50.0,
                },
                action: AutomationAction::Notify {
                    message: "hi".into(),
                },
                cooldown_secs: 60,
            }],
        );
        let env = Envelope::new(
            MessageKind::ToolResult,
            Uuid::new_v4(),
            None,
            json!({"tool":"system.metrics","ok":true,"output":{"cpu_usage":99.0}}),
        );
        let first = engine.evaluate(&env);
        assert_eq!(first.len(), 1);
        engine.apply(&first[0]).unwrap();
        let second = engine.evaluate(&env);
        assert!(second.is_empty());
    }
}
