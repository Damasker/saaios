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
/// ADR-089 (HIA-19's offline/degraded review): a Task that reaches
/// `Failed` used to do so silently -- visible only by reading the
/// entity store directly, since `saai-shell`'s "Входящие" only ever
/// listed `waiting_confirmation` Tasks. Same `entity_type`/`body`
/// shape `saai-shell`'s own `NOTIFICATION_ENTITY_TYPE` already uses
/// for `low_battery` -- reusing HIA-07's Object View (ADR-088)
/// instead of a new screen.
pub const NOTIFICATION_TYPE: &str = "saaios.notification";

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
/// IRAB-resolved semantic action. Not a raw runtime tool proposal.
pub const SEMANTIC_ACTION_KIND: &str = "saaios_semantic_action";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowStatus {
    Pending,
    Running,
    WaitingConfirmation,
    WaitingClarification,
    Verifying,
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
            Self::WaitingClarification => "waiting_clarification",
            Self::Verifying => "verifying",
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
            "waiting_clarification" => Some(Self::WaitingClarification),
            "verifying" => Some(Self::Verifying),
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
/// replies. From `Pending` it can reach `Running` (answered, Action
/// about to execute), `WaitingConfirmation` (the model proposed
/// something `saaios-runtime`'s own `policy-engine` flagged `ask_user`),
/// or `Failed` directly (the bridge couldn't even reach
/// `saaios-runtime` -- there was never anything to run). `Running` goes
/// to `Verifying` after an Action Result, or `Failed` if the worker
/// never produced a Result. `Done` is only from `Verifying` when a
/// Fresh Observation matches the contract (WORK-03 / ADR-259) -- never
/// from worker "ok". `WaitingConfirmation` can only reach `Running`
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
            | (Running, Verifying)
            | (Running, Failed)
            | (Verifying, Done)
            | (Verifying, Failed)
            | (Pending, WaitingConfirmation)
            | (Pending, WaitingClarification)
            | (Pending, Failed)
            | (WaitingConfirmation, Running)
            | (WaitingConfirmation, Cancelled)
            | (WaitingConfirmation, Failed)
            | (WaitingClarification, Pending)
            | (WaitingClarification, Running)
            | (WaitingClarification, Cancelled)
            | (WaitingClarification, Failed)
    )
}

