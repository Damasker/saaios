//! ADR-030: Intent/Task/Action/Result live as new `entity_type` values in
//! the existing `saai-entity-store` (S06), not as a port of Platform
//! Track's `policy-engine`/`tool-registry`. This module is the pure,
//! host-testable half of that decision: entity-type/property schema,
//! the workflow status state machine, idempotency, the dangerous
//! `delete_entity` Action (Change 3), and (S10 Change 2) the bridge
//! Action kind that lets a free-form Intent become a real, model-
//! authored proposal instead of a hardcoded transform. No IO here --
//! the wire clients live in `client.rs`/`runtime_bridge.rs`.

use chrono::{DateTime, Utc};
use saai_entity_protocol::Entity;
use saai_entity_store::MAX_ENTITY_TITLE_CHARS;
use serde_json::{json, Map, Value};
use uuid::Uuid;

pub const INTENT_TYPE: &str = "saaios.intent";
pub const TASK_TYPE: &str = "saaios.task";
pub const ACTION_TYPE: &str = "saaios.action";
pub const RESULT_TYPE: &str = "saaios.result";

/// S10 Change 3 (ADR-036): a schedule is its own native entity, not a
/// port of Platform Track's `automation-engine::TriggerKind` -- that
/// enum has no time-based variant at all, and its one existing "auto"
/// path bypasses `saai-entity-store` entirely (see the ADR). `every_secs`
/// is the simplest trigger shape that proves the mechanism; cron/at-style
/// triggers stay out of scope until something actually asks for them.
pub const SCHEDULE_TYPE: &str = "saaios.schedule";

/// Change 3's one dangerous Action: deletes another entity in the same
/// space. Irreversible from the live store's point of view (the
/// event log keeps history, but the entity itself is gone) and reaches
/// outside the Action's own arguments -- exactly the risk profile S09's
/// Threat/privacy impact section describes. Selected explicitly via an
/// intent's own `action_kind`/`target_entity_id` properties
/// (`dangerous_action_of`) -- free-form keyboard text never reaches
/// this path on its own.
pub const DELETE_ENTITY_ACTION_KIND: &str = "delete_entity";

/// S10 Change 2 (ADR-033): every free-form Intent that isn't the
/// explicit `delete_entity` path goes through the already-running
/// on-device `saaios-runtime` instead of a hardcoded transform --
/// Change 2 of S09 (`echo`) was always a placeholder to prove the
/// entity-store conveyor before a real model existed to ask; now that
/// one is physically proven reachable (ADR-033), there is no reason to
/// keep both. Whether this becomes `Done` immediately or
/// `WaitingConfirmation` is decided purely by whether the runtime's
/// response carries `pending` -- see `lib.rs`'s `process_planner_intent`.
pub const RUNTIME_ACTION_KIND: &str = "saaios_runtime_tool";

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

