//! WORK-07: bounded ReplanRequest. Retry ≠ Replan. The Scheduler asks
//! the Planner; the Supervisor does not rewrite the plan. Workers do
//! not spawn children.

use crate::model::{
    failure_class_of, has_open_task_for_intent, intent_id_of, FailureClass, INTENT_TYPE,
};
use saai_entity_protocol::Entity;
use uuid::Uuid;

/// One Planner pass per Intent after a verification mismatch.
pub const MAX_REPLANS_PER_INTENT: u32 = 1;
pub const REPLAN_COUNT_PROPERTY: &str = "replan_count";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplanRequest {
    pub intent_id: Uuid,
    pub failed_task_id: Uuid,
    pub class: FailureClass,
    pub attempt: u32,
}

pub fn replan_count_of(intent: &Entity) -> u32 {
    intent
        .properties
        .get(REPLAN_COUNT_PROPERTY)
        .and_then(|value| value.as_u64())
        .unwrap_or(0) as u32
}

/// Verification mismatch with room under the cap, and no in-flight
/// sibling, may ask the Planner once. Timeout stays retry (WORK-06).
pub fn should_issue_replan(
    task: &Entity,
    intent: &Entity,
    tasks: &[Entity],
) -> Option<ReplanRequest> {
    if intent.entity_type != INTENT_TYPE {
        return None;
    }
    let intent_id = intent_id_of(task)?;
    if intent.id != intent_id {
        return None;
    }
    if failure_class_of(task) != Some(FailureClass::VerificationMismatch) {
        return None;
    }
    if has_open_task_for_intent(tasks, intent_id) {
        return None;
    }
    let attempt = replan_count_of(intent);
    if attempt >= MAX_REPLANS_PER_INTENT {
        return None;
    }
    Some(ReplanRequest {
        intent_id,
        failed_task_id: task.id,
        class: FailureClass::VerificationMismatch,
        attempt: attempt + 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        task_properties, with_failure_class, WorkflowStatus, TASK_TYPE, VERIFICATION_KEY_PROPERTY,
    };
    use chrono::Utc;
    use serde_json::{json, Map, Value};
    use uuid::Uuid;

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

    fn mismatch_task(intent_id: Uuid) -> Entity {
        let mut properties = task_properties(intent_id, WorkflowStatus::Failed);
        properties.insert(VERIFICATION_KEY_PROPERTY.into(), json!("wifi.link"));
        properties = with_failure_class(properties, FailureClass::VerificationMismatch);
        entity(TASK_TYPE, properties)
    }

    #[test]
    fn mismatch_without_prior_replan_is_issued() {
        let intent_id = Uuid::new_v4();
        let task = mismatch_task(intent_id);
        let mut intent_props = Map::new();
        intent_props.insert("text".into(), json!("fix wifi"));
        let mut intent = entity(INTENT_TYPE, intent_props);
        intent.id = intent_id;
        let request = should_issue_replan(&task, &intent, &[task.clone()]).unwrap();
        assert_eq!(request.intent_id, intent_id);
        assert_eq!(request.failed_task_id, task.id);
        assert_eq!(request.attempt, 1);
        assert_eq!(request.class, FailureClass::VerificationMismatch);
    }

    #[test]
    fn second_mismatch_is_not_issued() {
        let intent_id = Uuid::new_v4();
        let task = mismatch_task(intent_id);
        let mut intent_props = Map::new();
        intent_props.insert(REPLAN_COUNT_PROPERTY.into(), json!(1));
        let mut intent = entity(INTENT_TYPE, intent_props);
        intent.id = intent_id;
        assert!(should_issue_replan(&task, &intent, &[task.clone()]).is_none());
    }

    #[test]
    fn timeout_is_retry_not_replan() {
        let intent_id = Uuid::new_v4();
        let mut properties = task_properties(intent_id, WorkflowStatus::Failed);
        properties = with_failure_class(properties, FailureClass::Timeout);
        let task = entity(TASK_TYPE, properties);
        let mut intent = entity(INTENT_TYPE, Map::new());
        intent.id = intent_id;
        assert!(should_issue_replan(&task, &intent, &[task.clone()]).is_none());
    }

    #[test]
    fn open_sibling_blocks_replan() {
        let intent_id = Uuid::new_v4();
        let failed = mismatch_task(intent_id);
        let pending = entity(
            TASK_TYPE,
            task_properties(intent_id, WorkflowStatus::Pending),
        );
        let mut intent = entity(INTENT_TYPE, Map::new());
        intent.id = intent_id;
        assert!(should_issue_replan(&failed, &intent, &[failed.clone(), pending]).is_none());
    }
}
