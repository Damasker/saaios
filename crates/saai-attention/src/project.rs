//! Build a projection from loaded entities. No I/O.

use crate::model::{
    AttentionActionability, AttentionItem, AttentionKey, AttentionPriority, AttentionProjection,
    AttentionRelevance, AttentionSource, AttentionSurfaces,
};
use chrono::Utc;
use saai_entity_store::Entity;
use saai_observation::{HealthReport, HealthState};
use serde_json::Value;
use std::collections::HashSet;
use uuid::Uuid;

const TASK_TYPE: &str = "saaios.task";
const NOTIFICATION_TYPE: &str = "saaios.notification";
const WAITING_CONFIRMATION: &str = "waiting_confirmation";

pub fn project_from_entities(entities: &[Entity]) -> AttentionProjection {
    project_with_health(entities, None)
}

/// ATTN-06: one Health report. Healthy/Unknown are not attention.
/// Unhealthy lights Orb. Degraded is NOW-only. Not a dashboard.
pub fn project_with_health(
    entities: &[Entity],
    health: Option<&HealthReport>,
) -> AttentionProjection {
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    for entity in entities {
        if let Some(item) = from_waiting_confirmation_task(entity) {
            if seen.insert(item.key.clone()) {
                items.push(item);
            }
        }
    }
    for entity in entities {
        if let Some(item) = from_undismissed_notification(entity) {
            if seen.insert(item.key.clone()) {
                items.push(item);
            }
        }
    }
    if let Some(item) = health.and_then(from_health) {
        if seen.insert(item.key.clone()) {
            items.push(item);
        }
    }
    AttentionProjection {
        generated_at: Utc::now(),
        items,
    }
}

/// Same source identity order as `inbox_rows()` in `saai-shell`.
pub fn inbox_source_ids(entities: &[Entity]) -> Vec<(AttentionSource, Uuid)> {
    project_from_entities(entities)
        .inbox_items()
        .filter_map(|item| match &item.source {
            AttentionSource::WorkflowTask { task_id } => Some((item.source.clone(), *task_id)),
            AttentionSource::Notification { notification_id } => {
                Some((item.source.clone(), *notification_id))
            }
            AttentionSource::Health { .. } => None,
        })
        .collect()
}

pub fn has_orb_attention(projection: &AttentionProjection) -> bool {
    projection.items.iter().any(|item| item.surfaces.orb)
}

fn from_waiting_confirmation_task(entity: &Entity) -> Option<AttentionItem> {
    if entity.entity_type != TASK_TYPE {
        return None;
    }
    let status = entity.properties.get("status").and_then(Value::as_str)?;
    if status != WAITING_CONFIRMATION {
        return None;
    }
    Some(AttentionItem {
        key: AttentionKey::task_confirmation(entity.id),
        source: AttentionSource::WorkflowTask { task_id: entity.id },
        title: entity.title.clone(),
        summary: None,
        priority: AttentionPriority::High,
        relevance: AttentionRelevance::Global,
        actionability: AttentionActionability::RequiresDecision,
        surfaces: AttentionSurfaces::ALL,
        object: None,
        occurred_at: Some(entity.updated_at),
    })
}

fn from_undismissed_notification(entity: &Entity) -> Option<AttentionItem> {
    if entity.entity_type != NOTIFICATION_TYPE {
        return None;
    }
    let dismissed = entity
        .properties
        .get("dismissed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if dismissed {
        return None;
    }
    // Apps control title/body only. Priority is always Normal for notifications
    // in v1 — even if the payload contains a spoofed "priority" property.
    Some(AttentionItem {
        key: AttentionKey::notification(entity.id),
        source: AttentionSource::Notification {
            notification_id: entity.id,
        },
        title: entity.title.clone(),
        summary: entity
            .properties
            .get("body")
            .and_then(Value::as_str)
            .map(str::to_string),
        priority: AttentionPriority::Normal,
        relevance: AttentionRelevance::Global,
        actionability: AttentionActionability::Informational,
        surfaces: AttentionSurfaces::ALL,
        object: None,
        occurred_at: Some(entity.updated_at),
    })
}

