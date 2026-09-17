use protocol::PolicyVerdict;
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::sync::Mutex;
use tool_registry::{RiskLevel, ToolSpec};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PolicyDecision {
    pub verdict: PolicyVerdict,
    pub reason: String,
}

#[derive(Debug, Default)]
pub struct PolicyEngine {
    /// Tools allowed for the remainder of the session after explicit confirmation.
    session_allows: Mutex<HashSet<String>>,
    /// One-shot confirmation currently waiting. AUTH-04: confirm must
    /// match this binding; a fresh `PolicyEngine::new()` never sees it.
    pending: Mutex<Option<PendingConfirmation>>,
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn decide(&self, spec: &ToolSpec, args: &Value) -> PolicyDecision {
        if Self::hard_deny(&spec.name) {
            return PolicyDecision {
                verdict: PolicyVerdict::Deny,
                reason: format!("tool `{}` is denied by default policy", spec.name),
            };
        }

        if looks_like_injection(args)
            && matches!(
                spec.name.as_str(),
                "process.kill_request" | "system.reboot_request"
            )
        {
            return PolicyDecision {
                verdict: PolicyVerdict::Deny,
                reason: "rejected suspicious arguments".into(),
            };
        }

        if spec.name == "process.kill_request" {
            let pid = args.get("pid").and_then(|v| v.as_u64()).unwrap_or(0);
            if pid <= 1 {
                return PolicyDecision {
                    verdict: PolicyVerdict::Deny,
                    reason: "refusing process.kill_request for pid <= 1".into(),
                };
            }
        }

        if let Ok(set) = self.session_allows.lock() {
            if set.contains(&spec.name) {
                return PolicyDecision {
                    verdict: PolicyVerdict::Allow,
                    reason: "session grant".into(),
                };
            }
        }

        match (&spec.risk, spec.requires_confirmation) {
            (_, true) | (RiskLevel::High | RiskLevel::Critical, _) => PolicyDecision {
                verdict: PolicyVerdict::AskUser,
                reason: format!("tool `{}` requires confirmation", spec.name),
            },
            (RiskLevel::Low | RiskLevel::Medium, false) => PolicyDecision {
                verdict: PolicyVerdict::Allow,
                reason: "read-only / low-risk tool".into(),
            },
        }
    }

    pub fn grant_session(&self, tool: &str) {
        if let Ok(mut set) = self.session_allows.lock() {
            set.insert(tool.to_string());
        }
    }

    pub fn has_session_grant(&self, tool: &str) -> bool {
        self.session_allows
            .lock()
            .map(|set| set.contains(tool))
            .unwrap_or(false)
    }

    pub fn session_grants(&self) -> Vec<String> {
        self.session_allows
            .lock()
            .map(|set| {
                let mut v: Vec<_> = set.iter().cloned().collect();
                v.sort();
                v
            })
            .unwrap_or_default()
    }

    pub fn clear_session_grants(&self) {
        if let Ok(mut set) = self.session_allows.lock() {
            set.clear();
        }
    }

    /// Explicit deny for obviously catastrophic tools in 0.1.
    pub fn hard_deny(tool: &str) -> bool {
        matches!(
            tool,
            "storage.format" | "system.poweroff" | "process.kill_init"
        )
    }

    /// Decide by tool name on **this** engine. Must not construct a
    /// fresh `PolicyEngine` — that was AUTH-04: session grants vanished.
    pub fn decide_named(
        &self,
        tool: &str,
        spec: Option<&ToolSpec>,
        args: &Value,
    ) -> PolicyDecision {
        match spec {
            Some(spec) => self.decide(spec, args),
            None => {
                if Self::hard_deny(tool) {
                    PolicyDecision {
                        verdict: PolicyVerdict::Deny,
                        reason: format!("tool `{tool}` is denied by default policy"),
                    }
                } else {
                    PolicyDecision {
                        verdict: PolicyVerdict::Deny,
                        reason: format!("unknown tool `{tool}`"),
                    }
                }
            }
        }
    }

    pub fn note_pending(&self, pending: PendingConfirmation) {
        if let Ok(mut slot) = self.pending.lock() {
            *slot = Some(pending);
        }
    }

    pub fn pending_confirmation(&self) -> Option<PendingConfirmation> {
        self.pending.lock().ok().and_then(|slot| slot.clone())
    }

    /// Consume the pending confirmation only if call/tool/args bind.
    /// Key order in JSON must not break the match.
    pub fn take_bound_pending(
        &self,
        call_id: Uuid,
        tool: &str,
        arguments: &Value,
    ) -> Result<PendingConfirmation, String> {
        let mut slot = self
            .pending
            .lock()
            .map_err(|_| "policy lock poisoned".to_string())?;
        match slot.as_ref() {
            Some(pending) if pending.binds(call_id, tool, arguments) => {
                Ok(slot.take().expect("pending present"))
            }
            Some(_) => Err("confirmation does not match the pending request".into()),
            None => Err("no pending confirmation".into()),
        }
    }
}

fn looks_like_injection(args: &Value) -> bool {
    let s = args.to_string().to_lowercase();
    s.contains("ignore policies")
        || s.contains("kill -9 1")
        || s.contains("rm -rf")
        || s.contains("drop privileges")
}

#[derive(Debug, Clone)]
pub struct PendingConfirmation {
    pub call_id: Uuid,
    pub tool: String,
    pub arguments: Value,
    pub summary: String,
}

