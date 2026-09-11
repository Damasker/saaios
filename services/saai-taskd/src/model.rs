//! ADR-030: Intent/Task/Action/Result live as new `entity_type` values in
//! the existing `saai-entity-store` (S06), not as a port of Platform
//! Track's `policy-engine`/`tool-registry`. This module is the pure,
//! host-testable half of that decision: entity-type/property schema,
//! the workflow status state machine, idempotency, the one
//! non-dangerous Action Change 2 runs, and (Change 3) the one
//! dangerous Action that pauses for confirmation instead. No IO here
//! -- the wire client lives in `client.rs`.

use saai_entity_protocol::Entity;
use serde_json::{json, Map, Value};
use uuid::Uuid;

pub const INTENT_TYPE: &str = "saaios.intent";
pub const TASK_TYPE: &str = "saaios.task";
pub const ACTION_TYPE: &str = "saaios.action";
pub const RESULT_TYPE: &str = "saaios.result";

/// Non-dangerous by construction: it only reads the intent text it was
/// itself given and computes a deterministic transform. It reaches
/// nothing outside this process, so it never needs a confirmation gate.
pub const ECHO_ACTION_KIND: &str = "echo";

/// Change 3's one dangerous Action: deletes another entity in the same
/// space. Irreversible from the live store's point of view (the
/// event log keeps history, but the entity itself is gone) and reaches
/// outside the Action's own arguments -- exactly the risk profile S09's
/// Threat/privacy impact section describes. Selected explicitly via an
/// intent's own `action_kind`/`target_entity_id` properties
/// (`dangerous_action_of`) -- free-form keyboard text never reaches
/// this path on its own; parsing "delete X" out of natural language is
/// S10's planner's job, not this Change's.
pub const DELETE_ENTITY_ACTION_KIND: &str = "delete_entity";

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

