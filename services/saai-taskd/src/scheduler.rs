//! ADR-121 WORK-02: Ready is a derived view over the workflow store.
//! ADR-504 WORK-04: read-only Tasks may share a separate bounded frontier.
//!
//! Never persist `ready` as a Task status. Default mutating concurrency is 1.
//! WaitingConfirmation is not ready and is never auto-admitted. After
//! reboot, call the same functions on `list_entities` — there is no
//! ready file to replay.

use crate::model::{
    dependencies_satisfied, depends_on_of, intent_id_of, status_of, WorkflowStatus,
    PROPOSAL_ID_PROPERTY, TASK_TYPE,
};
use saai_entity_protocol::Entity;
use serde_json::Value;
use uuid::Uuid;

pub const MAX_MUTATING_IN_FLIGHT: usize = 1;
/// WORK-04: read-only Tasks do not share the mutating slot. Bound is
/// host-measured; keep small until WORK-09 budgets land on Pixel.
pub const MAX_READ_ONLY_IN_FLIGHT: usize = 2;
pub const READ_ONLY_PROPERTY: &str = "read_only";

pub fn is_completed(status: WorkflowStatus) -> bool {
    matches!(status, WorkflowStatus::Done)
}

pub fn is_in_flight(status: WorkflowStatus) -> bool {
    matches!(status, WorkflowStatus::Running | WorkflowStatus::Verifying)
}

pub fn is_mutating_in_flight(status: WorkflowStatus) -> bool {
    is_in_flight(status)
}

pub fn is_read_only(task: &Entity) -> bool {
    task.properties
        .get(READ_ONLY_PROPERTY)
        .and_then(Value::as_bool)
        == Some(true)
}

pub fn mutating_in_flight(tasks: &[Entity]) -> usize {
    tasks
        .iter()
        .filter(|task| {
            task.entity_type == TASK_TYPE
                && !is_read_only(task)
                && status_of(task).is_some_and(is_in_flight)
        })
        .count()
}

pub fn read_only_in_flight(tasks: &[Entity]) -> usize {
    tasks
        .iter()
        .filter(|task| {
            task.entity_type == TASK_TYPE
                && is_read_only(task)
                && status_of(task).is_some_and(is_in_flight)
        })
        .count()
}

pub fn completed_ids(tasks: &[Entity]) -> Vec<Uuid> {
    tasks
        .iter()
        .filter(|task| task.entity_type == TASK_TYPE && status_of(task).is_some_and(is_completed))
        .map(|task| task.id)
        .collect()
}

/// Pending tasks whose hard parents are Done. Failed parents are omitted
/// from the completed set, so the child stays blocked. Confirmation and
/// clarification waits are not ready.
pub fn derive_ready_set(tasks: &[Entity]) -> Vec<Uuid> {
    let completed = completed_ids(tasks);
    let mut ready: Vec<Uuid> = tasks
        .iter()
        .filter(|task| {
            task.entity_type == TASK_TYPE
                && status_of(task) == Some(WorkflowStatus::Pending)
                && dependencies_satisfied(&depends_on_of(task), &completed)
                && !plan_confirmation_blocks(task, tasks)
        })
        .map(|task| task.id)
        .collect();
    ready.sort();
    ready
}

fn is_plan_step(task: &Entity) -> bool {
    task.properties
        .get(PROPOSAL_ID_PROPERTY)
        .and_then(Value::as_str)
        .is_some()
}

/// A plan does not bulk-allow: while any step of this Intent waits for
/// confirmation, sibling plan steps stay out of the ready set.
fn plan_confirmation_blocks(task: &Entity, tasks: &[Entity]) -> bool {
    if !is_plan_step(task) {
        return false;
    }
    let Some(intent_id) = intent_id_of(task) else {
        return false;
    };
    tasks.iter().any(|other| {
        intent_id_of(other) == Some(intent_id)
            && status_of(other) == Some(WorkflowStatus::WaitingConfirmation)
    })
}

