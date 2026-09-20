//! WORLD-06: one Health component from Observation. Not a dashboard.
//! Stale required evidence is Unknown, never last-known Healthy.

use crate::cache::WorldSnapshot;
use crate::metrics::KEY_CPU_USAGE;
use crate::model::{freshness_at, Freshness, Observation, ObservationQuality, ObservationSubject};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Sampler liveness for `system.cpu.usage`. Magnitude is not a threshold.
pub const COMPONENT_CPU_SAMPLER: &str = "system.cpu.sampler";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthState {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthReport {
    pub component_id: String,
    pub state: HealthState,
    pub observation_id: Option<Uuid>,
    pub freshness: Option<Freshness>,
}

/// CPU sampler: Fresh Direct sample → Healthy. Stale or missing → Unknown.
/// Percent is not Unhealthy. No graphs.
pub fn cpu_sampler_health(snapshot: &WorldSnapshot) -> HealthReport {
    health_of(COMPONENT_CPU_SAMPLER, KEY_CPU_USAGE, snapshot)
}

pub fn health_of(component_id: &str, key: &str, snapshot: &WorldSnapshot) -> HealthReport {
    let observation = snapshot
        .observations
        .iter()
        .find(|row| row.key == key && matches!(row.subject, ObservationSubject::LocalDevice));
    match observation {
        None => HealthReport {
            component_id: component_id.to_string(),
            state: HealthState::Unknown,
            observation_id: None,
            freshness: None,
        },
        Some(observation) => report_from_observation(component_id, observation, snapshot),
    }
}

fn report_from_observation(
    component_id: &str,
    observation: &Observation,
    snapshot: &WorldSnapshot,
) -> HealthReport {
    let freshness = freshness_at(observation, snapshot.captured_at);
    let state = match freshness {
        Freshness::Stale => HealthState::Unknown,
        Freshness::Fresh => match observation.value.as_bool() {
            Some(false) => HealthState::Unhealthy,
            _ if observation.quality == ObservationQuality::Approximate => HealthState::Degraded,
            _ => HealthState::Healthy,
        },
    };
    HealthReport {
        component_id: component_id.to_string(),
        state,
        observation_id: Some(observation.id),
        freshness: Some(freshness),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::ObservationCache;
    use crate::metrics::KEY_CPU_USAGE;
    use crate::model::{
        ObservationSource, ObservationSourceKind, ObservationSubject, SCHEMA_VERSION,
    };
    use chrono::{Duration as ChronoDuration, Utc};
    use serde_json::json;

    fn sample(key: &str, value: serde_json::Value, quality: ObservationQuality) -> Observation {
        Observation {
            schema: SCHEMA_VERSION,
            id: Uuid::new_v4(),
            subject: ObservationSubject::LocalDevice,
            key: key.into(),
            value,
            unit: None,
            source: ObservationSource {
                source_id: "test.sampler".into(),
                kind: ObservationSourceKind::Direct,
            },
            quality,
            observed_at: Utc::now(),
            valid_for_ms: 5_000,
            sequence: 1,
        }
    }

    #[test]
    fn missing_sample_is_unknown() {
        let cache = ObservationCache::new();
        let report = cpu_sampler_health(&cache.snapshot(Utc::now()));
        assert_eq!(report.state, HealthState::Unknown);
        assert!(report.observation_id.is_none());
    }

    #[test]
    fn stale_sample_is_unknown_not_unhealthy() {
        let cache = ObservationCache::new();
        let mut row = sample(KEY_CPU_USAGE, json!(12.0), ObservationQuality::Direct);
        row.observed_at = Utc::now() - ChronoDuration::seconds(30);
        cache.apply(vec![row]);
        let report = cpu_sampler_health(&cache.snapshot(Utc::now()));
        assert_eq!(report.freshness, Some(Freshness::Stale));
        assert_eq!(report.state, HealthState::Unknown);
        assert_ne!(report.state, HealthState::Unhealthy);
        assert_ne!(report.state, HealthState::Healthy);
    }

    #[test]
    fn fresh_cpu_sample_is_healthy_regardless_of_percent() {
        let cache = ObservationCache::new();
        cache.apply(vec![sample(
            KEY_CPU_USAGE,
            json!(97.0),
            ObservationQuality::Direct,
        )]);
        let report = cpu_sampler_health(&cache.snapshot(Utc::now()));
        assert_eq!(report.state, HealthState::Healthy);
        assert_eq!(report.freshness, Some(Freshness::Fresh));
    }

    #[test]
    fn fresh_approximate_is_degraded() {
        let cache = ObservationCache::new();
        cache.apply(vec![sample(
            KEY_CPU_USAGE,
            json!(12.0),
            ObservationQuality::Approximate,
        )]);
        let report = cpu_sampler_health(&cache.snapshot(Utc::now()));
        assert_eq!(report.state, HealthState::Degraded);
    }

    #[test]
    fn fresh_boolean_false_is_unhealthy() {
        let cache = ObservationCache::new();
        cache.apply(vec![sample(
            "system.cpu.ok",
            json!(false),
            ObservationQuality::Direct,
        )]);
        let report = health_of(
            "system.cpu.ok",
            "system.cpu.ok",
            &cache.snapshot(Utc::now()),
        );
        assert_eq!(report.state, HealthState::Unhealthy);
    }

    #[test]
    fn health_state_is_not_freshness() {
        let encoded = serde_json::to_value(HealthState::Unknown).unwrap();
        assert_eq!(encoded, "unknown");
        assert_ne!(encoded, serde_json::to_value(Freshness::Stale).unwrap());
    }
}