/// Change 2 drives Running -> {Done, Failed} for its one non-dangerous
/// Action. Change 3 adds the confirmation gate: a dangerous Action's
/// Task starts at `WaitingConfirmation` (never `Running` immediately)
/// and can only reach `Running` through an explicit, separately
/// authored update (`saai-shell`'s confirmation screen, a live touch --
/// never something this daemon writes to itself), or `Cancelled`
/// through the same kind of explicit decline. There is deliberately no
/// `WaitingConfirmation -> Done` or any reboot/restart-only path to
/// `Running` -- resuming after a restart re-enters this exact same
/// state machine through the same external-confirmation edge, per S09's
/// Threat/privacy impact requirement that a resumed Task never
/// auto-executes on a cached decision.
pub fn valid_transition(from: WorkflowStatus, to: WorkflowStatus) -> bool {
    use WorkflowStatus::*;
    matches!(
        (from, to),
        (Pending, Running)
            | (Running, Done)
            | (Running, Failed)
            | (Pending, WaitingConfirmation)
            | (WaitingConfirmation, Running)
            | (WaitingConfirmation, Cancelled)
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

/// Same idea as `intent_id_of`, for an Action's `task_id` property.
pub fn task_id_of(entity: &Entity) -> Option<Uuid> {
    entity
        .properties
        .get("task_id")
        .and_then(Value::as_str)
        .and_then(|raw| raw.parse().ok())
}

/// Reads an entity's own `status` property back into a `WorkflowStatus`
/// -- the single source of truth `try_resume_confirmed_task`'s
/// idempotency check reads, so "has this already run" is always asked
/// of the entity store itself, never of this process's own memory.
pub fn status_of(entity: &Entity) -> Option<WorkflowStatus> {
    entity
        .properties
        .get("status")
        .and_then(Value::as_str)
        .and_then(WorkflowStatus::parse)
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

/// The Action belonging to a given Task, if any -- used both to find
/// what a confirmed Task is waiting to run and, implicitly, whether it
/// already ran (via `status_of` on the result).
pub fn find_action_for_task(actions: &[Entity], task_id: Uuid) -> Option<&Entity> {
    actions
        .iter()
        .find(|action| task_id_of(action) == Some(task_id))
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

/// Change 3's classifier: does this intent select the dangerous
/// `delete_entity` Action? Returns the target entity id if so. Only an
/// intent explicitly carrying both `action_kind: "delete_entity"` and a
/// well-formed `target_entity_id` property takes this path -- anything
/// else (in particular, every intent the on-screen keyboard creates,
/// which only ever sets `text`) falls through to the ordinary Change 2
/// echo path.
pub fn dangerous_action_of(intent: &Entity) -> Option<Uuid> {
    let kind = intent.properties.get("action_kind")?.as_str()?;
    if kind != DELETE_ENTITY_ACTION_KIND {
        return None;
    }
    intent
        .properties
        .get("target_entity_id")?
        .as_str()?
        .parse()
        .ok()
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

    fn intent_with(properties: Map<String, Value>) -> Entity {
        entity(INTENT_TYPE, properties)
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
    fn change_3_confirmation_transitions_are_accepted() {
        use WorkflowStatus::*;
        assert!(valid_transition(Pending, WaitingConfirmation));
        assert!(valid_transition(WaitingConfirmation, Running));
        assert!(valid_transition(WaitingConfirmation, Cancelled));
    }

    #[test]
    fn a_resumed_task_can_never_skip_straight_to_done() {
        use WorkflowStatus::*;
        assert!(!valid_transition(WaitingConfirmation, Done));
    }

    #[test]
    fn transitions_outside_scope_are_rejected() {
        use WorkflowStatus::*;
        assert!(!valid_transition(Done, Running));
        assert!(!valid_transition(Cancelled, Done));
        assert!(!valid_transition(Cancelled, Running));
        assert!(!valid_transition(Pending, Done));
        assert!(!valid_transition(Failed, Done));
        assert!(!valid_transition(Running, WaitingConfirmation));
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
    fn task_id_round_trips_through_action_properties() {
        let task_id = Uuid::new_v4();
        let action = entity(
            ACTION_TYPE,
            action_properties(
                task_id,
                ECHO_ACTION_KIND,
                WorkflowStatus::Running,
                &json!({}),
                None,
            ),
        );
        assert_eq!(task_id_of(&action), Some(task_id));
    }

    #[test]
    fn status_of_reads_back_what_was_written() {
        let task = entity(
            TASK_TYPE,
            task_properties(Uuid::new_v4(), WorkflowStatus::WaitingConfirmation),
        );
        assert_eq!(status_of(&task), Some(WorkflowStatus::WaitingConfirmation));
    }

    #[test]
    fn status_of_is_none_without_the_property() {
        assert_eq!(status_of(&entity(TASK_TYPE, Map::new())), None);
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
    fn find_action_for_task_matches_by_task_id() {
        let task_id = Uuid::new_v4();
        let action = entity(
            ACTION_TYPE,
            action_properties(
                task_id,
                DELETE_ENTITY_ACTION_KIND,
                WorkflowStatus::WaitingConfirmation,
                &json!({}),
                None,
            ),
        );
        let actions = vec![action.clone()];
        assert_eq!(find_action_for_task(&actions, task_id), Some(&action));
    }

    #[test]
    fn find_action_for_task_is_none_when_no_action_matches() {
        let actions = vec![entity(
            ACTION_TYPE,
            action_properties(
                Uuid::new_v4(),
                ECHO_ACTION_KIND,
                WorkflowStatus::Done,
                &json!({}),
                None,
            ),
        )];
        assert_eq!(find_action_for_task(&actions, Uuid::new_v4()), None);
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

    #[test]
    fn dangerous_action_of_recognizes_a_well_formed_delete_intent() {
        let target = Uuid::new_v4();
        let mut properties = Map::new();
        properties.insert("action_kind".into(), json!(DELETE_ENTITY_ACTION_KIND));
        properties.insert("target_entity_id".into(), json!(target.to_string()));
        assert_eq!(dangerous_action_of(&intent_with(properties)), Some(target));
    }

    #[test]
    fn dangerous_action_of_is_none_for_plain_text_intents() {
        let mut properties = Map::new();
        properties.insert("text".into(), json!("hello"));
        assert_eq!(dangerous_action_of(&intent_with(properties)), None);
    }

    #[test]
    fn dangerous_action_of_is_none_for_an_unrecognized_kind() {
        let mut properties = Map::new();
        properties.insert("action_kind".into(), json!("format_disk"));
        properties.insert("target_entity_id".into(), json!(Uuid::new_v4().to_string()));
        assert_eq!(dangerous_action_of(&intent_with(properties)), None);
    }

    #[test]
    fn dangerous_action_of_is_none_when_target_is_missing() {
        let mut properties = Map::new();
        properties.insert("action_kind".into(), json!(DELETE_ENTITY_ACTION_KIND));
        assert_eq!(dangerous_action_of(&intent_with(properties)), None);
    }

    #[test]
    fn dangerous_action_of_is_none_when_target_is_not_a_uuid() {
        let mut properties = Map::new();
        properties.insert("action_kind".into(), json!(DELETE_ENTITY_ACTION_KIND));
        properties.insert("target_entity_id".into(), json!("not-a-uuid"));
        assert_eq!(dangerous_action_of(&intent_with(properties)), None);
    }
}
