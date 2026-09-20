use saai_entity_store::{Entity, ObjectRef};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

pub const SOURCE_PROPERTY: &str = "source";
pub const CONTEXT_PROPERTY: &str = "context";
pub const SEMANTIC_ACTION_PROPERTY: &str = "semantic_action_id";
pub const TEXT_PROPERTY: &str = "text";
pub const PLAN_PROPERTY: &str = "plan";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentSource {
    Orb,
    ObjectView,
    Now,
    Space,
    Automation,
    Schedule,
    Worker,
    External,
}

impl IntentSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Orb => "orb",
            Self::ObjectView => "object_view",
            Self::Now => "now",
            Self::Space => "space",
            Self::Automation => "automation",
            Self::Schedule => "schedule",
            Self::Worker => "worker",
            Self::External => "external",
        }
    }
}

/// Compact object captured at Intent creation. Not a second ObjectRef.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectSummary {
    pub object: ObjectRef,
    pub entity_type: String,
    pub title: String,
    pub revision: u64,
}

impl ObjectSummary {
    pub fn from_entity(entity: &Entity) -> Self {
        Self {
            object: ObjectRef::entity(entity.id),
            entity_type: entity.entity_type.clone(),
            title: entity.title.clone(),
            revision: entity.revision,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ContextSnapshot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_space_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active_space_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focused_object: Option<ObjectSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_objects: Vec<ObjectSummary>,
}

impl ContextSnapshot {
    pub fn from_focus(primary_space_id: impl Into<String>, focused: Option<&Entity>) -> Self {
        let primary_space_id = primary_space_id.into();
        Self {
            active_space_ids: vec![primary_space_id.clone()],
            primary_space_id: Some(primary_space_id),
            focused_object: focused.map(ObjectSummary::from_entity),
            selected_objects: Vec::new(),
        }
    }

    pub fn focused_ref(&self) -> Option<&ObjectRef> {
        self.focused_object.as_ref().map(|summary| &summary.object)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntentInput {
    pub text: String,
    pub source: IntentSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explicit_action_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_object: Option<ObjectRef>,
    pub context: ContextSnapshot,
    /// Structured PlanProposal JSON (ADR-238). Absent on Direct/Confirm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<Value>,
}

impl IntentInput {
    pub fn from_entity(entity: &Entity) -> Self {
        let text = entity
            .properties
            .get(TEXT_PROPERTY)
            .and_then(Value::as_str)
            .unwrap_or(&entity.title)
            .to_string();
        let source = entity
            .properties
            .get(SOURCE_PROPERTY)
            .and_then(Value::as_str)
            .and_then(|raw| serde_json::from_value(Value::String(raw.into())).ok())
            .unwrap_or(IntentSource::Orb);
        let explicit_action_id = entity
            .properties
            .get(SEMANTIC_ACTION_PROPERTY)
            .and_then(Value::as_str)
            .map(str::to_string);
        let context: ContextSnapshot = entity
            .properties
            .get(CONTEXT_PROPERTY)
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default();
        let primary_object = entity
            .properties
            .get("target")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
            .or_else(|| context.focused_ref().cloned());
        let plan = entity.properties.get(PLAN_PROPERTY).cloned().filter(|value| {
            !value.is_null() && value.as_object().is_some()
        });
        Self {
            text,
            source,
            explicit_action_id,
            primary_object,
            context,
            plan,
        }
    }
}

pub fn intent_properties(
    text: &str,
    source: IntentSource,
    context: &ContextSnapshot,
    explicit_action_id: Option<&str>,
) -> Map<String, Value> {
    let mut properties = Map::new();
    properties.insert(TEXT_PROPERTY.into(), json!(text));
    properties.insert(SOURCE_PROPERTY.into(), json!(source.as_str()));
    if let Ok(value) = serde_json::to_value(context) {
        properties.insert(CONTEXT_PROPERTY.into(), value);
    }
    if let Some(action_id) = explicit_action_id {
        properties.insert(SEMANTIC_ACTION_PROPERTY.into(), json!(action_id));
    }
    properties
}