/// Temporary property form of Task DAG edges (ADR-121 WORK-01).
/// Prefer SOM `saaios.depends-on` once that relation is the only format.
pub const DEPENDS_ON_PROPERTY: &str = "depends_on_task_ids";
/// Proposal id from a PlanProposal step (ADR-238). Distinct from Task UUID.
pub const PROPOSAL_ID_PROPERTY: &str = "proposal_id";
/// WORK-03: Observation key that must be Fresh and match before Done.
pub const VERIFICATION_KEY_PROPERTY: &str = "verification_key";
pub const VERIFICATION_EXPECTED_PROPERTY: &str = "verification_expected";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationEvidence {
    pub key: String,
    pub fresh: bool,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationDecision {
    Verified,
    StillVerifying,
    FailedMismatch,
}

/// Worker "ok" is never Done (ADR-121). Fresh matching Observation is.
pub fn decide_verification(
    expected_key: Option<&str>,
    expected_value: Option<&str>,
    evidence: Option<&ObservationEvidence>,
) -> VerificationDecision {
    let Some(key) = expected_key.filter(|key| !key.is_empty()) else {
        return VerificationDecision::StillVerifying;
    };
    let Some(sample) = evidence else {
        return VerificationDecision::StillVerifying;
    };
    if sample.key != key {
        return VerificationDecision::StillVerifying;
    }
    if !sample.fresh {
        return VerificationDecision::StillVerifying;
    }
    match expected_value {
        Some(expected) if sample.value != expected => VerificationDecision::FailedMismatch,
        _ => VerificationDecision::Verified,
    }
}

/// Status lists only Fresh rows (ADR-246). A listed key is therefore
/// Fresh evidence. JSON strings compare as the string; other values
/// use their JSON text.
pub fn observation_value_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

pub fn evidence_from_fresh_rows<'a, I>(key: &str, rows: I) -> Option<ObservationEvidence>
where
    I: IntoIterator<Item = (&'a str, &'a Value)>,
{
    rows.into_iter()
        .find(|(observed, _)| *observed == key)
        .map(|(_, value)| ObservationEvidence {
            key: key.to_string(),
            fresh: true,
            value: observation_value_text(value),
        })
}

pub fn verification_key_of(entity: &Entity) -> Option<&str> {
    entity
        .properties
        .get(VERIFICATION_KEY_PROPERTY)
        .and_then(Value::as_str)
        .filter(|key| !key.is_empty())
}

pub fn verification_expected_of(entity: &Entity) -> Option<&str> {
    entity
        .properties
        .get(VERIFICATION_EXPECTED_PROPERTY)
        .and_then(Value::as_str)
}

/// After Result, Task is Verifying until Fresh evidence matches.
pub fn status_after_verification(
    task: &Entity,
    evidence: Option<&ObservationEvidence>,
) -> WorkflowStatus {
    match decide_verification(
        verification_key_of(task),
        verification_expected_of(task),
        evidence,
    ) {
        VerificationDecision::Verified => WorkflowStatus::Done,
        VerificationDecision::FailedMismatch => WorkflowStatus::Failed,
        VerificationDecision::StillVerifying => WorkflowStatus::Verifying,
    }
}

pub fn task_properties_after_result(
    task: &Entity,
    intent_id: Uuid,
    result_id: Uuid,
    status: WorkflowStatus,
) -> Map<String, Value> {
    let mut properties = task_properties(intent_id, status);
    properties.insert("result_id".into(), json!(result_id.to_string()));
    if let Some(key) = verification_key_of(task) {
        properties.insert(VERIFICATION_KEY_PROPERTY.into(), json!(key));
    }
    if let Some(expected) = verification_expected_of(task) {
        properties.insert(VERIFICATION_EXPECTED_PROPERTY.into(), json!(expected));
    }
    if let Some(proposal) = task.properties.get(PROPOSAL_ID_PROPERTY).cloned() {
        properties.insert(PROPOSAL_ID_PROPERTY.into(), proposal);
    }
    with_depends_on(properties, &depends_on_of(task))
}

pub fn task_properties(intent_id: Uuid, status: WorkflowStatus) -> Map<String, Value> {
    let mut map = Map::new();
    map.insert("intent_id".into(), json!(intent_id.to_string()));
    map.insert("status".into(), json!(status.as_str()));
    map
}

pub fn with_depends_on(mut properties: Map<String, Value>, deps: &[Uuid]) -> Map<String, Value> {
    if !deps.is_empty() {
        properties.insert(
            DEPENDS_ON_PROPERTY.into(),
            json!(deps.iter().map(ToString::to_string).collect::<Vec<_>>()),
        );
    }
    properties
}

/// Hard dependencies only (WSV2 v1). Missing/unparseable ids are skipped.
pub fn depends_on_of(entity: &Entity) -> Vec<Uuid> {
    entity
        .properties
        .get(DEPENDS_ON_PROPERTY)
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .filter_map(|raw| raw.parse().ok())
                .collect()
        })
        .unwrap_or_default()
}

