//! World Model v1 (ADR-122 WORLD-01/02/06): Observation, cache, one Health.
//!
//! Does not own Attention or a daemon. Does not write Entities.
//! `system.metrics` tool JSON stays the compatibility input.

mod cache;
mod health;
mod metrics;
mod model;

pub use cache::{ObservationCache, WorldSnapshot};
pub use health::{cpu_sampler_health, health_of, HealthReport, HealthState, COMPONENT_CPU_SAMPLER};
pub use metrics::{
    observations_from_system_metrics, MetricsOrigin, KEY_CPU_USAGE, KEY_LOAD_AVERAGE,
    KEY_MEMORY_USED_PERCENT, METRICS_CPU_TTL, METRICS_LOAD_TTL, METRICS_MEMORY_TTL,
};
pub use model::{
    freshness_at, Freshness, Observation, ObservationError, ObservationQuality, ObservationSource,
    ObservationSourceKind, ObservationSubject, SCHEMA_VERSION,
};