/// In-memory frontier: at most `max - in_flight` ready ids. Does not
/// write a status. Live dispatch (ADR-237) starts the admitted Task.
pub fn admit_frontier(ready: &[Uuid], in_flight: usize, max: usize) -> Vec<Uuid> {
    let slots = max.saturating_sub(in_flight);
    ready.iter().copied().take(slots).collect()
}

fn task_by_id(tasks: &[Entity], id: Uuid) -> Option<&Entity> {
    tasks.iter().find(|task| task.id == id)
}

/// WORK-04: admit read-only and mutating frontiers independently.
/// Mutating stays capped at [`MAX_MUTATING_IN_FLIGHT`]. Read-only uses
/// [`MAX_READ_ONLY_IN_FLIGHT`]. Order: all admitted read-only ids first
/// (stable ready order), then mutating.
pub fn admit_ready(tasks: &[Entity]) -> Vec<Uuid> {
    let ready = derive_ready_set(tasks);
    let mut read_only_ready = Vec::new();
    let mut mutating_ready = Vec::new();
    for id in ready {
        match task_by_id(tasks, id) {
            Some(task) if is_read_only(task) => read_only_ready.push(id),
            Some(_) => mutating_ready.push(id),
            None => {}
        }
    }
    let mut admitted = admit_frontier(
        &read_only_ready,
        read_only_in_flight(tasks),
        MAX_READ_ONLY_IN_FLIGHT,
    );
    admitted.extend(admit_frontier(
        &mutating_ready,
        mutating_in_flight(tasks),
        MAX_MUTATING_IN_FLIGHT,
    ));
    admitted
}

pub fn next_admission(tasks: &[Entity]) -> Option<Uuid> {
    admit_ready(tasks).into_iter().next()
}

/// Host measure for WORK-04 before raising bounds (WORK-09 on Pixel).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParallelismMeasure {
    pub ready_read_only: usize,
    pub ready_mutating: usize,
    pub read_only_in_flight: usize,
    pub mutating_in_flight: usize,
    pub read_only_slots_free: usize,
    pub mutating_slots_free: usize,
}