/// A Task now starts `Pending` whenever what it will become isn't known
/// yet -- true for every planner-bridged Intent (S10 Change 2), since
/// nothing here knows whether the model will answer outright or propose
/// something that needs confirmation until `saaios-runtime` actually
/// replies. From `Pending` it can reach `Running` (answered outright,
/// about to be finalized `Done`), `WaitingConfirmation` (the model
/// proposed something `saaios-runtime`'s own `policy-engine` flagged
/// `ask_user`), or `Failed` directly (the bridge couldn't even reach
/// `saaios-runtime` -- there was never anything to run). `Running` only
/// ever continues to `Done`/`Failed` for itself, matching Change 2 of
/// S09's original shape. `WaitingConfirmation` can only reach `Running`
/// or `Cancelled` through an explicit, separately authored update
/// (`saai-shell`'s confirmation screen, a live touch -- never something
/// this daemon writes to itself) -- there is deliberately no
/// `WaitingConfirmation -> Done` and no reboot/restart-only path to
/// `Running`, per S09's Threat/privacy impact requirement that a
/// resumed Task never auto-executes on a cached decision.
/// `WaitingConfirmation -> Failed` is the one exception this daemon
/// *does* write to a confirmed Action itself: the human already made
/// the "yes, run it" decision (the Task is `Running` by then, not
/// `WaitingConfirmation` any more) -- this transition belongs to the
/// still-`WaitingConfirmation` Action underneath it, when actually
/// resuming it fails (`saaios-runtime` unreachable, malformed
/// response), not to a cached auto-decision about whether to run at
/// all.
pub fn valid_transition(from: WorkflowStatus, to: WorkflowStatus) -> bool {
    use WorkflowStatus::*;
    matches!(
        (from, to),
        (Pending, Running)
            | (Running, Done)
            | (Running, Failed)
            | (Pending, WaitingConfirmation)
            | (Pending, Failed)
            | (WaitingConfirmation, Running)
            | (WaitingConfirmation, Cancelled)
            | (WaitingConfirmation, Failed)
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

/// Discovered the hard way (S10 Change 2 physical testing, before this
/// was committed): `saai-entity-store` requires a title that's
/// non-empty after trimming, at most `MAX_ENTITY_TITLE_CHARS`, and free
/// of control characters -- this daemon's own hand-written titles
/// (short, static strings, or a `short_id()`) always satisfied that by
/// construction, but a model's own summary text has no such guarantee
/// (a multi-line answer, or one long enough to exceed the limit, made
/// `saai-entityd` reject the `CreateEntity` call outright and crashed
/// this daemon before this existed). Every title built from intent text
/// or a model's own output goes through this first.
pub fn safe_title(text: &str, fallback: &str) -> String {
    let cleaned: String = text.chars().filter(|c| !c.is_control()).collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        return fallback.to_string();
    }
    if trimmed.chars().count() > MAX_ENTITY_TITLE_CHARS {
        trimmed.chars().take(MAX_ENTITY_TITLE_CHARS).collect()
    } else {
        trimmed.to_string()
    }
}

/// Change 3's classifier: does this intent select the dangerous
/// `delete_entity` Action? Returns the target entity id if so. Only an
/// intent explicitly carrying both `action_kind: "delete_entity"` and a
/// well-formed `target_entity_id` property takes this path -- anything
/// else (in particular, every intent the on-screen keyboard creates,
/// which only ever sets `text`) falls through to the planner bridge
/// (S10 Change 2).
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

/// Builds/rebuilds a schedule's full property set -- `update_entity`
/// replaces properties wholesale (same as every other entity in this
/// module), so re-firing a schedule always passes all five fields back,
/// not just the ones that changed.
pub fn schedule_properties(
    every_secs: u64,
    text: &str,
    enabled: bool,
    last_fired_at: Option<DateTime<Utc>>,
    fire_count: u64,
) -> Map<String, Value> {
    let mut map = Map::new();
    map.insert("every_secs".into(), json!(every_secs));
    map.insert("text".into(), json!(text));
    map.insert("enabled".into(), json!(enabled));
    map.insert(
        "last_fired_at".into(),
        match last_fired_at {
            Some(ts) => json!(ts.to_rfc3339()),
            None => Value::Null,
        },
    );
    map.insert("fire_count".into(), json!(fire_count));
    map
}

pub fn schedule_every_secs(entity: &Entity) -> Option<u64> {
    entity.properties.get("every_secs").and_then(Value::as_u64)
}

/// The text a due schedule turns into a new Intent's `text` property --
/// deliberately the exact same property name/shape the on-screen
/// keyboard already writes, so the schedule's Intent is indistinguishable
/// from one a human typed (ADR-036's central point: nothing downstream
/// of Intent creation changes for this Change).
pub fn schedule_text(entity: &Entity) -> Option<&str> {
    entity.properties.get("text").and_then(Value::as_str)
}

fn schedule_enabled(entity: &Entity) -> bool {
    entity
        .properties
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn schedule_last_fired_at(entity: &Entity) -> Option<DateTime<Utc>> {
    entity
        .properties
        .get("last_fired_at")?
        .as_str()?
        .parse()
        .ok()
}

pub fn schedule_fire_count(entity: &Entity) -> u64 {
    entity
        .properties
        .get("fire_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

/// Whether a schedule should fire right now. A schedule that's disabled,
/// missing `every_secs`, or not yet due is simply not due -- there is no
/// separate error/malformed state, unlike Task/Action's `Failed`, since
/// nothing has been attempted yet.
pub fn is_schedule_due(entity: &Entity, now: DateTime<Utc>) -> bool {
    if entity.entity_type != SCHEDULE_TYPE || !schedule_enabled(entity) {
        return false;
    }
    let Some(every_secs) = schedule_every_secs(entity) else {
        return false;
    };
    match schedule_last_fired_at(entity) {
        None => true,
        Some(last) => (now - last).num_seconds() >= every_secs as i64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration as ChronoDuration, Utc};
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
    fn s10_change_2_bridge_failure_transitions_are_accepted() {
        use WorkflowStatus::*;
        assert!(valid_transition(Pending, Failed));
        assert!(valid_transition(WaitingConfirmation, Failed));
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
                RUNTIME_ACTION_KIND,
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
                RUNTIME_ACTION_KIND,
                WorkflowStatus::Done,
                &json!({}),
                None,
            ),
        )];
        assert_eq!(find_action_for_task(&actions, Uuid::new_v4()), None);
    }

    #[test]
    fn safe_title_passes_short_clean_text_through_unchanged() {
        assert_eq!(safe_title("Задача: привет", "fallback"), "Задача: привет");
    }

    #[test]
    fn safe_title_falls_back_on_empty_or_whitespace_only_text() {
        assert_eq!(safe_title("", "fallback"), "fallback");
        assert_eq!(safe_title("   \t  ", "fallback"), "fallback");
    }

    #[test]
    fn safe_title_strips_control_characters_like_a_multiline_model_answer() {
        assert_eq!(
            safe_title("line one\nline two\r\n", "fallback"),
            "line oneline two"
        );
    }

    #[test]
    fn safe_title_truncates_by_chars_not_bytes_and_stays_under_the_limit() {
        // Multi-byte UTF-8 chars (2 bytes each in UTF-8) -- a byte-based
        // truncation at 160 would either panic mid-character or cut the
        // count in half; this must truncate at exactly 160 *characters*.
        let long = "п".repeat(200);
        let title = safe_title(&long, "fallback");
        assert_eq!(title.chars().count(), 160);
    }

    #[test]
    fn safe_title_leaves_text_exactly_at_the_limit_untouched() {
        let exact = "a".repeat(160);
        assert_eq!(safe_title(&exact, "fallback"), exact);
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

    fn schedule(properties: Map<String, Value>) -> Entity {
        entity(SCHEDULE_TYPE, properties)
    }

    #[test]
    fn schedule_properties_round_trip_through_their_own_readers() {
        let now = Utc::now();
        let props = schedule_properties(30, "напомни попить воды", true, Some(now), 3);
        let sched = schedule(props);
        assert_eq!(schedule_text(&sched), Some("напомни попить воды"));
        assert_eq!(schedule_fire_count(&sched), 3);
        assert!(schedule_enabled(&sched));
        // Round-tripped through RFC3339 text -- compare at second
        // precision, not exact `DateTime` equality.
        assert_eq!(
            schedule_last_fired_at(&sched).unwrap().timestamp(),
            now.timestamp()
        );
    }

    #[test]
    fn schedule_properties_store_null_for_never_fired() {
        let sched = schedule(schedule_properties(60, "hi", true, None, 0));
        assert_eq!(schedule_last_fired_at(&sched), None);
    }

    #[test]
    fn a_never_fired_enabled_schedule_is_due_immediately() {
        let sched = schedule(schedule_properties(3600, "hi", true, None, 0));
        assert!(is_schedule_due(&sched, Utc::now()));
    }

    #[test]
    fn a_schedule_fired_less_than_every_secs_ago_is_not_due() {
        let now = Utc::now();
        let sched = schedule(schedule_properties(3600, "hi", true, Some(now), 1));
        assert!(!is_schedule_due(&sched, now + ChronoDuration::seconds(10)));
    }

    #[test]
    fn a_schedule_fired_at_least_every_secs_ago_is_due_again() {
        let now = Utc::now();
        let sched = schedule(schedule_properties(60, "hi", true, Some(now), 1));
        assert!(is_schedule_due(&sched, now + ChronoDuration::seconds(60)));
        assert!(is_schedule_due(&sched, now + ChronoDuration::seconds(120)));
    }

    #[test]
    fn a_disabled_schedule_is_never_due() {
        let sched = schedule(schedule_properties(1, "hi", false, None, 0));
        assert!(!is_schedule_due(&sched, Utc::now()));
    }

    #[test]
    fn a_schedule_missing_every_secs_is_never_due() {
        let mut props = Map::new();
        props.insert("text".into(), json!("hi"));
        props.insert("enabled".into(), json!(true));
        let sched = schedule(props);
        assert!(!is_schedule_due(&sched, Utc::now()));
    }

    #[test]
    fn is_schedule_due_ignores_non_schedule_entities() {
        let mut props = schedule_properties(1, "hi", true, None, 0);
        // Same shape as a due schedule, but the wrong entity_type --
        // must never be picked up by a listing that also contains
        // Task/Action/Result/Intent entities in the same space.
        props.insert("every_secs".into(), json!(1));
        let not_a_schedule = entity(TASK_TYPE, props);
        assert!(!is_schedule_due(&not_a_schedule, Utc::now()));
    }
}
