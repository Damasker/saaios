//! Versioned, UI-independent data model for SaaiOS spaces and entities.
//!
//! Persistence is added in the next S06 change. Keeping validation beside the
//! wire/storage types makes malformed identifiers and cross-space records
//! impossible to accept accidentally at either boundary.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;
use uuid::Uuid;

mod store;

pub use store::{BootstrapResult, EntityStore, StoreError};

pub const SCHEMA_VERSION: u32 = 1;
pub const BUILTIN_SPACE_IDS: [&str; 4] = ["home", "work", "personal", "saaios"];
pub const MAX_SPACE_ID_BYTES: usize = 48;
pub const MAX_SPACE_NAME_CHARS: usize = 64;
pub const MAX_ENTITY_TYPE_BYTES: usize = 96;
pub const MAX_ENTITY_TITLE_CHARS: usize = 160;
pub const MAX_ENTITY_PROPERTIES_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpaceKind {
    User,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Space {
    pub schema: u32,
    pub id: String,
    pub name: String,
    pub kind: SpaceKind,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Entity {
    pub schema: u32,
    pub id: Uuid,
    pub space_id: String,
    pub entity_type: String,
    pub title: String,
    #[serde(default)]
    pub properties: Map<String, Value>,
    pub revision: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum EventPayload {
    SpaceCreated { space: Space },
    EntityCreated { entity: Entity },
    EntityUpdated { entity: Entity },
    EntityDeleted { entity_id: Uuid, revision: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub schema: u32,
    pub id: Uuid,
    pub sequence: u64,
    pub space_id: String,
    pub timestamp: DateTime<Utc>,
    pub payload: EventPayload,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SelectionSource {
    Default,
    Legacy,
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SpaceSelection {
    pub schema: u32,
    pub space_id: String,
    pub source: SelectionSource,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ValidationError {
    #[error("unsupported {record} schema {actual}; expected {SCHEMA_VERSION}")]
    UnsupportedSchema { record: &'static str, actual: u32 },
    #[error("invalid space id")]
    InvalidSpaceId,
    #[error("invalid space name")]
    InvalidSpaceName,
    #[error("invalid entity id")]
    InvalidEntityId,
    #[error("invalid event id")]
    InvalidEventId,
    #[error("invalid entity type")]
    InvalidEntityType,
    #[error("invalid entity title")]
    InvalidEntityTitle,
    #[error("entity properties exceed {MAX_ENTITY_PROPERTIES_BYTES} bytes")]
    EntityPropertiesTooLarge,
    #[error("revision must be greater than zero")]
    InvalidRevision,
    #[error("event sequence must be greater than zero")]
    InvalidSequence,
    #[error("record belongs to a different space")]
    CrossSpaceRecord,
    #[error("updated_at precedes created_at")]
    InvalidTimestampOrder,
}

impl Space {
    pub fn validate(&self) -> Result<(), ValidationError> {
        validate_schema("space", self.schema)?;
        validate_space_id(&self.id)?;
        validate_human_text(&self.name, MAX_SPACE_NAME_CHARS)
            .map_err(|_| ValidationError::InvalidSpaceName)
    }
}

impl Entity {
    pub fn validate(&self) -> Result<(), ValidationError> {
        validate_schema("entity", self.schema)?;
        if self.id.is_nil() {
            return Err(ValidationError::InvalidEntityId);
        }
        validate_space_id(&self.space_id)?;
        if !is_dotted_token(&self.entity_type, MAX_ENTITY_TYPE_BYTES) {
            return Err(ValidationError::InvalidEntityType);
        }
        validate_human_text(&self.title, MAX_ENTITY_TITLE_CHARS)
            .map_err(|_| ValidationError::InvalidEntityTitle)?;
        if serde_json::to_vec(&self.properties)
            .map(|bytes| bytes.len() > MAX_ENTITY_PROPERTIES_BYTES)
            .unwrap_or(true)
        {
            return Err(ValidationError::EntityPropertiesTooLarge);
        }
        if self.revision == 0 {
            return Err(ValidationError::InvalidRevision);
        }
        if self.updated_at < self.created_at {
            return Err(ValidationError::InvalidTimestampOrder);
        }
        Ok(())
    }
}

impl Event {
    pub fn validate(&self) -> Result<(), ValidationError> {
        validate_schema("event", self.schema)?;
        if self.id.is_nil() {
            return Err(ValidationError::InvalidEventId);
        }
        if self.sequence == 0 {
            return Err(ValidationError::InvalidSequence);
        }
        validate_space_id(&self.space_id)?;
        match &self.payload {
            EventPayload::SpaceCreated { space } => {
                space.validate()?;
                ensure_same_space(&self.space_id, &space.id)
            }
            EventPayload::EntityCreated { entity } | EventPayload::EntityUpdated { entity } => {
                entity.validate()?;
                ensure_same_space(&self.space_id, &entity.space_id)
            }
            EventPayload::EntityDeleted {
                entity_id,
                revision,
            } => {
                if entity_id.is_nil() {
                    return Err(ValidationError::InvalidEntityId);
                }
                if *revision == 0 {
                    return Err(ValidationError::InvalidRevision);
                }
                Ok(())
            }
        }
    }
}

impl SpaceSelection {
    pub fn validate(&self) -> Result<(), ValidationError> {
        validate_schema("space_selection", self.schema)?;
        validate_space_id(&self.space_id)
    }
}

pub fn validate_space_id(value: &str) -> Result<(), ValidationError> {
    if value.is_empty() || value.len() > MAX_SPACE_ID_BYTES || !value.is_ascii() || !is_slug(value)
    {
        return Err(ValidationError::InvalidSpaceId);
    }
    Ok(())
}

fn validate_schema(record: &'static str, actual: u32) -> Result<(), ValidationError> {
    if actual != SCHEMA_VERSION {
        return Err(ValidationError::UnsupportedSchema { record, actual });
    }
    Ok(())
}

fn ensure_same_space(expected: &str, actual: &str) -> Result<(), ValidationError> {
    if expected == actual {
        Ok(())
    } else {
        Err(ValidationError::CrossSpaceRecord)
    }
}

fn validate_human_text(value: &str, max_chars: usize) -> Result<(), ()> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.chars().count() > max_chars
        || value.chars().any(char::is_control)
    {
        return Err(());
    }
    Ok(())
}

fn is_slug(value: &str) -> bool {
    let mut bytes = value.bytes();
    if !matches!(bytes.next(), Some(b'a'..=b'z')) {
        return false;
    }
    let mut previous_dash = false;
    for byte in bytes {
        match byte {
            b'a'..=b'z' | b'0'..=b'9' => previous_dash = false,
            b'-' if !previous_dash => previous_dash = true,
            _ => return false,
        }
    }
    !previous_dash
}

fn is_dotted_token(value: &str, max_bytes: usize) -> bool {
    if value.is_empty() || value.len() > max_bytes || !value.is_ascii() {
        return false;
    }
    value.split('.').all(is_slug)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use serde_json::json;

    fn now() -> DateTime<Utc> {
        Utc.timestamp_opt(1_788_912_000, 0).unwrap()
    }

    fn entity(space_id: &str) -> Entity {
        Entity {
            schema: SCHEMA_VERSION,
            id: Uuid::from_u128(1),
            space_id: space_id.into(),
            entity_type: "task.item".into(),
            title: "Проверить питание".into(),
            properties: Map::from_iter([("done".into(), json!(false))]),
            revision: 1,
            created_at: now(),
            updated_at: now(),
        }
    }

    #[test]
    fn builtin_spaces_and_valid_entity_are_accepted() {
        for id in BUILTIN_SPACE_IDS {
            validate_space_id(id).unwrap();
            entity(id).validate().unwrap();
        }
    }

    #[test]
    fn traversal_and_noncanonical_space_ids_are_rejected() {
        for invalid in [
            "../home",
            "/home",
            "Home",
            "home/notes",
            "home..work",
            "-home",
            "home-",
        ] {
            assert_eq!(
                validate_space_id(invalid),
                Err(ValidationError::InvalidSpaceId)
            );
        }
    }

    #[test]
    fn unknown_fields_are_rejected_during_decode() {
        let mut value = serde_json::to_value(entity("home")).unwrap();
        value["unexpected"] = json!(true);
        assert!(serde_json::from_value::<Entity>(value).is_err());
    }

    #[test]
    fn unknown_schema_is_rejected_after_decode() {
        let mut value = entity("home");
        value.schema = 2;
        assert!(matches!(
            value.validate(),
            Err(ValidationError::UnsupportedSchema {
                record: "entity",
                actual: 2
            })
        ));
    }

    #[test]
    fn properties_are_bounded_by_serialized_size() {
        let mut value = entity("home");
        value.properties.insert(
            "body".into(),
            Value::String("x".repeat(MAX_ENTITY_PROPERTIES_BYTES)),
        );
        assert_eq!(
            value.validate(),
            Err(ValidationError::EntityPropertiesTooLarge)
        );
    }

    #[test]
    fn event_cannot_embed_an_entity_from_another_space() {
        let event = Event {
            schema: SCHEMA_VERSION,
            id: Uuid::from_u128(2),
            sequence: 1,
            space_id: "home".into(),
            timestamp: now(),
            payload: EventPayload::EntityCreated {
                entity: entity("work"),
            },
        };
        assert_eq!(event.validate(), Err(ValidationError::CrossSpaceRecord));
    }

    #[test]
    fn unknown_event_kind_is_rejected() {
        let value = json!({
            "schema": 1,
            "id": Uuid::from_u128(3),
            "sequence": 1,
            "space_id": "home",
            "timestamp": now(),
            "payload": {"kind": "entity_moved", "data": {}}
        });
        assert!(serde_json::from_value::<Event>(value).is_err());
    }
}