pub fn measure_parallelism(tasks: &[Entity]) -> ParallelismMeasure {
    let ready = derive_ready_set(tasks);
    let ready_read_only = ready
        .iter()
        .filter(|id| task_by_id(tasks, **id).is_some_and(is_read_only))
        .count();
    let ready_mutating = ready.len().saturating_sub(ready_read_only);
    let ro_flight = read_only_in_flight(tasks);
    let mut_flight = mutating_in_flight(tasks);
    ParallelismMeasure {
        ready_read_only,
        ready_mutating,
        read_only_in_flight: ro_flight,
        mutating_in_flight: mut_flight,
        read_only_slots_free: MAX_READ_ONLY_IN_FLIGHT.saturating_sub(ro_flight),
        mutating_slots_free: MAX_MUTATING_IN_FLIGHT.saturating_sub(mut_flight),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{task_properties, with_depends_on};
    use chrono::Utc;
    use saai_entity_protocol::Entity;
    use serde_json::json;

    fn task(status: WorkflowStatus, deps: &[Uuid]) -> Entity {
        let now = Utc::now();
        Entity {
            schema: 1,
            id: Uuid::new_v4(),
            space_id: "home".into(),
            entity_type: TASK_TYPE.into(),
            title: "test".into(),
            properties: with_depends_on(task_properties(Uuid::new_v4(), status), deps),
            revision: 1,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn ready_excludes_waiting_confirmation() {
        let waiting = task(WorkflowStatus::WaitingConfirmation, &[]);
        assert!(derive_ready_set(&[waiting]).is_empty());
    }

    #[test]
    fn ready_excludes_waiting_clarification() {
        let waiting = task(WorkflowStatus::WaitingClarification, &[]);
        assert!(derive_ready_set(&[waiting]).is_empty());
    }

    #[test]
    fn child_is_not_ready_until_parent_is_done() {
        let parent = task(WorkflowStatus::Pending, &[]);
        let child = task(WorkflowStatus::Pending, &[parent.id]);
        let snapshot = vec![parent.clone(), child.clone()];
        assert_eq!(derive_ready_set(&snapshot), vec![parent.id]);

        let mut done_parent = parent.clone();
        done_parent.properties = task_properties(
            crate::model::intent_id_of(&parent).unwrap_or(Uuid::nil()),
            WorkflowStatus::Done,
        );
        done_parent.id = parent.id;
        let after = vec![done_parent, child.clone()];
        assert_eq!(derive_ready_set(&after), vec![child.id]);
    }

    #[test]
    fn failed_parent_blocks_child() {
        let parent = task(WorkflowStatus::Failed, &[]);
        let child = task(WorkflowStatus::Pending, &[parent.id]);
        assert!(derive_ready_set(&[parent, child]).is_empty());
    }

    #[test]
    fn join_waits_for_every_parent() {
        let a = task(WorkflowStatus::Done, &[]);
        let b = task(WorkflowStatus::Pending, &[]);
        let child = task(WorkflowStatus::Pending, &[a.id, b.id]);
        let ready = derive_ready_set(&[a.clone(), b.clone(), child.clone()]);
        assert_eq!(ready, vec![b.id]);
        assert!(!ready.contains(&child.id));
        let mut b_done = b.clone();
        b_done.properties = task_properties(Uuid::nil(), WorkflowStatus::Done);
        b_done.id = b.id;
        assert_eq!(
            derive_ready_set(&[a, b_done, child.clone()]),
            vec![child.id]
        );
    }

    #[test]
    fn concurrency_one_admits_nothing_while_mutating_runs() {
        let running = task(WorkflowStatus::Running, &[]);
        let pending = task(WorkflowStatus::Pending, &[]);
        let ready = derive_ready_set(&[running.clone(), pending.clone()]);
        assert_eq!(ready, vec![pending.id]);
        assert_eq!(mutating_in_flight(&[running.clone(), pending.clone()]), 1);
        assert!(admit_frontier(
            &ready,
            mutating_in_flight(&[running, pending]),
            MAX_MUTATING_IN_FLIGHT
        )
        .is_empty());
    }

    #[test]
    fn concurrency_one_admits_single_ready_when_idle() {
        let a = task(WorkflowStatus::Pending, &[]);
        let b = task(WorkflowStatus::Pending, &[]);
        let ready = derive_ready_set(&[a.clone(), b.clone()]);
        assert_eq!(ready.len(), 2);
        let admitted = admit_frontier(&ready, 0, MAX_MUTATING_IN_FLIGHT);
        assert_eq!(admitted.len(), 1);
        assert_eq!(admitted[0], ready[0]);
        assert_eq!(next_admission(&[a, b]), Some(admitted[0]));
    }

    #[test]
    fn next_admission_is_none_while_mutating_runs() {
        let running = task(WorkflowStatus::Running, &[]);
        let pending = task(WorkflowStatus::Pending, &[]);
        assert_eq!(next_admission(&[running, pending]), None);
    }

    #[test]
    fn verifying_parent_blocks_child() {
        let parent = task(WorkflowStatus::Verifying, &[]);
        let child = task(WorkflowStatus::Pending, &[parent.id]);
        assert_eq!(
            derive_ready_set(&[parent.clone(), child.clone()]),
            Vec::<Uuid>::new()
        );
        assert_eq!(mutating_in_flight(&[parent.clone()]), 1);
        assert_eq!(next_admission(&[parent, child]), None);
    }

    #[test]
    fn verifying_counts_as_in_flight() {
        let verifying = task(WorkflowStatus::Verifying, &[]);
        let pending = task(WorkflowStatus::Pending, &[]);
        assert_eq!(mutating_in_flight(&[verifying.clone(), pending.clone()]), 1);
        assert_eq!(next_admission(&[verifying, pending]), None);
    }

    #[test]
    fn next_admission_unblocks_child_after_parent_done() {
        let parent = task(WorkflowStatus::Done, &[]);
        let child = task(WorkflowStatus::Pending, &[parent.id]);
        assert_eq!(next_admission(&[parent, child.clone()]), Some(child.id));
    }

    #[test]
    fn plan_does_not_admit_sibling_while_confirming() {
        let intent = Uuid::new_v4();
        let mut waiting = task(WorkflowStatus::WaitingConfirmation, &[]);
        waiting
            .properties
            .insert("intent_id".into(), json!(intent.to_string()));
        waiting
            .properties
            .insert(PROPOSAL_ID_PROPERTY.into(), json!("a"));
        let mut pending = task(WorkflowStatus::Pending, &[]);
        pending
            .properties
            .insert("intent_id".into(), json!(intent.to_string()));
        pending
            .properties
            .insert(PROPOSAL_ID_PROPERTY.into(), json!("b"));
        assert_eq!(next_admission(&[waiting, pending]), None);
    }

    #[test]
    fn rebuild_from_store_snapshot_matches_live_derivation() {
        let parent = task(WorkflowStatus::Done, &[]);
        let child = task(WorkflowStatus::Pending, &[parent.id]);
        let live = derive_ready_set(&[parent.clone(), child.clone()]);
        // Reboot: same entities, no extra ready file.
        let rebuilt = derive_ready_set(&[parent, child]);
        assert_eq!(live, rebuilt);
    }

    #[test]
    fn ready_is_not_a_persisted_status() {
        assert!(WorkflowStatus::parse("ready").is_none());
    }

    fn read_only_task(status: WorkflowStatus) -> Entity {
        let mut entity = task(status, &[]);
        entity
            .properties
            .insert(READ_ONLY_PROPERTY.into(), json!(true));
        entity
    }

    #[test]
    fn work04_two_read_only_admit_while_mutating_idle() {
        let a = read_only_task(WorkflowStatus::Pending);
        let b = read_only_task(WorkflowStatus::Pending);
        let admitted = admit_ready(&[a.clone(), b.clone()]);
        assert_eq!(admitted.len(), 2);
        assert!(admitted.contains(&a.id));
        assert!(admitted.contains(&b.id));
        let measure = measure_parallelism(&[a, b]);
        assert_eq!(measure.ready_read_only, 2);
        assert_eq!(measure.read_only_slots_free, 2);
        assert_eq!(measure.mutating_slots_free, 1);
    }

    #[test]
    fn work04_read_only_admits_beside_mutating_in_flight() {
        let running = task(WorkflowStatus::Running, &[]);
        let pending_ro = read_only_task(WorkflowStatus::Pending);
        let pending_mut = task(WorkflowStatus::Pending, &[]);
        assert_eq!(mutating_in_flight(&[running.clone(), pending_ro.clone()]), 1);
        assert_eq!(
            admit_ready(&[running.clone(), pending_ro.clone()]),
            vec![pending_ro.id]
        );
        assert_eq!(
            admit_ready(&[running, pending_ro.clone(), pending_mut]),
            vec![pending_ro.id]
        );
    }

    #[test]
    fn work04_read_only_bound_is_two() {
        let running = read_only_task(WorkflowStatus::Running);
        let a = read_only_task(WorkflowStatus::Pending);
        let b = read_only_task(WorkflowStatus::Pending);
        let c = read_only_task(WorkflowStatus::Pending);
        let admitted = admit_ready(&[running.clone(), a.clone(), b.clone(), c.clone()]);
        assert_eq!(admitted.len(), 1);
        assert_eq!(read_only_in_flight(&[running, a, b, c]), 1);
        assert_eq!(MAX_READ_ONLY_IN_FLIGHT, 2);
    }

    #[test]
    fn work04_third_read_only_waits_for_a_slot() {
        let a = read_only_task(WorkflowStatus::Running);
        let b = read_only_task(WorkflowStatus::Running);
        let c = read_only_task(WorkflowStatus::Pending);
        assert!(admit_ready(&[a, b, c]).is_empty());
    }
}
