//! Object Action Model types.
//!
//! Semantic actions over SOM objects. Risk, confirmation and schemas
//! stay on `ToolSpec`; this crate only says which action applies and
//! how to bind arguments.

use saai_entity_store::{Entity, ObjectRef};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;

pub const MAX_ACTION_ID_BYTES: usize = 96;
pub const MAX_PROVIDER_ID_BYTES: usize = 96;
pub const MAX_TOOL_NAME_BYTES: usize = 96;
pub const MAX_TITLE_CHARS: usize = 80;
pub const MAX_DESCRIPTION_CHARS: usize = 240;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ObjectActionError {
    #[error("invalid action id")]
    InvalidActionId,
    #[error("invalid provider id")]
    InvalidProviderId,
    #[error("invalid tool name")]
    InvalidToolName,
    #[error("invalid action title")]
    InvalidTitle,
    #[error("invalid action description")]
    InvalidDescription,
    #[error("object selector has no entity types")]
    EmptySelector,
    #[error("invalid entity type in selector")]
    InvalidSelectorType,
    #[error("malformed argument binding")]
    MalformedBinding,
    #[error("duplicate object action spec")]
    DuplicateSpec,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectSelector {
    pub entity_types: Vec<String>,
}

impl ObjectSelector {
    pub fn matches(&self, entity_type: &str) -> bool {
        self.entity_types
            .iter()
            .any(|candidate| candidate == entity_type)
    }

    fn canonical_types(&self) -> Vec<String> {
        let mut types = self.entity_types.clone();
        types.sort();
        types.dedup();
        types
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ArgumentSource {
    Constant(Value),
    ObjectId,
    ObjectTitle,
    ObjectProperty { name: String },
    ContextSpaceId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArgumentBinding {
    pub argument: String,
    pub source: ArgumentSource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectActionSpec {
    pub action_id: String,
    pub title: String,
    pub description: String,
    pub applies_to: ObjectSelector,
    pub tool_name: String,
    pub arguments: Vec<ArgumentBinding>,
    pub required_properties: Vec<String>,
    pub provider_id: String,
}

impl ObjectActionSpec {
    pub fn validate(&self) -> Result<(), ObjectActionError> {
        if !is_dotted_token(&self.action_id, MAX_ACTION_ID_BYTES) {
            return Err(ObjectActionError::InvalidActionId);
        }
        if !is_dotted_token(&self.provider_id, MAX_PROVIDER_ID_BYTES) {
            return Err(ObjectActionError::InvalidProviderId);
        }
        if !is_tool_name(&self.tool_name) {
            return Err(ObjectActionError::InvalidToolName);
        }
        if !is_human_text(&self.title, MAX_TITLE_CHARS) {
            return Err(ObjectActionError::InvalidTitle);
        }
        if !is_human_text(&self.description, MAX_DESCRIPTION_CHARS) {
            return Err(ObjectActionError::InvalidDescription);
        }
        if self.applies_to.entity_types.is_empty() {
            return Err(ObjectActionError::EmptySelector);
        }
        for entity_type in &self.applies_to.entity_types {
            if !is_dotted_token(entity_type, MAX_ACTION_ID_BYTES) {
                return Err(ObjectActionError::InvalidSelectorType);
            }
        }
        for required in &self.required_properties {
            if required.is_empty() || required.chars().any(char::is_control) {
                return Err(ObjectActionError::MalformedBinding);
            }
        }
        for binding in &self.arguments {
            if !is_argument_name(&binding.argument) {
                return Err(ObjectActionError::MalformedBinding);
            }
            if let ArgumentSource::ObjectProperty { name } = &binding.source {
                if name.is_empty() || name.chars().any(char::is_control) {
                    return Err(ObjectActionError::MalformedBinding);
                }
            }
        }
        Ok(())
    }

    pub fn identity_key(&self) -> (String, String, Vec<String>, String) {
        (
            self.action_id.clone(),
            self.provider_id.clone(),
            self.applies_to.canonical_types(),
            self.tool_name.clone(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionAvailability {
    Available,
    Unavailable { reason: String },
}

impl ActionAvailability {
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedAction {
    pub action_id: String,
    pub title: String,
    pub description: String,
    pub target: ObjectRef,
    pub provider_id: String,
    pub tool_name: String,
    pub arguments: Value,
    pub availability: ActionAvailability,
    pub object_revision: u64,
}

impl ResolvedAction {
    pub fn is_stale(&self, object: &Entity) -> bool {
        self.target != ObjectRef::entity(object.id) || self.object_revision != object.revision
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActionResolution {
    Resolved(ResolvedAction),
    Ambiguous {
        action_id: String,
        candidates: Vec<ResolvedAction>,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActionResolveContext {
    pub space_id: Option<String>,
}

pub fn bind_arguments(
    spec: &ObjectActionSpec,
    object: &Entity,
    context: &ActionResolveContext,
) -> Result<Value, String> {
    for required in &spec.required_properties {
        if !object.properties.contains_key(required) {
            return Err(format!("missing object property: {required}"));
        }
    }
    let mut arguments = Map::new();
    for binding in &spec.arguments {
        let value = match &binding.source {
            ArgumentSource::Constant(value) => value.clone(),
            ArgumentSource::ObjectId => Value::String(object.id.to_string()),
            ArgumentSource::ObjectTitle => Value::String(object.title.clone()),
            ArgumentSource::ObjectProperty { name } => object
                .properties
                .get(name)
                .cloned()
                .ok_or_else(|| format!("missing object property: {name}"))?,
            ArgumentSource::ContextSpaceId => context
                .space_id
                .as_ref()
                .map(|space_id| Value::String(space_id.clone()))
                .ok_or_else(|| "missing context space_id".to_string())?,
        };
        arguments.insert(binding.argument.clone(), value);
    }
    Ok(Value::Object(arguments))
}

pub fn display_inspect_spec() -> ObjectActionSpec {
    ObjectActionSpec {
        action_id: "display.inspect".into(),
        title: "Состояние экрана".into(),
        description: "Прочитать локальную идентичность устройства без побочных эффектов".into(),
        applies_to: ObjectSelector {
            entity_types: vec!["saaios.display".into()],
        },
        tool_name: "system.identity".into(),
        arguments: Vec::new(),
        required_properties: Vec::new(),
        provider_id: "saai.local-system".into(),
    }
}

fn is_dotted_token(value: &str, max_bytes: usize) -> bool {
    if value.is_empty() || value.len() > max_bytes || !value.is_ascii() {
        return false;
    }
    value.split('.').all(is_slug)
}

/// Existing ToolSpec names use underscores (`process.kill_request`)
/// and may be a single slug (`echo`). Action IDs stay dotted tokens.
fn is_tool_name(value: &str) -> bool {
    if value.is_empty() || value.len() > MAX_TOOL_NAME_BYTES || !value.is_ascii() {
        return false;
    }
    value.split('.').all(is_tool_slug)
}

fn is_slug(value: &str) -> bool {
    is_token_slug(value, false)
}

fn is_tool_slug(value: &str) -> bool {
    is_token_slug(value, true)
}

fn is_token_slug(value: &str, allow_underscore: bool) -> bool {
    let mut bytes = value.bytes();
    if !matches!(bytes.next(), Some(b'a'..=b'z')) {
        return false;
    }
    let mut previous_sep = false;
    for byte in bytes {
        match byte {
            b'a'..=b'z' | b'0'..=b'9' => previous_sep = false,
            b'-' if !previous_sep => previous_sep = true,
            b'_' if allow_underscore && !previous_sep => previous_sep = true,
            _ => return false,
        }
    }
    !previous_sep
}

fn is_human_text(value: &str, max_chars: usize) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty()
        && trimmed.chars().count() <= max_chars
        && !value.chars().any(char::is_control)
}

fn is_argument_name(value: &str) -> bool {
    !value.is_empty() && value.is_ascii() && !value.chars().any(char::is_control)
}
