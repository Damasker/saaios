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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AttentionSource {
    WorkflowTask { task_id: Uuid },
    Notification { notification_id: Uuid },
    Health { component_id: String },
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

impl AttentionProjection {
    pub fn inbox_items(&self) -> impl Iterator<Item = &AttentionItem> {
        self.items.iter().filter(|item| item.surfaces.inbox)
    }

    pub fn now_items(&self) -> impl Iterator<Item = &AttentionItem> {
        self.items.iter().filter(|item| item.surfaces.now)
    }
}
