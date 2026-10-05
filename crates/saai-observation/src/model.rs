//! Observation is an atomic measured fact: subject, key, value, source, time, TTL.
//! Freshness is derived. It is not HealthState.

use chrono::{DateTime, Utc};
use saai_entity_store::ObjectRef;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ObservationSubject {
    LocalDevice,
    Object { object: ObjectRef },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationSourceKind {
    Direct,
    Derived,
    Imported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationSource {
    pub source_id: String,
    pub kind: ObservationSourceKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationQuality {
    Authoritative,
    Direct,
    Derived,
    Approximate,
}

/// Fresh vs Stale is not a HealthState. Stale required evidence → Unknown health later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    Fresh,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub schema: u32,
    pub id: Uuid,
    pub subject: ObservationSubject,
    pub key: String,
    pub value: Value,
    pub unit: Option<String>,
    pub source: ObservationSource,
    pub quality: ObservationQuality,
    pub observed_at: DateTime<Utc>,
    pub valid_for_ms: u64,
    pub sequence: u64,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ObservationError {
    #[error("observation key is empty")]
    EmptyKey,
    #[error("observation source_id is empty")]
    EmptySource,
    #[error("observation value is not a finite number")]
    InvalidNumericValue,
}

impl Observation {
    pub fn validate(&self) -> Result<(), ObservationError> {
        if self.key.is_empty() {
            return Err(ObservationError::EmptyKey);
        }
        if self.source.source_id.is_empty() {
            return Err(ObservationError::EmptySource);
        }
        if let Some(n) = self.value.as_f64() {
            if !n.is_finite() {
                return Err(ObservationError::InvalidNumericValue);
            }
        }
        Ok(())
    }
}

pub fn freshness_at(observation: &Observation, now: DateTime<Utc>) -> Freshness {
    let age = now.signed_duration_since(observation.observed_at);
    if age.num_milliseconds() < 0 {
        return Freshness::Stale;
    }
    let age = age.to_std().unwrap_or(Duration::MAX);
    if age > Duration::from_millis(observation.valid_for_ms) {
        Freshness::Stale
    } else {
        Freshness::Fresh
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration as ChronoDuration;

    fn sample(valid_for_ms: u64, observed_at: DateTime<Utc>) -> Observation {
        Observation {
            schema: SCHEMA_VERSION,
            id: Uuid::new_v4(),
            subject: ObservationSubject::LocalDevice,
            key: "system.cpu.usage".into(),
            value: serde_json::json!(37.0),
            unit: Some("percent".into()),
            source: ObservationSource {
                source_id: "mock.system.metrics".into(),
                kind: ObservationSourceKind::Direct,
            },
            quality: ObservationQuality::Direct,
            observed_at,
            valid_for_ms,
            sequence: 1,
        }
    }

    #[test]
    fn fresh_within_ttl() {
        let t0 = Utc::now();
        let obs = sample(5_000, t0);
        assert_eq!(
            freshness_at(&obs, t0 + ChronoDuration::seconds(2)),
            Freshness::Fresh
        );
    }

    #[test]
    fn stale_after_ttl() {
        let t0 = Utc::now();
        let obs = sample(5_000, t0);
        assert_eq!(
            freshness_at(&obs, t0 + ChronoDuration::seconds(6)),
            Freshness::Stale
        );
    }

    #[test]
    fn stale_is_not_a_health_state() {
        // Compile-time documentation: Freshness and Health stay separate types.
        let _fresh = Freshness::Stale;
        let encoded = serde_json::to_value(Freshness::Stale).unwrap();
        assert_eq!(encoded, "stale");
        assert!(serde_json::to_string(&encoded).unwrap().contains("stale"));
        assert!(!serde_json::to_string(&encoded)
            .unwrap()
            .contains("unhealthy"));
    }

    #[test]
    fn rejects_empty_key() {
        let mut obs = sample(1_000, Utc::now());
        obs.key.clear();
        assert_eq!(obs.validate(), Err(ObservationError::EmptyKey));
    }
}