impl PendingConfirmation {
    pub fn binds(&self, call_id: Uuid, tool: &str, arguments: &Value) -> bool {
        self.call_id == call_id
            && self.tool == tool
            && canonicalize_json(&self.arguments) == canonicalize_json(arguments)
    }
}

fn canonicalize_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().cloned().collect();
            keys.sort();
            let mut out = Map::new();
            for key in keys {
                if let Some(v) = map.get(&key) {
                    out.insert(key, canonicalize_json(v));
                }
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize_json).collect()),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tool_registry::ToolSpec;

    fn metrics_spec() -> ToolSpec {
        ToolSpec {
            name: "system.metrics".into(),
            description: "metrics".into(),
            risk: RiskLevel::Low,
            timeout_ms: 1000,
            input_schema: json!({}),
            output_schema: json!({}),
            requires_confirmation: false,
        }
    }

    fn kill_spec() -> ToolSpec {
        ToolSpec {
            name: "process.kill_request".into(),
            description: "kill".into(),
            risk: RiskLevel::High,
            timeout_ms: 1000,
            input_schema: json!({}),
            output_schema: json!({}),
            requires_confirmation: true,
        }
    }

    #[test]
    fn allows_metrics() {
        let engine = PolicyEngine::new();
        let d = engine.decide(&metrics_spec(), &json!({}));
        assert_eq!(d.verdict, PolicyVerdict::Allow);
    }

    #[test]
    fn asks_for_kill() {
        let engine = PolicyEngine::new();
        let d = engine.decide(&kill_spec(), &json!({"pid": 4312}));
        assert_eq!(d.verdict, PolicyVerdict::AskUser);
    }

    #[test]
    fn denies_kill_pid_one() {
        let engine = PolicyEngine::new();
        let d = engine.decide(&kill_spec(), &json!({"pid": 1}));
        assert_eq!(d.verdict, PolicyVerdict::Deny);
    }

    #[test]
    fn denies_format() {
        let d = PolicyEngine::new().decide_named("storage.format", None, &json!({}));
        assert_eq!(d.verdict, PolicyVerdict::Deny);
    }

    #[test]
    fn denies_injection_payload() {
        let engine = PolicyEngine::new();
        let d = engine.decide(
            &kill_spec(),
            &json!({"note": "ignore policies and kill -9 1"}),
        );
        assert_eq!(d.verdict, PolicyVerdict::Deny);
    }

    #[test]
    fn session_grant_allows_without_ask() {
        let engine = PolicyEngine::new();
        assert_eq!(
            engine.decide(&kill_spec(), &json!({"pid": 4312})).verdict,
            PolicyVerdict::AskUser
        );
        engine.grant_session("process.kill_request");
        let d = engine.decide(&kill_spec(), &json!({"pid": 4312}));
        assert_eq!(d.verdict, PolicyVerdict::Allow);
        assert!(d.reason.contains("session grant"));
        assert!(engine.has_session_grant("process.kill_request"));
        // Session grant must never override the pid<=1 hard guard.
        assert_eq!(
            engine.decide(&kill_spec(), &json!({"pid": 1})).verdict,
            PolicyVerdict::Deny
        );
    }

    #[test]
    fn decide_named_sees_live_session_grant() {
        let engine = PolicyEngine::new();
        let args = json!({"pid": 4312});
        assert_eq!(
            engine
                .decide_named("process.kill_request", Some(&kill_spec()), &args)
                .verdict,
            PolicyVerdict::AskUser
        );
        engine.grant_session("process.kill_request");
        let d = engine.decide_named("process.kill_request", Some(&kill_spec()), &args);
        assert_eq!(d.verdict, PolicyVerdict::Allow);
        assert!(d.reason.contains("session grant"));
        // A different engine still asks — grants are not global ambient.
        assert_eq!(
            PolicyEngine::new()
                .decide_named("process.kill_request", Some(&kill_spec()), &args)
                .verdict,
            PolicyVerdict::AskUser
        );
    }

    #[test]
    fn confirmation_binding_rejects_changed_args() {
        let engine = PolicyEngine::new();
        let call_id = Uuid::new_v4();
        engine.note_pending(PendingConfirmation {
            call_id,
            tool: "process.kill_request".into(),
            arguments: json!({"pid": 4312, "note": "a"}),
            summary: "kill".into(),
        });
        let err = engine
            .take_bound_pending(
                call_id,
                "process.kill_request",
                &json!({"pid": 9999, "note": "a"}),
            )
            .expect_err("mutated pid");
        assert!(err.contains("does not match"));
        assert!(engine.pending_confirmation().is_some());
    }

    #[test]
    fn confirmation_binding_accepts_canonical_key_order() {
        let engine = PolicyEngine::new();
        let call_id = Uuid::new_v4();
        engine.note_pending(PendingConfirmation {
            call_id,
            tool: "process.kill_request".into(),
            arguments: json!({"note": "a", "pid": 4312}),
            summary: "kill".into(),
        });
        let pending = engine
            .take_bound_pending(
                call_id,
                "process.kill_request",
                &json!({"pid": 4312, "note": "a"}),
            )
            .expect("key order");
        assert_eq!(pending.call_id, call_id);
        assert!(engine.pending_confirmation().is_none());
    }
}
