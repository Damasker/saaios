//! ADR-030: Intent/Task/Action/Result live as new `entity_type` values in
//! the existing `saai-entity-store` (S06), not as a port of Platform
//! Track's `policy-engine`/`tool-registry`. This module is the pure,
//! host-testable half of that decision: entity-type/property schema,
//! the workflow status state machine, idempotency, and the one
//! non-dangerous action Change 2 actually runs. No IO here -- the wire
//! client lives in `client.rs`.

use saai_entity_protocol::Entity;
use serde_json::{json, Map, Value};
use uuid::Uuid;

pub const INTENT_TYPE: &str = "saaios.intent";
pub const TASK_TYPE: &str = "saaios.task";
pub const ACTION_TYPE: &str = "saaios.action";
pub const RESULT_TYPE: &str = "saaios.result";

/// Non-dangerous by construction: it only reads the intent text it was
/// itself given and computes a deterministic transform. It reaches
/// nothing outside this process, so it never needs `policy-engine`'s
/// confirmation path -- that path is Change 3's job, for the first
/// Action kind that actually can.
pub const ECHO_ACTION_KIND: &str = "echo";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowStatus {
    Pending,
    Running,
    WaitingConfirmation,
    Done,
    Failed,
    Cancelled,
}

impl WorkflowStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::WaitingConfirmation => "waiting_confirmation",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "running" => Some(Self::Running),
            "waiting_confirmation" => Some(Self::WaitingConfirmation),
            "done" => Some(Self::Done),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

/// Change 2 only ever drives Running -> {Done, Failed}: the intent is
/// turned into a task and its one non-dangerous action immediately, no
/// confirmation gate. `WaitingConfirmation`/`Cancelled` are part of
/// ADR-030's decided vocabulary but have no producer yet -- their
/// transitions are deliberately left undeclared here rather than
/// guessed at, so this validator only vouches for the state machine
/// this Change actually exercises. Change 3 (dangerous actions) extends
/// this match, it does not need to replace it.
pub fn valid_transition(from: WorkflowStatus, to: WorkflowStatus) -> bool {
    use WorkflowStatus::*;
    matches!(
        (from, to),
        (Pending, Running) | (Running, Done) | (Running, Failed)
    )
}

pub fn task_properties(intent_id: Uuid, status: WorkflowStatus) -> Map<String, Value> {
    let mut map = Map::new();
    map.insert("intent_id".into(), json!(intent_id.to_string()));
    map.insert("status".into(), json!(status.as_str()));
    map
}

pub fn action_properties(
    task_id: Uuid,
    kind: &str,
    status: WorkflowStatus,
    input: &Value,
    output: Option<&Value>,
) -> Map<String, Value> {
    let mut map = Map::new();
    map.insert("task_id".into(), json!(task_id.to_string()));
    map.insert("kind".into(), json!(kind));
    map.insert("status".into(), json!(status.as_str()));
    map.insert("input".into(), input.clone());
    if let Some(output) = output {
        map.insert("output".into(), output.clone());
    }
    map
}

pub fn result_properties(task_id: Uuid, action_id: Uuid, summary: &str) -> Map<String, Value> {
    let mut map = Map::new();
    map.insert("task_id".into(), json!(task_id.to_string()));
    map.insert("action_id".into(), json!(action_id.to_string()));
    map.insert("summary".into(), json!(summary));
    map
}

/// Reads the same `intent_id` property `task_properties` writes.
/// Isolated as its own function so both the idempotency check and any
/// future reconciliation logic read the property the exact same way --
/// a hand-copied `.get("intent_id")` at each call site is how the two
/// would silently drift.
pub fn intent_id_of(entity: &Entity) -> Option<Uuid> {
    entity
        .properties
        .get("intent_id")
        .and_then(Value::as_str)
        .and_then(|raw| raw.parse().ok())
}

/// ADR-030's idempotency requirement: replaying the same intent
/// (duplicate event delivery, or a restart re-scanning entities that
/// were already processed) must not spawn a second Task. This is the
/// one check that stands between "at least once" event delivery and a
/// non-dangerous action actually running twice.
pub fn has_task_for_intent(existing_tasks: &[Entity], intent_id: Uuid) -> bool {
    existing_tasks
        .iter()
        .any(|task| intent_id_of(task) == Some(intent_id))
}

