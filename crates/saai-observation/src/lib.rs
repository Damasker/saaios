//! World Model v1 (ADR-122 WORLD-01/02/04/06): Observation, cache, Linux
//! observers, one Health.
//!
//! Does not own Attention or a daemon. Does not write Entities.
//! `system.metrics` tool JSON stays the compatibility input.

mod cache;
mod cellular;
mod health;
mod linux;
mod locality;
mod metrics;
mod model;

pub use cache::{ObservationCache, WorldSnapshot};
pub use cellular::{
    bearer_is_live, camp_action, camp_open, cmdline_is_camp_owner, endpoint_holder,
    is_cellular_iface, last_activity, last_allow_data, last_aptime, last_barring, last_calllist,
    last_cellinfo, last_data_call, last_data_profile, last_data_registration_raw, last_data_setup,
    last_dbgtrace, last_device_service, last_dns, last_dns6, last_endc, last_fastdorm,
    last_gpslock, last_gpsnfw, last_initial_attach, last_ipv4, last_ipv6, last_link_estimate,
    last_linkcrit, last_modem_config, last_phy_channel, last_prefmodem, last_pssvc,
    last_radio_power, last_radio_token, last_samode, last_screen, last_selection_mode, last_sgc,
    last_sigcrit, last_signal_mask, last_sim_presence, last_slot, last_smsact, last_smsc,
    last_smscb, last_stack_mode, last_supervisor_token, last_throttle, last_tty, last_unsol,
    last_unsolff, last_voice_operation, last_voice_registration_raw, last_voice_set, last_vonrget,
    latest_numeric_epoch, link_is_modem_endpoint, observations_from_cellular, owner_fact_lines,
    owner_fact_lines_from_lines,
    traffic_token, CellularReading, DataCallList, IfaceSample, LinkEstimate, PhyChannel,
    KEY_BEARER, KEY_CP_STATE, KEY_REGISTRATION_RAW,
};
pub use health::{cpu_sampler_health, health_of, HealthReport, HealthState, COMPONENT_CPU_SAMPLER};
pub use linux::{
    cpu_usage_between, read_cpu_times, read_load_average, read_meminfo_mb,
    sample_host_metrics_blocking, HostMetricsSample,
};
pub use locality::{
    observations_from_locality_cache, observations_from_open_meteo_current, wmo_condition_label,
    Locality, LocalityCache, ATTRIBUTION, KEY_ATTRIBUTION, KEY_CONDITION, KEY_HUMIDITY,
    KEY_LATITUDE, KEY_LOCALITY_NAME, KEY_LONGITUDE, KEY_TEMPERATURE, KEY_WEATHER_CODE,
    KEY_WIND_SPEED, LOCALITY_WEATHER_TTL_MS, SOURCE_OPEN_METEO,
};
pub use metrics::{
    observations_from_system_metrics, MetricsOrigin, KEY_CPU_USAGE, KEY_LOAD_AVERAGE,
    KEY_MEMORY_USED_PERCENT, METRICS_CPU_TTL, METRICS_LOAD_TTL, METRICS_MEMORY_TTL,
};
pub use model::{
    freshness_at, Freshness, Observation, ObservationError, ObservationQuality, ObservationSource,
    ObservationSourceKind, ObservationSubject, SCHEMA_VERSION,
};
