//! Saai Object Model relationship types.
//!
//! `Object` remains the domain name for `Entity`; this module adds the
//! first-class edge, not a second object database. Persistence lives in
//! `store`.

use crate::{
    is_dotted_token, validate_schema, validate_space_id, ValidationError,
    MAX_PROVENANCE_SOURCE_BYTES, MAX_RELATIONSHIP_PROPERTIES_BYTES, MAX_RELATION_TYPE_BYTES,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

pub const RELATION_IN_SPACE: &str = "saaios.in-space";
pub const RELATION_REALIZES: &str = "saaios.realizes";
pub const RELATION_EXECUTES: &str = "saaios.executes";
pub const RELATION_PRODUCES: &str = "saaios.produces";
pub const RELATION_RESULT_OF: &str = "saaios.result-of";

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObjectRef {
    Entity { id: Uuid },
    Space { id: String },
}

impl ObjectRef {
    pub fn entity(id: Uuid) -> Self {
        Self::Entity { id }
    }

    pub fn space(id: impl Into<String>) -> Self {
        Self::Space { id: id.into() }
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        match self {
            Self::Entity { id } if id.is_nil() => Err(ValidationError::InvalidObjectRef),
            Self::Entity { .. } => Ok(()),
            Self::Space { id } => validate_space_id(id),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Provenance {
    User,
    System,
    Import {
        source: String,
    },
    Worker {
        worker_id: String,
    },
    Model {
        provider: Option<String>,
        model: Option<String>,
    },
    Derived {
        rule: String,
    },
}

impl Provenance {
    pub fn kind_token(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::System => "system",
            Self::Import { .. } => "import",
            Self::Worker { .. } => "worker",
            Self::Model { .. } => "model",
            Self::Derived { .. } => "derived",
        }
    }

    pub fn is_inferred(&self) -> bool {
        matches!(
            self,
            Self::Model { .. } | Self::Worker { .. } | Self::Derived { .. }
        )
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        match self {
            Self::User | Self::System => Ok(()),
            Self::Import { source } => validate_provenance_label(source),
            Self::Worker { worker_id } => validate_provenance_label(worker_id),
            Self::Model { provider, model } => {
                if let Some(provider) = provider {
                    validate_provenance_label(provider)?;
                }
                if let Some(model) = model {
                    validate_provenance_label(model)?;
                }
                Ok(())
            }
            Self::Derived { rule } => validate_provenance_label(rule),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationDirection {
    From,
    To,
    Either,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Relationship {
    pub schema: u32,
    pub id: Uuid,
    pub source: ObjectRef,
    pub target: ObjectRef,
    pub relation_type: String,
    pub provenance: Provenance,
    pub confidence: Option<f32>,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
    #[serde(default)]
    pub properties: Map<String, Value>,
    pub revision: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Relationship {
    pub fn validate(&self) -> Result<(), ValidationError> {
        validate_schema("relationship", self.schema)?;
        if self.id.is_nil() {
            return Err(ValidationError::InvalidRelationshipId);
        }
        self.source.validate()?;
        self.target.validate()?;
        if !is_dotted_token(&self.relation_type, MAX_RELATION_TYPE_BYTES) {
            return Err(ValidationError::InvalidRelationType);
        }
        self.provenance.validate()?;
        match self.confidence {
            Some(value) if self.provenance.is_inferred() && (0.0..=1.0).contains(&value) => {}
            Some(_) => return Err(ValidationError::InvalidConfidence),
            None => {}
        }
        if let (Some(from), Some(until)) = (self.valid_from, self.valid_until) {
            if until < from {
                return Err(ValidationError::InvalidValidityWindow);
            }
        }
        if serde_json::to_vec(&self.properties)
            .map(|bytes| bytes.len() > MAX_RELATIONSHIP_PROPERTIES_BYTES)
            .unwrap_or(true)
        {
            return Err(ValidationError::RelationshipPropertiesTooLarge);
        }
        if self.revision == 0 {
            return Err(ValidationError::InvalidRevision);
        }
        if self.updated_at < self.created_at {
            return Err(ValidationError::InvalidTimestampOrder);
        }
        validate_known_relation_shape(&self.relation_type, &self.source, &self.target)
    }

    pub fn is_active_at(&self, now: DateTime<Utc>) -> bool {
        self.valid_from.is_none_or(|from| from <= now)
            && self.valid_until.is_none_or(|until| now < until)
    }

    pub fn user_confirmed(&self) -> bool {
        self.properties
            .get("user_confirmed")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RelationshipEventPayload {
    RelationshipCreated {
        relationship: Relationship,
    },
    RelationshipUpdated {
        relationship: Relationship,
    },
    RelationshipDeleted {
        relationship_id: Uuid,
        revision: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RelationshipEvent {
    pub schema: u32,
    pub id: Uuid,
    pub sequence: u64,
    pub timestamp: DateTime<Utc>,
    pub payload: RelationshipEventPayload,
}

impl RelationshipEvent {
    pub fn validate(&self) -> Result<(), ValidationError> {
        validate_schema("relationship_event", self.schema)?;
        if self.id.is_nil() {
            return Err(ValidationError::InvalidEventId);
        }
        if self.sequence == 0 {
            return Err(ValidationError::InvalidSequence);
        }
        match &self.payload {
            RelationshipEventPayload::RelationshipCreated { relationship }
            | RelationshipEventPayload::RelationshipUpdated { relationship } => {
                relationship.validate()
            }
            RelationshipEventPayload::RelationshipDeleted {
                relationship_id,
                revision,
            } => {
                if relationship_id.is_nil() {
                    return Err(ValidationError::InvalidRelationshipId);
                }
                if *revision == 0 {
                    return Err(ValidationError::InvalidRevision);
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RelationshipQuery {
    pub object: Option<ObjectRef>,
    pub source: Option<ObjectRef>,
    pub target: Option<ObjectRef>,
    pub relation_type: Option<String>,
    pub direction: Option<RelationDirection>,
    pub active_at: Option<DateTime<Utc>>,
    pub provenance_kind: Option<String>,
    pub minimum_confidence: Option<f32>,
}

impl RelationshipQuery {
    pub fn matches(&self, relationship: &Relationship) -> bool {
        if let Some(object) = &self.object {
            let from = &relationship.source == object;
            let to = &relationship.target == object;
            let hits = match self.direction.unwrap_or(RelationDirection::Either) {
                RelationDirection::From => from,
                RelationDirection::To => to,
                RelationDirection::Either => from || to,
            };
            if !hits {
                return false;
            }
        }
        if self
            .source
            .as_ref()
            .is_some_and(|source| &relationship.source != source)
        {
            return false;
        }
        if self
            .target
            .as_ref()
            .is_some_and(|target| &relationship.target != target)
        {
            return false;
        }
        if self
            .relation_type
            .as_ref()
            .is_some_and(|relation_type| &relationship.relation_type != relation_type)
        {
            return false;
        }
        if self
            .active_at
            .is_some_and(|now| !relationship.is_active_at(now))
        {
            return false;
        }
        if self
            .provenance_kind
            .as_ref()
            .is_some_and(|kind| relationship.provenance.kind_token() != kind)
        {
            return false;
        }
        if let Some(minimum) = self.minimum_confidence {
            match relationship.confidence {
                Some(value) if value >= minimum => {}
                _ => return false,
            }
        }
        true
    }
}

fn validate_known_relation_shape(
    relation_type: &str,
    source: &ObjectRef,
    target: &ObjectRef,
) -> Result<(), ValidationError> {
    match relation_type {
        RELATION_IN_SPACE => match (source, target) {
            (ObjectRef::Entity { .. }, ObjectRef::Space { .. }) => Ok(()),
            _ => Err(ValidationError::InvalidRelationshipEndpoint),
        },
        RELATION_REALIZES | RELATION_EXECUTES | RELATION_PRODUCES | RELATION_RESULT_OF => {
            match (source, target) {
                (ObjectRef::Entity { .. }, ObjectRef::Entity { .. }) => Ok(()),
                _ => Err(ValidationError::InvalidRelationshipEndpoint),
            }
        }
        _ => Ok(()),
    }
}

fn validate_provenance_label(value: &str) -> Result<(), ValidationError> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.len() > MAX_PROVENANCE_SOURCE_BYTES
        || value.chars().any(char::is_control)
    {
        return Err(ValidationError::InvalidProvenance);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SCHEMA_VERSION;
    use chrono::TimeZone;
    use serde_json::json;

    fn now() -> DateTime<Utc> {
        Utc.timestamp_opt(1_788_912_000, 0).unwrap()
    }

    fn relationship() -> Relationship {
        Relationship {
            schema: SCHEMA_VERSION,
            id: Uuid::from_u128(10),
            source: ObjectRef::entity(Uuid::from_u128(1)),
            target: ObjectRef::space("work"),
            relation_type: RELATION_IN_SPACE.into(),
            provenance: Provenance::User,
            confidence: None,
            valid_from: None,
            valid_until: None,
            properties: Map::new(),
            revision: 1,
            created_at: now(),
            updated_at: now(),
        }
    }

    #[test]
    fn valid_relationship_is_accepted() {
        relationship().validate().unwrap();
    }

    #[test]
    fn nil_source_entity_is_rejected() {
        let mut value = relationship();
        value.source = ObjectRef::entity(Uuid::nil());
        assert_eq!(value.validate(), Err(ValidationError::InvalidObjectRef));
    }

    #[test]
    fn invalid_target_space_is_rejected() {
        let mut value = relationship();
        value.target = ObjectRef::space("../work");
        assert_eq!(value.validate(), Err(ValidationError::InvalidSpaceId));
    }

    #[test]
    fn bad_relation_type_is_rejected() {
        let mut value = relationship();
        value.relation_type = "saaios.in_space".into();
        assert_eq!(value.validate(), Err(ValidationError::InvalidRelationType));
    }

    #[test]
    fn confidence_above_one_is_rejected() {
        let mut value = relationship();
        value.provenance = Provenance::Model {
            provider: None,
            model: Some("probe".into()),
        };
        value.confidence = Some(1.1);
        assert_eq!(value.validate(), Err(ValidationError::InvalidConfidence));
    }

    #[test]
    fn confidence_below_zero_is_rejected() {
        let mut value = relationship();
        value.provenance = Provenance::Derived {
            rule: "path-depth-2".into(),
        };
        value.confidence = Some(-0.01);
        assert_eq!(value.validate(), Err(ValidationError::InvalidConfidence));
    }

    #[test]
    fn asserted_user_relation_cannot_carry_confidence() {
        let mut value = relationship();
        value.confidence = Some(1.0);
        assert_eq!(value.validate(), Err(ValidationError::InvalidConfidence));
    }

    #[test]
    fn invalid_validity_window_is_rejected() {
        let mut value = relationship();
        value.valid_from = Some(now());
        value.valid_until = Some(now() - chrono::Duration::seconds(1));
        assert_eq!(
            value.validate(),
            Err(ValidationError::InvalidValidityWindow)
        );
    }

    #[test]
    fn revision_zero_is_rejected() {
        let mut value = relationship();
        value.revision = 0;
        assert_eq!(value.validate(), Err(ValidationError::InvalidRevision));
    }

    #[test]
    fn in_space_requires_entity_to_space() {
        let mut value = relationship();
        value.source = ObjectRef::space("home");
        assert_eq!(
            value.validate(),
            Err(ValidationError::InvalidRelationshipEndpoint)
        );
    }

    #[test]
    fn model_provenance_stays_distinct_from_user_confirmation() {
        let mut value = relationship();
        value.provenance = Provenance::Model {
            provider: Some("local".into()),
            model: Some("notes".into()),
        };
        value.confidence = Some(0.74);
        value
            .properties
            .insert("user_confirmed".into(), json!(false));
        value.validate().unwrap();
        assert!(value.provenance.is_inferred());
        assert!(!value.user_confirmed());
        value
            .properties
            .insert("user_confirmed".into(), json!(true));
        assert!(value.user_confirmed());
        assert_eq!(value.provenance.kind_token(), "model");
    }

    #[test]
    fn derived_relation_keeps_derivation_rule() {
        let mut value = relationship();
        value.relation_type = "saaios.related-to".into();
        value.target = ObjectRef::entity(Uuid::from_u128(2));
        value.provenance = Provenance::Derived {
            rule: "member-of-then-in-space".into(),
        };
        value.validate().unwrap();
        assert!(matches!(
            value.provenance,
            Provenance::Derived { rule } if rule == "member-of-then-in-space"
        ));
    }

    #[test]
    fn expired_relation_is_not_active() {
        let mut value = relationship();
        value.valid_from = Some(now() - chrono::Duration::hours(2));
        value.valid_until = Some(now() - chrono::Duration::hours(1));
        assert!(!value.is_active_at(now()));
        value.valid_until = Some(now() + chrono::Duration::hours(1));
        assert!(value.is_active_at(now()));
    }

    #[test]
    fn query_filters_direction_type_and_active_window() {
        let home = relationship();
        let mut work = relationship();
        work.id = Uuid::from_u128(11);
        work.target = ObjectRef::space("saaios");
        work.valid_until = Some(now() - chrono::Duration::seconds(1));

        let query = RelationshipQuery {
            object: Some(ObjectRef::entity(Uuid::from_u128(1))),
            direction: Some(RelationDirection::From),
            relation_type: Some(RELATION_IN_SPACE.into()),
            active_at: Some(now()),
            ..RelationshipQuery::default()
        };
        assert!(query.matches(&home));
        assert!(!query.matches(&work));
    }
}
