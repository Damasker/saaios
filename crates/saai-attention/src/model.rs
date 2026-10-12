//! Domain types. Presentation (color, badge count) lives in the shell.

use chrono::{DateTime, Utc};
use saai_entity_store::ObjectRef;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AttentionKey(pub String);

impl AttentionKey {
    pub fn task_confirmation(task_id: Uuid) -> Self {
        Self(format!("task:{task_id}:confirmation"))
    }

    pub fn notification(notification_id: Uuid) -> Self {
        Self(format!("notification:{notification_id}"))
    }

    pub fn health(component_id: &str) -> Self {
        Self(format!("health:{component_id}"))
    }

    pub fn suggested_action(object_id: Uuid, action_id: &str) -> Self {
        Self(format!("action:{object_id}:{action_id}"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AttentionSource {
    WorkflowTask { task_id: Uuid },
    Notification { notification_id: Uuid },
    Health { component_id: String },
    SuggestedAction { action_id: String, object_id: Uuid },
}

/// ATTN-07: at most one OAM action. The caller already filtered
/// capability + policy. This crate does not invent a title or a verb.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuggestedAction {
    pub action_id: String,
    pub object_id: Uuid,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuggestedActionCandidate {
    pub action_id: String,
    pub object_id: Uuid,
    pub title: String,
    pub permitted: bool,
}

/// Zero or many permitted candidates produce nothing.
pub fn one_suggested_action(candidates: &[SuggestedActionCandidate]) -> Option<SuggestedAction> {
    let permitted: Vec<&SuggestedActionCandidate> =
        candidates.iter().filter(|candidate| candidate.permitted).collect();
    if permitted.len() != 1 {
        return None;
    }
    let chosen = permitted[0];
    if chosen.action_id.is_empty() || chosen.title.is_empty() {
        return None;
    }
    Some(SuggestedAction {
        action_id: chosen.action_id.clone(),
        object_id: chosen.object_id,
        title: chosen.title.clone(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttentionPriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttentionActionability {
    Informational,
    Inspectable,
    ActionAvailable,
    RequiresDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttentionRelevance {
    Background,
    Global,
    RelatedContext,
    CurrentContext,
    CurrentObject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttentionSurfaces {
    pub now: bool,
    pub inbox: bool,
    pub orb: bool,
}

impl AttentionSurfaces {
    pub const ALL: Self = Self {
        now: true,
        inbox: true,
        orb: true,
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttentionItem {
    pub key: AttentionKey,
    pub source: AttentionSource,
    pub title: String,
    pub summary: Option<String>,
    pub priority: AttentionPriority,
    pub relevance: AttentionRelevance,
    pub actionability: AttentionActionability,
    pub surfaces: AttentionSurfaces,
    pub object: Option<ObjectRef>,
    pub occurred_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttentionProjection {
    pub generated_at: DateTime<Utc>,
    pub items: Vec<AttentionItem>,
}

/// Active ContextFrame for ATTN-05. Empty means no space judgment:
/// every item stays [`AttentionRelevance::Global`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AttentionContext {
    pub space_ids: Vec<String>,
    pub primary_space_id: Option<String>,
    pub object_id: Option<Uuid>,
}

impl AttentionContext {
    pub fn from_primary_space(space_id: impl Into<String>) -> Self {
        let space_id = space_id.into();
        Self {
            space_ids: vec![space_id.clone()],
            primary_space_id: Some(space_id),
            object_id: None,
        }
    }
}

impl AttentionProjection {
    pub fn inbox_items(&self) -> impl Iterator<Item = &AttentionItem> {
        self.items.iter().filter(|item| item.surfaces.inbox)
    }

    pub fn now_items(&self) -> impl Iterator<Item = &AttentionItem> {
        self.items.iter().filter(|item| {
            item.surfaces.now && item.relevance != AttentionRelevance::Background
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(title: &str, permitted: bool) -> SuggestedActionCandidate {
        SuggestedActionCandidate {
            action_id: format!("act.{title}"),
            object_id: Uuid::nil(),
            title: title.into(),
            permitted,
        }
    }

    #[test]
    fn one_permitted_candidate_is_kept() {
        let chosen = one_suggested_action(&[candidate("keep", true), candidate("deny", false)])
            .expect("one");
        assert_eq!(chosen.title, "keep");
        assert_eq!(chosen.action_id, "act.keep");
    }

    #[test]
    fn zero_or_two_permitted_candidates_invent_nothing() {
        assert!(one_suggested_action(&[]).is_none());
        assert!(one_suggested_action(&[candidate("a", false)]).is_none());
        assert!(one_suggested_action(&[candidate("a", true), candidate("b", true)]).is_none());
    }

    #[test]
    fn empty_title_or_id_is_not_a_suggestion() {
        let blank = SuggestedActionCandidate {
            action_id: String::new(),
            object_id: Uuid::nil(),
            title: "Состояние экрана".into(),
            permitted: true,
        };
        assert!(one_suggested_action(&[blank]).is_none());
    }
}