/// Ready derivation input: a Pending task is dependency-ready iff every
/// hard parent is in `completed` (Done, and Verified when that stage exists).
/// Failed parents are omitted from `completed`, so the child stays blocked.
pub fn dependencies_satisfied(depends_on: &[Uuid], completed: &[Uuid]) -> bool {
    depends_on.iter().all(|parent| completed.contains(parent))
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

pub fn result_id_of(entity: &Entity) -> Option<Uuid> {
    entity
        .properties
        .get("result_id")
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

fn is_open_workflow(status: WorkflowStatus) -> bool {
    matches!(
        status,
        WorkflowStatus::Pending
            | WorkflowStatus::Running
            | WorkflowStatus::WaitingConfirmation
            | WorkflowStatus::WaitingClarification
            | WorkflowStatus::Verifying
    )
}

/// A still-in-flight Task for this Intent. Failed/Done/Cancelled do not
/// count -- ADR-236 retry creates a sibling Task after a timeout.
pub fn has_open_task_for_intent(existing_tasks: &[Entity], intent_id: Uuid) -> bool {
    existing_tasks.iter().any(|task| {
        intent_id_of(task) == Some(intent_id) && status_of(task).is_some_and(is_open_workflow)
    })
}

/// Timeout/unreachable Failed Tasks are retryable. Malformed responses
/// are not. Object View later writes `retry_requested`; this daemon
/// never auto-retries mutating work (ADR-121).
pub fn is_retryable_failure(entity: &Entity) -> bool {
    status_of(entity) == Some(WorkflowStatus::Failed)
        && entity.properties.get("retryable").and_then(Value::as_bool) == Some(true)
}

pub fn retry_requested(entity: &Entity) -> bool {
    entity
        .properties
        .get("retry_requested")
        .and_then(Value::as_bool)
        == Some(true)
}

pub fn should_retry_failed_task(entity: &Entity) -> bool {
    is_retryable_failure(entity) && retry_requested(entity)
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
            WorkflowStatus::WaitingClarification,
            WorkflowStatus::Verifying,
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
        assert!(valid_transition(Running, Verifying));
        assert!(valid_transition(Running, Failed));
        assert!(valid_transition(Verifying, Done));
        assert!(valid_transition(Verifying, Failed));
        assert!(!valid_transition(Running, Done));
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
    fn clarification_is_distinct_from_confirmation() {
        use WorkflowStatus::*;
        assert!(valid_transition(Pending, WaitingClarification));
        assert!(valid_transition(WaitingClarification, Pending));
        assert!(valid_transition(WaitingClarification, Cancelled));
        assert!(valid_transition(WaitingClarification, Failed));
        assert!(!valid_transition(WaitingClarification, Done));
        assert!(!valid_transition(WaitingConfirmation, WaitingClarification));
        assert!(!valid_transition(WaitingClarification, WaitingConfirmation));
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
        assert!(!valid_transition(Verifying, Running));
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
    fn failed_task_is_not_an_open_task() {
        let intent_id = Uuid::new_v4();
        let tasks = vec![entity(
            TASK_TYPE,
            task_properties(intent_id, WorkflowStatus::Failed),
        )];
        assert!(has_task_for_intent(&tasks, intent_id));
        assert!(!has_open_task_for_intent(&tasks, intent_id));
    }

    #[test]
    fn pending_task_is_open() {
        let intent_id = Uuid::new_v4();
        let tasks = vec![entity(
            TASK_TYPE,
            task_properties(intent_id, WorkflowStatus::Pending),
        )];
        assert!(has_open_task_for_intent(&tasks, intent_id));
    }

    #[test]
    fn verifying_task_is_open() {
        let intent_id = Uuid::new_v4();
        let tasks = vec![entity(
            TASK_TYPE,
            task_properties(intent_id, WorkflowStatus::Verifying),
        )];
        assert!(has_open_task_for_intent(&tasks, intent_id));
    }

    #[test]
    fn timeout_failed_task_retries_only_when_requested() {
        let intent_id = Uuid::new_v4();
        let mut properties = task_properties(intent_id, WorkflowStatus::Failed);
        properties.insert("retryable".into(), json!(true));
        let failed = entity(TASK_TYPE, properties.clone());
        assert!(is_retryable_failure(&failed));
        assert!(!should_retry_failed_task(&failed));
        properties.insert("retry_requested".into(), json!(true));
        let requested = entity(TASK_TYPE, properties);
        assert!(should_retry_failed_task(&requested));
    }

    #[test]
    fn malformed_failure_is_not_retryable() {
        let task = entity(
            TASK_TYPE,
            task_properties(Uuid::new_v4(), WorkflowStatus::Failed),
        );
        assert!(!is_retryable_failure(&task));
        assert!(!should_retry_failed_task(&task));
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

    #[test]
    fn depends_on_round_trips_through_task_properties() {
        let parent = Uuid::new_v4();
        let child_props = with_depends_on(
            task_properties(Uuid::new_v4(), WorkflowStatus::Pending),
            &[parent],
        );
        let child = entity(TASK_TYPE, child_props);
        assert_eq!(depends_on_of(&child), vec![parent]);
    }

    #[test]
    fn ready_excludes_unsatisfied_dependency() {
        let parent = Uuid::new_v4();
        assert!(!dependencies_satisfied(&[parent], &[]));
    }

    #[test]
    fn ready_includes_satisfied_dependency() {
        let parent = Uuid::new_v4();
        assert!(dependencies_satisfied(&[parent], &[parent]));
    }

    #[test]
    fn join_waits_for_every_parent() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        assert!(!dependencies_satisfied(&[a, b], &[a]));
        assert!(dependencies_satisfied(&[a, b], &[a, b]));
    }

    #[test]
    fn failed_dependency_blocks_child() {
        let failed = Uuid::new_v4();
        let done = Uuid::new_v4();
        // Failed parent is omitted from the completed set.
        assert!(!dependencies_satisfied(&[failed, done], &[done]));
    }

    #[test]
    fn worker_ok_without_contract_is_not_done() {
        let task = entity(
            TASK_TYPE,
            task_properties(Uuid::new_v4(), WorkflowStatus::Running),
        );
        assert_eq!(
            decide_verification(None, None, None),
            VerificationDecision::StillVerifying
        );
        assert_eq!(
            status_after_verification(&task, None),
            WorkflowStatus::Verifying
        );
        assert!(!valid_transition(
            WorkflowStatus::Running,
            WorkflowStatus::Done
        ));
    }

    #[test]
    fn stale_observation_is_not_verified() {
        let mut properties = task_properties(Uuid::new_v4(), WorkflowStatus::Verifying);
        properties.insert(VERIFICATION_KEY_PROPERTY.into(), json!("system.metrics"));
        properties.insert(VERIFICATION_EXPECTED_PROPERTY.into(), json!("ok"));
        let task = entity(TASK_TYPE, properties);
        let stale = ObservationEvidence {
            key: "system.metrics".into(),
            fresh: false,
            value: "ok".into(),
        };
        assert_eq!(
            decide_verification(
                verification_key_of(&task),
                verification_expected_of(&task),
                Some(&stale)
            ),
            VerificationDecision::StillVerifying
        );
        assert_eq!(
            status_after_verification(&task, Some(&stale)),
            WorkflowStatus::Verifying
        );
    }

    #[test]
    fn missing_observation_stays_verifying() {
        let mut properties = task_properties(Uuid::new_v4(), WorkflowStatus::Verifying);
        properties.insert(VERIFICATION_KEY_PROPERTY.into(), json!("system.metrics"));
        let task = entity(TASK_TYPE, properties);
        assert_eq!(
            status_after_verification(&task, None),
            WorkflowStatus::Verifying
        );
    }

    #[test]
    fn fresh_matching_observation_is_done() {
        let mut properties = task_properties(Uuid::new_v4(), WorkflowStatus::Verifying);
        properties.insert(VERIFICATION_KEY_PROPERTY.into(), json!("system.metrics"));
        properties.insert(VERIFICATION_EXPECTED_PROPERTY.into(), json!("ok"));
        let task = entity(TASK_TYPE, properties);
        let fresh = ObservationEvidence {
            key: "system.metrics".into(),
            fresh: true,
            value: "ok".into(),
        };
        assert_eq!(
            status_after_verification(&task, Some(&fresh)),
            WorkflowStatus::Done
        );
        assert!(valid_transition(
            WorkflowStatus::Verifying,
            WorkflowStatus::Done
        ));
    }

    #[test]
    fn fresh_mismatch_is_failed() {
        let mut properties = task_properties(Uuid::new_v4(), WorkflowStatus::Verifying);
        properties.insert(VERIFICATION_KEY_PROPERTY.into(), json!("system.metrics"));
        properties.insert(VERIFICATION_EXPECTED_PROPERTY.into(), json!("ok"));
        let task = entity(TASK_TYPE, properties);
        let mismatch = ObservationEvidence {
            key: "system.metrics".into(),
            fresh: true,
            value: "wrong".into(),
        };
        assert_eq!(
            status_after_verification(&task, Some(&mismatch)),
            WorkflowStatus::Failed
        );
    }

    #[test]
    fn after_result_keeps_verification_contract() {
        let intent_id = Uuid::new_v4();
        let result_id = Uuid::new_v4();
        let mut properties = task_properties(intent_id, WorkflowStatus::Running);
        properties.insert(VERIFICATION_KEY_PROPERTY.into(), json!("wifi.link"));
        properties.insert(VERIFICATION_EXPECTED_PROPERTY.into(), json!("up"));
        let task = entity(TASK_TYPE, properties);
        let closed =
            task_properties_after_result(&task, intent_id, result_id, WorkflowStatus::Verifying);
        assert_eq!(
            closed
                .get(VERIFICATION_KEY_PROPERTY)
                .and_then(Value::as_str),
            Some("wifi.link")
        );
        assert_eq!(
            closed
                .get(VERIFICATION_EXPECTED_PROPERTY)
                .and_then(Value::as_str),
            Some("up")
        );
        assert_eq!(
            closed.get("result_id").and_then(Value::as_str),
            Some(result_id.to_string()).as_deref()
        );
    }

    #[test]
    fn status_fresh_row_is_live_evidence() {
        let value = json!("up");
        let rows = [("wifi.link", &value)];
        let evidence = evidence_from_fresh_rows("wifi.link", rows).expect("row");
        assert!(evidence.fresh);
        assert_eq!(evidence.value, "up");
        let mut properties = task_properties(Uuid::new_v4(), WorkflowStatus::Verifying);
        properties.insert(VERIFICATION_KEY_PROPERTY.into(), json!("wifi.link"));
        properties.insert(VERIFICATION_EXPECTED_PROPERTY.into(), json!("up"));
        let task = entity(TASK_TYPE, properties);
        assert_eq!(
            status_after_verification(&task, Some(&evidence)),
            WorkflowStatus::Done
        );
    }

    #[test]
    fn status_omits_stale_so_missing_row_stays_verifying() {
        let other = json!("ok");
        let rows = [("cpu.usage", &other)];
        assert!(evidence_from_fresh_rows("wifi.link", rows).is_none());
    }

    #[test]
    fn numeric_observation_compares_as_json_text() {
        let value = json!(40.0);
        let rows = [("cpu.usage", &value)];
        let evidence = evidence_from_fresh_rows("cpu.usage", rows).expect("row");
        assert_eq!(evidence.value, "40.0");
    }
}
