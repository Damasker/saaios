//! WORLD-02: in-process latest-by-key cache. Not a daemon. Not entity-store.

use crate::model::{freshness_at, Freshness, Observation, ObservationSubject, SCHEMA_VERSION};
use chrono::{DateTime, Utc};
use saai_entity_store::ObjectRef;
use std::collections::HashMap;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum CacheKey {
    Device(String),
    Entity { id: Uuid, key: String },
    Space { id: String, key: String },
}

impl CacheKey {
    fn from_observation(observation: &Observation) -> Self {
        match &observation.subject {
            ObservationSubject::LocalDevice => CacheKey::Device(observation.key.clone()),
            ObservationSubject::Object { object } => match object {
                ObjectRef::Entity { id } => CacheKey::Entity {
                    id: *id,
                    key: observation.key.clone(),
                },
                ObjectRef::Space { id } => CacheKey::Space {
                    id: id.clone(),
                    key: observation.key.clone(),
                },
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorldSnapshot {
    pub schema: u32,
    pub revision: u64,
    pub captured_at: DateTime<Utc>,
    pub observations: Vec<Observation>,
}

impl WorldSnapshot {
    pub fn freshness(&self, observation: &Observation) -> Freshness {
        freshness_at(observation, self.captured_at)
    }
}

struct Inner {
    revision: u64,
    rows: HashMap<CacheKey, Observation>,
}

/// Latest Observation per subject+key. Empty after construction: reboot
/// must not resurrect samples as Fresh.
pub struct ObservationCache {
    inner: Mutex<Inner>,
}

impl ObservationCache {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                revision: 0,
                rows: HashMap::new(),
            }),
        }
    }

    pub fn revision(&self) -> u64 {
        self.inner.lock().expect("observation cache").revision
    }

    /// Replace latest rows. Invalid observations are dropped. Revision
    /// bumps once when at least one row is stored.
    pub fn apply(&self, observations: Vec<Observation>) -> u64 {
        let mut inner = self.inner.lock().expect("observation cache");
        let mut stored = 0u64;
        for observation in observations {
            if observation.validate().is_err() {
                continue;
            }
            inner
                .rows
                .insert(CacheKey::from_observation(&observation), observation);
            stored += 1;
        }
        if stored > 0 {
            inner.revision = inner.revision.saturating_add(1);
        }
        inner.revision
    }

    pub fn snapshot(&self, now: DateTime<Utc>) -> WorldSnapshot {
        let inner = self.inner.lock().expect("observation cache");
        let mut observations: Vec<Observation> = inner.rows.values().cloned().collect();
        observations.sort_by(|a, b| a.key.cmp(&b.key).then(a.id.as_bytes().cmp(b.id.as_bytes())));
        WorldSnapshot {
            schema: SCHEMA_VERSION,
            revision: inner.revision,
            captured_at: now,
            observations,
        }
    }
}

impl Default for ObservationCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::{observations_from_system_metrics, MetricsOrigin, KEY_CPU_USAGE};
    use crate::model::{
        ObservationQuality, ObservationSource, ObservationSourceKind, ObservationSubject,
    };
    use chrono::Duration as ChronoDuration;
    use serde_json::json;

    fn cpu(valid_for_ms: u64, observed_at: DateTime<Utc>, sequence: u64) -> Observation {
        Observation {
            schema: SCHEMA_VERSION,
            id: Uuid::new_v4(),
            subject: ObservationSubject::LocalDevice,
            key: KEY_CPU_USAGE.into(),
            value: json!(12.0),
            unit: Some("percent".into()),
            source: ObservationSource {
                source_id: "mock.system.metrics".into(),
                kind: ObservationSourceKind::Direct,
            },
            quality: ObservationQuality::Direct,
            observed_at,
            valid_for_ms,
            sequence,
        }
    }

    #[test]
    fn empty_cache_is_revision_zero_not_fresh_metrics() {
        let cache = ObservationCache::new();
        let snap = cache.snapshot(Utc::now());
        assert_eq!(snap.revision, 0);
        assert!(snap.observations.is_empty());
    }

    #[test]
    fn apply_replaces_latest_and_bumps_revision_once() {
        let cache = ObservationCache::new();
        let t0 = Utc::now();
        let first = cpu(5_000, t0, 1);
        let mut second = cpu(5_000, t0, 2);
        second.value = json!(40.0);
        assert_eq!(cache.apply(vec![first, second.clone()]), 1);
        let snap = cache.snapshot(t0);
        assert_eq!(snap.revision, 1);
        assert_eq!(snap.observations.len(), 1);
        assert_eq!(snap.observations[0].value, json!(40.0));
        assert_eq!(snap.observations[0].sequence, 2);
        assert_eq!(cache.apply(vec![second]), 2);
    }

    #[test]
    fn invalid_rows_do_not_bump_revision() {
        let cache = ObservationCache::new();
        let mut bad = cpu(1_000, Utc::now(), 1);
        bad.key.clear();
        assert_eq!(cache.apply(vec![bad]), 0);
        assert!(cache.snapshot(Utc::now()).observations.is_empty());
    }

    #[test]
    fn stale_ttl_is_not_unhealthy() {
        let cache = ObservationCache::new();
        let t0 = Utc::now();
        cache.apply(vec![cpu(5_000, t0, 1)]);
        let snap = cache.snapshot(t0 + ChronoDuration::seconds(6));
        assert_eq!(snap.freshness(&snap.observations[0]), Freshness::Stale);
        let encoded = serde_json::to_string(&snap.freshness(&snap.observations[0])).unwrap();
        assert!(encoded.contains("stale"));
        assert!(!encoded.contains("unhealthy"));
    }

    #[test]
    fn metrics_conversion_fills_cache_without_entity_store_write() {
        let cache = ObservationCache::new();
        let now = Utc::now();
        let rows = observations_from_system_metrics(
            &json!({
                "cpu_usage": 11.0,
                "load_average": 0.4,
                "mem_used_pct": 20.0
            }),
            MetricsOrigin::Mock,
            now,
            3,
        );
        cache.apply(rows);
        let snap = cache.snapshot(now);
        assert_eq!(snap.revision, 1);
        assert_eq!(snap.observations.len(), 3);
        assert!(snap
            .observations
            .iter()
            .all(|row| snap.freshness(row) == Freshness::Fresh));
    }
}
