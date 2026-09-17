//! ADR-121 WORK-02: Ready is a derived view over the workflow store.
//!
//! Never persist `ready` as a Task status. Default mutating concurrency is 1.
//! WaitingConfirmation is not ready and is never auto-admitted. After
//! reboot, call the same functions on `list_entities` — there is no
//! ready file to replay.

use crate::model::{dependencies_satisfied, depends_on_of, status_of, WorkflowStatus, TASK_TYPE};
use saai_entity_protocol::Entity;
use uuid::Uuid;

pub const MAX_MUTATING_IN_FLIGHT: usize = 1;

pub fn is_completed(status: WorkflowStatus) -> bool {
    matches!(status, WorkflowStatus::Done)
}

pub fn is_mutating_in_flight(status: WorkflowStatus) -> bool {
    matches!(status, WorkflowStatus::Running)
}

pub fn mutating_in_flight(tasks: &[Entity]) -> usize {
    tasks
        .iter()
        .filter(|task| {
            task.entity_type == TASK_TYPE && status_of(task).is_some_and(is_mutating_in_flight)
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
        })
        .map(|task| task.id)
        .collect();
    ready.sort();
    ready
}

/// In-memory frontier: at most `max - in_flight` ready ids. Does not
/// write a status and does not start work.
pub fn admit_frontier(ready: &[Uuid], in_flight: usize, max: usize) -> Vec<Uuid> {
    let slots = max.saturating_sub(in_flight);
    ready.iter().copied().take(slots).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{task_properties, with_depends_on};
    use chrono::Utc;
    use saai_entity_protocol::Entity;

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
}
