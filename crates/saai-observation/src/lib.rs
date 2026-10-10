//! World Model v1 (ADR-122 WORLD-01/02/06): Observation, cache, one Health.
//!
//! Does not own Attention or a daemon. Does not write Entities.
//! `system.metrics` tool JSON stays the compatibility input.

mod cache;
mod cellular;
mod health;
mod metrics;
mod model;

pub use cache::{ObservationCache, WorldSnapshot};
pub use cellular::{
    bearer_is_live, camp_action, camp_open, cmdline_is_camp_owner, is_cellular_iface, last_data_registration_raw,
    last_voice_registration_raw, last_radio_token, last_selection_mode, last_stack_mode,
    last_device_service, last_voice_operation, last_allow_data, last_initial_attach,
    last_dns, last_dns6, last_modem_config, last_sgc, last_radio_power, last_voice_set,
    last_ipv4, last_ipv6, last_data_setup, last_data_profile, last_activity, last_fastdorm,
    last_endc, last_throttle, last_unsolff, last_unsol, last_screen, last_cellinfo, last_smsc,
    last_vonrget, last_aptime, last_dbgtrace, last_tty, last_pssvc, last_prefmodem, last_slot,
    last_sigcrit, last_smsact, last_linkcrit, last_smscb, last_calllist,
    last_sim_presence, last_supervisor_token,
    latest_numeric_epoch,
    link_is_modem_endpoint, observations_from_cellular, owner_fact_lines, endpoint_holder,
    CellularReading, IfaceSample, KEY_BEARER, KEY_CP_STATE, KEY_REGISTRATION_RAW,
};
pub use health::{cpu_sampler_health, health_of, HealthReport, HealthState, COMPONENT_CPU_SAMPLER};
pub use metrics::{
    observations_from_system_metrics, MetricsOrigin, KEY_CPU_USAGE, KEY_LOAD_AVERAGE,
    KEY_MEMORY_USED_PERCENT, METRICS_CPU_TTL, METRICS_LOAD_TTL, METRICS_MEMORY_TTL,
};
pub use model::{
    freshness_at, Freshness, Observation, ObservationError, ObservationQuality, ObservationSource,
    ObservationSourceKind, ObservationSubject, SCHEMA_VERSION,
};