/// The one non-dangerous Action Change 2 runs. Deterministic and pure:
/// same input always produces the same output, which is what makes the
/// idempotency guard above sufficient on its own -- there's no
/// observable difference between running it once and running it twice
/// on the same intent, other than the duplicate Task/Result this
/// function has nothing to do with preventing.
pub fn run_echo_action(text: &str) -> Value {
    json!({
        "echo": text.to_uppercase(),
        "chars": text.chars().count(),
        "words": text.split_whitespace().count(),
    })
}

pub fn result_summary(text: &str, output: &Value) -> String {
    let echo = output.get("echo").and_then(Value::as_str).unwrap_or("");
    let chars = output.get("chars").and_then(Value::as_u64).unwrap_or(0);
    let words = output.get("words").and_then(Value::as_u64).unwrap_or(0);
    format!("Намерение «{text}» обработано: {chars} симв., {words} слов -> {echo}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use saai_entity_protocol::Entity;

    fn entity(entity_type: &str, properties: Map<String, Value>) -> Entity {
        let now = Utc::now();
        Entity {
            schema: 1,
            id: Uuid::new_v4(),
            space_id: "home".into(),
            entity_type: entity_type.into(),
            title: "test".into(),
            properties,
            revision: 1,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn status_round_trips_through_its_own_string_form() {
        for status in [
            WorkflowStatus::Pending,
            WorkflowStatus::Running,
            WorkflowStatus::WaitingConfirmation,
            WorkflowStatus::Done,
            WorkflowStatus::Failed,
            WorkflowStatus::Cancelled,
        ] {
            assert_eq!(WorkflowStatus::parse(status.as_str()), Some(status));
        }
    }

    #[test]
    fn status_rejects_garbage() {
        assert_eq!(WorkflowStatus::parse("in_progress"), None);
        assert_eq!(WorkflowStatus::parse(""), None);
    }

    #[test]
    fn change_2_transitions_are_accepted() {
        use WorkflowStatus::*;
        assert!(valid_transition(Pending, Running));
        assert!(valid_transition(Running, Done));
        assert!(valid_transition(Running, Failed));
    }

    #[test]
    fn transitions_outside_change_2_scope_are_rejected() {
        use WorkflowStatus::*;
        assert!(!valid_transition(Done, Running));
        assert!(!valid_transition(Cancelled, Done));
        assert!(!valid_transition(Pending, Done));
        assert!(!valid_transition(Failed, Done));
    }

    #[test]
    fn intent_id_round_trips_through_task_properties() {
        let intent_id = Uuid::new_v4();
        let task = entity(
            TASK_TYPE,
            task_properties(intent_id, WorkflowStatus::Running),
        );
        assert_eq!(intent_id_of(&task), Some(intent_id));
    }

    #[test]
    fn intent_id_of_is_none_for_entities_without_the_property() {
        let stray = entity(TASK_TYPE, Map::new());
        assert_eq!(intent_id_of(&stray), None);
    }

    #[test]
    fn idempotency_guard_finds_an_existing_task_for_the_same_intent() {
        let intent_id = Uuid::new_v4();
        let tasks = vec![entity(
            TASK_TYPE,
            task_properties(intent_id, WorkflowStatus::Done),
        )];
        assert!(has_task_for_intent(&tasks, intent_id));
    }

    #[test]
    fn idempotency_guard_does_not_match_a_different_intent() {
        let tasks = vec![entity(
            TASK_TYPE,
            task_properties(Uuid::new_v4(), WorkflowStatus::Done),
        )];
        assert!(!has_task_for_intent(&tasks, Uuid::new_v4()));
    }

    #[test]
    fn idempotency_guard_is_false_on_an_empty_task_list() {
        assert!(!has_task_for_intent(&[], Uuid::new_v4()));
    }

    #[test]
    fn echo_action_is_deterministic() {
        let first = run_echo_action("hi there");
        let second = run_echo_action("hi there");
        assert_eq!(first, second);
        assert_eq!(first["echo"], json!("HI THERE"));
        assert_eq!(first["chars"], json!(8));
        assert_eq!(first["words"], json!(2));
    }

    #[test]
    fn echo_action_counts_unicode_chars_not_bytes() {
        let output = run_echo_action("привет");
        assert_eq!(output["chars"], json!(6));
    }

    #[test]
    fn result_summary_embeds_the_computed_output() {
        let output = run_echo_action("hi");
        let summary = result_summary("hi", &output);
        assert!(summary.contains("HI"));
        assert!(summary.contains("2 симв"));
        assert!(summary.contains("1 слов"));
    }
}