fn from_health(report: &HealthReport) -> Option<AttentionItem> {
    let (priority, actionability, surfaces) = match report.state {
        HealthState::Healthy | HealthState::Unknown => return None,
        HealthState::Degraded => (
            AttentionPriority::Normal,
            AttentionActionability::Informational,
            AttentionSurfaces {
                now: true,
                inbox: false,
                orb: false,
            },
        ),
        HealthState::Unhealthy => (
            AttentionPriority::High,
            AttentionActionability::Inspectable,
            AttentionSurfaces {
                now: true,
                inbox: false,
                orb: true,
            },
        ),
    };
    Some(AttentionItem {
        key: AttentionKey::health(&report.component_id),
        source: AttentionSource::Health {
            component_id: report.component_id.clone(),
        },
        title: report.component_id.clone(),
        summary: Some(match report.state {
            HealthState::Degraded => "degraded".into(),
            HealthState::Unhealthy => "unhealthy".into(),
            HealthState::Healthy | HealthState::Unknown => unreachable!(),
        }),
        priority,
        relevance: AttentionRelevance::Global,
        actionability,
        surfaces,
        object: None,
        occurred_at: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use saai_entity_store::{Entity, SCHEMA_VERSION};
    use saai_observation::{HealthReport, HealthState};
    use serde_json::{json, Map};
    use uuid::Uuid;

    fn entity(entity_type: &str, title: &str, properties: Map<String, Value>) -> Entity {
        let now = Utc::now();
        Entity {
            schema: SCHEMA_VERSION,
            id: Uuid::new_v4(),
            space_id: "home".into(),
            entity_type: entity_type.into(),
            title: title.into(),
            properties,
            revision: 1,
            created_at: now,
            updated_at: now,
        }
    }

    fn task(status: &str) -> Entity {
        let mut properties = Map::new();
        properties.insert("status".into(), json!(status));
        entity(TASK_TYPE, "Confirm delete", properties)
    }

    fn notification(dismissed: bool, extra: Map<String, Value>) -> Entity {
        let mut properties = extra;
        properties.insert("dismissed".into(), json!(dismissed));
        properties
            .entry("body".to_string())
            .or_insert_with(|| json!("body"));
        entity(NOTIFICATION_TYPE, "Notice", properties)
    }

    /// Mirrors `inbox_rows` in saai-shell: waiting-confirmation tasks, then
    /// undismissed notifications, in input order.
    fn legacy_inbox_ids(entities: &[Entity]) -> Vec<Uuid> {
        let mut ids: Vec<Uuid> = entities
            .iter()
            .filter(|e| {
                e.entity_type == TASK_TYPE
                    && e.properties.get("status").and_then(Value::as_str)
                        == Some(WAITING_CONFIRMATION)
            })
            .map(|e| e.id)
            .collect();
        ids.extend(
            entities
                .iter()
                .filter(|e| {
                    e.entity_type == NOTIFICATION_TYPE
                        && !e
                            .properties
                            .get("dismissed")
                            .and_then(Value::as_bool)
                            .unwrap_or(false)
                })
                .map(|e| e.id),
        );
        ids
    }

    #[test]
    fn waiting_confirmation_task_creates_candidate() {
        let t = task(WAITING_CONFIRMATION);
        let proj = project_from_entities(std::slice::from_ref(&t));
        assert_eq!(proj.items.len(), 1);
        assert_eq!(
            proj.items[0].source,
            AttentionSource::WorkflowTask { task_id: t.id }
        );
        assert_eq!(proj.items[0].key, AttentionKey::task_confirmation(t.id));
        assert_eq!(
            proj.items[0].actionability,
            AttentionActionability::RequiresDecision
        );
        assert_eq!(proj.items[0].priority, AttentionPriority::High);
    }

    #[test]
    fn running_task_does_not_create_attention() {
        let proj = project_from_entities(&[task("running")]);
        assert!(proj.items.is_empty());
    }

    #[test]
    fn done_task_does_not_create_candidate() {
        let proj = project_from_entities(&[task("done")]);
        assert!(proj.items.is_empty());
    }

    #[test]
    fn undismissed_notification_creates_candidate() {
        let n = notification(false, Map::new());
        let proj = project_from_entities(std::slice::from_ref(&n));
        assert_eq!(proj.items.len(), 1);
        assert_eq!(
            proj.items[0].source,
            AttentionSource::Notification {
                notification_id: n.id
            }
        );
        assert_eq!(proj.items[0].key, AttentionKey::notification(n.id));
    }

    #[test]
    fn dismissed_notification_does_not_create_candidate() {
        let proj = project_from_entities(&[notification(true, Map::new())]);
        assert!(proj.items.is_empty());
    }

    #[test]
    fn third_party_notification_cannot_become_critical() {
        let mut extra = Map::new();
        extra.insert("priority".into(), json!("Critical"));
        extra.insert("kind".into(), json!("app:mahjong"));
        let n = notification(false, extra);
        let item = &project_from_entities(&[n]).items[0];
        assert_eq!(item.priority, AttentionPriority::Normal);
        assert_ne!(item.priority, AttentionPriority::Critical);
    }

    #[test]
    fn stable_keys_for_same_sources() {
        let t = task(WAITING_CONFIRMATION);
        let n = notification(false, Map::new());
        let a = project_from_entities(&[t.clone(), n.clone()]);
        let b = project_from_entities(&[t.clone(), n.clone()]);
        assert_eq!(a.items[0].key, b.items[0].key);
        assert_eq!(a.items[1].key, b.items[1].key);
    }

    #[test]
    fn projection_deduplicates_same_source() {
        let t = task(WAITING_CONFIRMATION);
        let proj = project_from_entities(&[t.clone(), t.clone()]);
        assert_eq!(proj.items.len(), 1);
    }

    #[test]
    fn inbox_parity_matches_legacy_inbox_rows() {
        let waiting = task(WAITING_CONFIRMATION);
        let running = task("running");
        let note = notification(false, Map::new());
        let gone = notification(true, Map::new());
        let entities = vec![running, waiting.clone(), gone, note.clone()];
        let legacy = legacy_inbox_ids(&entities);
        let projected: Vec<Uuid> = inbox_source_ids(&entities)
            .into_iter()
            .map(|(_, id)| id)
            .collect();
        assert_eq!(projected, legacy);
        assert_eq!(projected, vec![waiting.id, note.id]);
    }

    #[test]
    fn requires_decision_ranks_before_notification_in_inbox() {
        let n = notification(false, Map::new());
        let t = task(WAITING_CONFIRMATION);
        let ids: Vec<_> = inbox_source_ids(&[n.clone(), t.clone()])
            .into_iter()
            .map(|(_, id)| id)
            .collect();
        assert_eq!(ids, vec![t.id, n.id]);
        assert_eq!(
            project_from_entities(&[n, t]).items[0].actionability,
            AttentionActionability::RequiresDecision
        );
    }

    #[test]
    fn background_relevance_does_not_delete_item() {
        let mut item = project_from_entities(&[notification(false, Map::new())]).items;
        item[0].relevance = AttentionRelevance::Background;
        assert_eq!(item.len(), 1);
    }

    #[test]
    fn surface_filters() {
        let mut item = project_from_entities(&[task(WAITING_CONFIRMATION)])
            .items
            .remove(0);
        item.surfaces.now = false;
        item.surfaces.inbox = true;
        item.surfaces.orb = false;
        let proj = AttentionProjection {
            generated_at: Utc::now(),
            items: vec![item],
        };
        assert_eq!(proj.now_items().count(), 0);
        assert_eq!(proj.inbox_items().count(), 1);
        assert!(!has_orb_attention(&proj));
    }

    #[test]
    fn default_v1_items_are_on_now_inbox_and_orb() {
        let t = task(WAITING_CONFIRMATION);
        let proj = project_from_entities(&[t]);
        assert!(proj.items[0].surfaces.now);
        assert!(proj.items[0].surfaces.inbox);
        assert!(proj.items[0].surfaces.orb);
        assert!(has_orb_attention(&proj));
    }

    fn report(state: HealthState) -> HealthReport {
        HealthReport {
            component_id: "system.cpu.sampler".into(),
            state,
            observation_id: None,
            freshness: None,
        }
    }

    #[test]
    fn healthy_and_unknown_are_not_attention() {
        assert!(
            project_with_health(&[], Some(&report(HealthState::Healthy)))
                .items
                .is_empty()
        );
        assert!(
            project_with_health(&[], Some(&report(HealthState::Unknown)))
                .items
                .is_empty()
        );
    }

    #[test]
    fn degraded_is_now_only_unhealthy_lights_orb() {
        let degraded = project_with_health(&[], Some(&report(HealthState::Degraded)));
        assert_eq!(degraded.items.len(), 1);
        assert!(degraded.items[0].surfaces.now);
        assert!(!degraded.items[0].surfaces.inbox);
        assert!(!degraded.items[0].surfaces.orb);
        assert!(!has_orb_attention(&degraded));
        let unhealthy = project_with_health(&[], Some(&report(HealthState::Unhealthy)));
        assert!(unhealthy.items[0].surfaces.orb);
        assert!(has_orb_attention(&unhealthy));
        assert_eq!(
            unhealthy.items[0].key,
            AttentionKey::health("system.cpu.sampler")
        );
    }

    #[test]
    fn health_does_not_break_inbox_parity() {
        let waiting = task(WAITING_CONFIRMATION);
        let note = notification(false, Map::new());
        let entities = vec![waiting.clone(), note.clone()];
        let with_health = project_with_health(&entities, Some(&report(HealthState::Unhealthy)));
        let inbox: Vec<Uuid> = with_health
            .inbox_items()
            .filter_map(|item| match &item.source {
                AttentionSource::WorkflowTask { task_id } => Some(*task_id),
                AttentionSource::Notification { notification_id } => Some(*notification_id),
                AttentionSource::Health { .. } => None,
            })
            .collect();
        assert_eq!(inbox, vec![waiting.id, note.id]);
    }
}
