//! MEM-03: typed MemoryRecord. Legacy MemoryFact JSONL is a view.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::MemoryFact;

pub const MEMORY_SCHEMA_V2: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    ExplicitFact,
    ExplicitPreference,
    LearnedHypothesis,
}

impl MemoryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExplicitFact => "explicit_fact",
            Self::ExplicitPreference => "explicit_preference",
            Self::LearnedHypothesis => "learned_hypothesis",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryState {
    Active,
    Invalidated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemorySensitivity {
    Normal,
    Restricted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryActor {
    Store,
    LegacyImport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProvenance {
    pub actor: MemoryActor,
    pub assigned_at: DateTime<Utc>,
    pub correlation_id: Option<Uuid>,
}

impl MemoryProvenance {
    pub fn assign(correlation_id: Option<Uuid>) -> Self {
        Self {
            actor: MemoryActor::Store,
            assigned_at: Utc::now(),
            correlation_id,
        }
    }

    pub fn legacy(correlation_id: Option<Uuid>) -> Self {
        Self {
            actor: MemoryActor::LegacyImport,
            assigned_at: Utc::now(),
            correlation_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MemoryValidity {
    Durable,
    Until { at: DateTime<Utc> },
}

impl Default for MemoryValidity {
    fn default() -> Self {
        Self::Durable
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryRecord {
    #[serde(default = "default_schema")]
    pub schema: u32,
    pub id: Uuid,
    pub ts: DateTime<Utc>,
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub space_id: Option<String>,
    #[serde(default)]
    pub kind: MemoryKind,
    #[serde(default)]
    pub state: MemoryState,
    #[serde(default)]
    pub sensitivity: MemorySensitivity,
    #[serde(default)]
    pub validity: MemoryValidity,
    pub provenance: MemoryProvenance,
}

fn default_schema() -> u32 {
    MEMORY_SCHEMA_V2
}

impl Default for MemoryKind {
    fn default() -> Self {
        Self::ExplicitFact
    }
}

impl Default for MemoryState {
    fn default() -> Self {
        Self::Active
    }
}

impl Default for MemorySensitivity {
    fn default() -> Self {
        Self::Normal
    }
}

impl MemoryRecord {
    pub fn from_legacy(fact: MemoryFact) -> Self {
        Self {
            schema: 1,
            id: fact.id,
            ts: fact.ts,
            key: fact.key,
            value: fact.value,
            tags: fact.tags,
            space_id: fact.space_id,
            kind: MemoryKind::ExplicitFact,
            state: if fact.deleted {
                MemoryState::Invalidated
            } else {
                MemoryState::Active
            },
            sensitivity: MemorySensitivity::Normal,
            validity: MemoryValidity::Durable,
            provenance: MemoryProvenance::legacy(fact.origin_correlation_id),
        }
    }

    pub fn from_write(fact: MemoryFact, kind: MemoryKind) -> Self {
        Self {
            schema: MEMORY_SCHEMA_V2,
            id: if fact.id.is_nil() {
                Uuid::new_v4()
            } else {
                fact.id
            },
            ts: if fact.ts.timestamp() == 0 {
                Utc::now()
            } else {
                fact.ts
            },
            key: fact.key,
            value: fact.value,
            tags: fact.tags,
            space_id: fact.space_id,
            kind,
            state: MemoryState::Active,
            sensitivity: MemorySensitivity::Normal,
            validity: MemoryValidity::Durable,
            provenance: MemoryProvenance::assign(fact.origin_correlation_id),
        }
    }

    pub fn is_live(&self, now: DateTime<Utc>) -> bool {
        if self.state != MemoryState::Active {
            return false;
        }
        match self.validity {
            MemoryValidity::Durable => true,
            MemoryValidity::Until { at } => now < at,
        }
    }

    pub fn as_fact(&self) -> MemoryFact {
        MemoryFact {
            id: self.id,
            ts: self.ts,
            key: self.key.clone(),
            value: self.value.clone(),
            tags: self.tags.clone(),
            source: Some(match self.provenance.actor {
                MemoryActor::Store => "store".into(),
                MemoryActor::LegacyImport => "legacy".into(),
            }),
            deleted: self.state == MemoryState::Invalidated,
            space_id: self.space_id.clone(),
            origin_correlation_id: self.provenance.correlation_id,
        }
    }
}

pub fn parse_line(line: &str) -> Result<MemoryRecord, serde_json::Error> {
    let value: serde_json::Value = serde_json::from_str(line)?;
    let v2 = value
        .get("schema")
        .and_then(|schema| schema.as_u64())
        .is_some_and(|schema| schema >= 2)
        || value.get("kind").is_some()
        || value.get("provenance").is_some();
    if v2 {
        serde_json::from_value(value)
    } else {
        let fact: MemoryFact = serde_json::from_value(value)?;
        Ok(MemoryRecord::from_legacy(fact))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_jsonl_becomes_explicit_fact_view() {
        let line = r#"{"id":"11111111-1111-1111-1111-111111111111","ts":"2026-09-01T00:00:00Z","key":"host.role","value":"pi5","deleted":false}"#;
        let record = parse_line(line).unwrap();
        assert_eq!(record.kind, MemoryKind::ExplicitFact);
        assert_eq!(record.state, MemoryState::Active);
        assert_eq!(record.provenance.actor, MemoryActor::LegacyImport);
        assert_eq!(record.key, "host.role");
        assert!(!record.as_fact().deleted);
    }

    #[test]
    fn legacy_deleted_is_invalidated() {
        let line = r#"{"id":"11111111-1111-1111-1111-111111111111","ts":"2026-09-01T00:00:00Z","key":"host.role","value":"pi5","deleted":true}"#;
        let record = parse_line(line).unwrap();
        assert_eq!(record.state, MemoryState::Invalidated);
        assert!(record.as_fact().deleted);
    }

    #[test]
    fn caller_source_is_not_provenance() {
        let mut fact = MemoryFact::new("ui.detail", "technical");
        fact.source = Some("model".into());
        let record = MemoryRecord::from_write(fact, MemoryKind::ExplicitFact);
        assert_eq!(record.provenance.actor, MemoryActor::Store);
        assert_ne!(record.as_fact().source.as_deref(), Some("model"));
    }

    #[test]
    fn v2_round_trip_keeps_kind() {
        let mut fact = MemoryFact::new("theme", "dark");
        fact.space_id = Some("home".into());
        let record = MemoryRecord::from_write(fact, MemoryKind::ExplicitPreference);
        let line = serde_json::to_string(&record).unwrap();
        let parsed = parse_line(&line).unwrap();
        assert_eq!(parsed.kind, MemoryKind::ExplicitPreference);
        assert_eq!(parsed.schema, MEMORY_SCHEMA_V2);
        assert_eq!(parsed.space_id.as_deref(), Some("home"));
    }

    #[test]
    fn until_validity_expires() {
        let mut record =
            MemoryRecord::from_write(MemoryFact::new("tmp", "x"), MemoryKind::ExplicitFact);
        record.validity = MemoryValidity::Until {
            at: Utc::now() - chrono::Duration::seconds(1),
        };
        assert!(!record.is_live(Utc::now()));
    }
}
