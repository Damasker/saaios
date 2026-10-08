//! Cellular facts for the observation cache.
//! The caller supplies already-read text. This module does not open files
//! and does not accept an address, operator name, or radio technology.

use crate::model::{
    Observation, ObservationQuality, ObservationSource, ObservationSourceKind, ObservationSubject,
    SCHEMA_VERSION,
};
use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

pub const KEY_CP_STATE: &str = "cellular.cp_state";
pub const KEY_REGISTRATION_RAW: &str = "cellular.registration_raw";
pub const KEY_VOICE_REGISTRATION_RAW: &str = "cellular.voice_registration_raw";
pub const KEY_BEARER: &str = "cellular.bearer";
pub const KEY_SUPERVISOR: &str = "cellular.supervisor";
pub const KEY_RADIO: &str = "cellular.radio";
pub const KEY_SELECTION: &str = "cellular.selection";
pub const KEY_SIM_APP: &str = "cellular.sim_app";
pub const KEY_OWNER: &str = "cellular.owner";
pub const KEY_BOOT_EPOCH: &str = "cellular.boot_epoch";
pub const KEY_ENDPOINT: &str = "cellular.endpoint";
pub const KEY_ACTION: &str = "cellular.action";
pub const KEY_OPEN: &str = "cellular.open";

/// Longer than the default 30s telemetry interval, so one missed sample
/// does not mark the row stale.
pub const CELLULAR_TTL_MS: u64 = 90_000;

pub struct IfaceSample {
    pub name: String,
    pub has_ipv4: bool,
    pub rx: u64,
    pub tx: u64,
}

pub struct CellularReading {
    pub cp_text: Option<String>,
    pub owner_log: Option<String>,
    pub supervisor_log: Option<String>,
    pub owner_running: bool,
    pub boot_epoch: Option<u64>,
    /// Who has `umts_ipc0` or `umts_rfs0` open: `owner`, `modemd`, or `shared`.
    pub endpoint: Option<&'static str>,
    /// A `sit-sim-status` process is already running.
    pub status_lock_busy: bool,
    pub ifaces: Vec<IfaceSample>,
}

pub fn is_cellular_iface(name: &str) -> bool {
    let stem = name
        .split(|c: char| c == '.' || c == '@')
        .next()
        .unwrap_or(name);
    stem.starts_with("rmnet")
        || stem.starts_with("wwan")
        || stem.starts_with("qmimux")
        || stem.starts_with("ccmni")
}

pub fn observations_from_cellular(
    reading: &CellularReading,
    observed_at: DateTime<Utc>,
    sequence: u64,
) -> Vec<Observation> {
    let mut out = Vec::new();
    if let Some(state) = reading.cp_text.as_deref().and_then(cp_state_token) {
        out.push(text_observation(
            KEY_CP_STATE,
            state,
            "sysfs.cpif.modem_state",
            observed_at,
            sequence,
        ));
    }
    let cp_online = reading.cp_text.as_deref().and_then(cp_state_token) == Some("ONLINE");
    let log = (reading.owner_running && cp_online)
        .then_some(reading.owner_log.as_deref())
        .flatten();
    if let Some(raw) = log.and_then(last_data_registration_raw) {
        out.push(number_observation(
            KEY_REGISTRATION_RAW,
            raw,
            "camp.owner.data_registration",
            observed_at,
            sequence,
        ));
    }
    if let Some(raw) = log.and_then(last_voice_registration_raw) {
        out.push(number_observation(
            KEY_VOICE_REGISTRATION_RAW,
            raw,
            "camp.owner.voice_registration",
            observed_at,
            sequence,
        ));
    }
    let mut live: Vec<&str> = reading
        .ifaces
        .iter()
        .filter(|iface| is_cellular_iface(&iface.name) && sample_is_live(iface))
        .map(|iface| iface.name.as_str())
        .collect();
    live.sort();
    live.dedup();
    if !live.is_empty() {
        out.push(text_observation(
            KEY_BEARER,
            &live.join(" · "),
            "sysfs.net.bearer",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_radio_token) {
        out.push(text_observation(
            KEY_RADIO,
            token,
            "camp.owner.radio",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_selection_mode) {
        out.push(text_observation(
            KEY_SELECTION,
            token,
            "camp.owner.selection",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_sim_presence) {
        out.push(text_observation(
            KEY_SIM_APP,
            token,
            "camp.owner.sim_app",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = reading
        .supervisor_log
        .as_deref()
        .and_then(last_supervisor_token)
    {
        out.push(text_observation(
            KEY_SUPERVISOR,
            token,
            "camp.supervisor",
            observed_at,
            sequence,
        ));
    }
    out.push(text_observation(
        KEY_OWNER,
        if reading.owner_running { "running" } else { "gone" },
        "proc.camp_owner",
        observed_at,
        sequence,
    ));
    if let Some(epoch) = reading.boot_epoch {
        out.push(observation(
            KEY_BOOT_EPOCH,
            Value::from(epoch),
            "boot.archive",
            observed_at,
            sequence,
        ));
    }
    if let Some(holder) = reading.endpoint {
        out.push(text_observation(
            KEY_ENDPOINT,
            holder,
            "proc.fd.modem_endpoint",
            observed_at,
            sequence,
        ));
    }
    out.push(text_observation(
        KEY_ACTION,
        camp_action(reading.cp_text.as_deref(), reading.owner_running),
        "camp.lifecycle",
        observed_at,
        sequence,
    ));
    out.push(text_observation(
        KEY_OPEN,
        camp_open(
            reading.cp_text.as_deref(),
            reading.owner_running,
            reading.endpoint,
            reading.status_lock_busy,
        ),
        "camp.lifecycle",
        observed_at,
        sequence,
    ));
    out
}

/// Whether a new camp may start. A running owner, or any CP state other
/// than missing/`OFFLINE`, stays on the camp that is already up.
pub fn camp_action(cp_state: Option<&str>, owner_running: bool) -> &'static str {
    if owner_running {
        return "attend";
    }
    match cp_state.map(str::trim) {
        None | Some("OFFLINE") => "launch-once",
        Some(_) => "attend",
    }
}

/// Whether a query may open the modem endpoint. `ready` is the only
/// admission. Every other word is the first reason to refuse.
pub fn camp_open(
    cp_state: Option<&str>,
    owner_running: bool,
    endpoint_holder: Option<&str>,
    status_lock_busy: bool,
) -> &'static str {
    if owner_running {
        "owner"
    } else if matches!(endpoint_holder, Some("owner" | "modemd" | "shared")) {
        "endpoint"
    } else if status_lock_busy {
        "lock"
    } else if cp_state.map(str::trim) != Some("ONLINE") {
        "cp"
    } else {
        "ready"
    }
}

/// `owner` holds the modem endpoint, `modemd` holds it, or both do.
/// Neither means the key stays absent.
pub fn endpoint_holder(owner_has: bool, modemd_has: bool) -> Option<&'static str> {
    match (owner_has, modemd_has) {
        (true, false) => Some("owner"),
        (false, true) => Some("modemd"),
        (true, true) => Some("shared"),
        (false, false) => None,
    }
}

pub fn link_is_modem_endpoint(target: &str) -> bool {
    target.ends_with("/umts_ipc0") || target.ends_with("/umts_rfs0")
}

/// Newest all-digit name. Other names are ignored.
pub fn latest_numeric_epoch<'a, I>(names: I) -> Option<u64>
where
    I: IntoIterator<Item = &'a str>,
{
    names
        .into_iter()
        .filter(|name| !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_digit()))
        .filter_map(|name| name.parse().ok())
        .max()
}

/// Last `supervise=` word the boot supervisor actually prints.
/// `handoff-exit` and anything else stay out of the cache.
pub fn last_supervisor_token(log: &str) -> Option<&str> {
    log.lines().rev().find_map(|line| {
        let rest = line.trim().strip_prefix("supervise=")?;
        let token = rest.split_whitespace().next()?;
        matches!(
            token,
            "hold" | "owner-gone" | "cp-left" | "attend" | "launch-once"
        )
        .then_some(token)
    })
}

pub fn last_radio_token(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some(rest) = line.split_once("field=radio radio_raw=") {
            let digits: String = rest.1.chars().take_while(|c| c.is_ascii_digit()).collect();
            return match digits.parse::<u32>().ok() {
                Some(10) => Some("on"),
                _ => None,
            };
        }
        if line.contains("field=radio status=unknown_short") {
            return None;
        }
    }
    None
}

/// Last data-registration, radio, and SIM lines. Early one-shot lines stay
/// even after the camp log grows past a tail window.
pub fn owner_fact_lines(log: &str) -> String {
    let mut data = None;
    let mut voice = None;
    let mut radio = None;
    let mut selection = None;
    let mut sim = None;
    for line in log.lines() {
        if line.contains("field=data registration_raw=") {
            data = Some(line);
        } else if line.contains("field=voice registration_raw=") {
            voice = Some(line);
        } else if line.contains("field=radio ") {
            radio = Some(line);
        } else if line.contains("field=selection ") {
            selection = Some(line);
        } else if line.contains("camp_sim=") || line.contains("field=sim ") {
            sim = Some(line);
        }
    }
    let mut out = String::new();
    for line in [data, voice, radio, selection, sim].into_iter().flatten() {
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Last SIM application fact from the camp log. PIN state stays out.
pub fn last_sim_presence(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if line.contains("camp_sim=ready") {
            return Some("ready");
        }
        if line.contains("field=sim ") {
            if line.contains("status=unknown_short") {
                return None;
            }
            let Some(rest) = line.split_once("apps=") else {
                return None;
            };
            let digits: String = rest.1.chars().take_while(|c| c.is_ascii_digit()).collect();
            return match digits.parse::<u32>().ok() {
                Some(0) => Some("absent"),
                Some(n) if (1..=8).contains(&n) => Some("present"),
                _ => None,
            };
        }
    }
    None
}

/// An address on the iface, or both directions moved. The address itself
/// stays with the caller.
pub fn bearer_is_live(has_ipv4: bool, rx: u64, tx: u64) -> bool {
    has_ipv4 || (rx > 0 && tx > 0)
}

fn sample_is_live(iface: &IfaceSample) -> bool {
    bearer_is_live(iface.has_ipv4, iface.rx, iface.tx)
}

fn cp_state_token(text: &str) -> Option<&str> {
    let word = text.trim();
    if word.is_empty()
        || word.len() > 16
        || !word
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
    {
        return None;
    }
    Some(word)
}

/// Last selection mode. `0` is automatic, `1` is manual. Anything else stays out.
pub fn last_selection_mode(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some(rest) = line.split_once("field=selection mode_raw=") {
            let digits: String = rest.1.chars().take_while(|c| c.is_ascii_digit()).collect();
            return match digits.parse::<u32>().ok() {
                Some(0) => Some("automatic"),
                Some(1) => Some("manual"),
                _ => None,
            };
        }
        if line.contains("field=selection status=unknown_short") {
            return None;
        }
    }
    None
}

/// Last data `registration_raw` in 0..=5. Reject, LAC, and CID stay out.
pub fn last_data_registration_raw(log: &str) -> Option<u32> {
    last_named_registration_raw(log, "field=data registration_raw=")
}

/// Last voice `registration_raw` in 0..=5. Reject, LAC, and CID stay out.
pub fn last_voice_registration_raw(log: &str) -> Option<u32> {
    last_named_registration_raw(log, "field=voice registration_raw=")
}

fn last_named_registration_raw(log: &str, needle: &str) -> Option<u32> {
    log.lines().rev().find_map(|line| {
        let rest = line.split_once(needle)?.1;
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        let raw: u32 = digits.parse().ok()?;
        (raw <= 5).then_some(raw)
    })
}

fn text_observation(
    key: &'static str,
    value: &str,
    source_id: &'static str,
    observed_at: DateTime<Utc>,
    sequence: u64,
) -> Observation {
    observation(key, Value::String(value.to_string()), source_id, observed_at, sequence)
}

fn number_observation(
    key: &'static str,
    value: u32,
    source_id: &'static str,
    observed_at: DateTime<Utc>,
    sequence: u64,
) -> Observation {
    observation(key, Value::from(value), source_id, observed_at, sequence)
}

fn observation(
    key: &'static str,
    value: Value,
    source_id: &'static str,
    observed_at: DateTime<Utc>,
    sequence: u64,
) -> Observation {
    Observation {
        schema: SCHEMA_VERSION,
        id: Uuid::new_v4(),
        subject: ObservationSubject::LocalDevice,
        key: key.into(),
        value,
        unit: None,
        source: ObservationSource {
            source_id: source_id.into(),
            kind: ObservationSourceKind::Direct,
        },
        quality: ObservationQuality::Direct,
        observed_at,
        valid_for_ms: CELLULAR_TTL_MS,
        sequence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample<'a>(name: &'a str, has_ipv4: bool, rx: u64, tx: u64) -> IfaceSample {
        IfaceSample {
            name: name.into(),
            has_ipv4,
            rx,
            tx,
        }
    }

    #[test]
    fn a_live_bearer_is_an_address_or_both_directions() {
        assert!(bearer_is_live(true, 0, 0));
        assert!(bearer_is_live(false, 1, 1));
        assert!(!bearer_is_live(false, 0, 1));
        assert!(!bearer_is_live(false, 1, 0));
    }

    #[test]
    fn publishes_the_three_live_facts_and_drops_stubs() {
        let reading = CellularReading {
            cp_text: Some("ONLINE\n".into()),
            owner_log: Some(
                "camp_reg field=voice registration_raw=3 reject_raw=0\n\
                 camp_reg field=data registration_raw=1 reject_raw=0 lac=0\n\
                 camp_reg field=selection mode_raw=0\n"
                    .into(),
            ),
            supervisor_log: None,
            owner_running: true,
            boot_epoch: None,
            endpoint: None,
            status_lock_busy: false,
            ifaces: vec![
                sample("usb0", true, 10, 10),
                sample("rmnet0", false, 0, 0),
                sample("rmnet1", true, 0, 0),
                sample("wlan0", true, 9, 9),
            ],
        };
        let rows = observations_from_cellular(&reading, Utc::now(), 4);
        assert_eq!(rows.len(), 8);
        let cp = rows.iter().find(|row| row.key == KEY_CP_STATE).unwrap();
        assert_eq!(cp.value, json!("ONLINE"));
        assert_eq!(cp.source.source_id, "sysfs.cpif.modem_state");
        let reg = rows
            .iter()
            .find(|row| row.key == KEY_REGISTRATION_RAW)
            .unwrap();
        assert_eq!(reg.value, json!(1));
        let voice = rows
            .iter()
            .find(|row| row.key == KEY_VOICE_REGISTRATION_RAW)
            .unwrap();
        assert_eq!(voice.value, json!(3));
        assert_eq!(voice.source.source_id, "camp.owner.voice_registration");
        let selection = rows.iter().find(|row| row.key == KEY_SELECTION).unwrap();
        assert_eq!(selection.value, json!("automatic"));
        assert!(rows.iter().all(|row| !row.value.to_string().contains("lac")));
        let bearer = rows.iter().find(|row| row.key == KEY_BEARER).unwrap();
        assert_eq!(bearer.value, json!("rmnet1"));
        let owner = rows.iter().find(|row| row.key == KEY_OWNER).unwrap();
        assert_eq!(owner.value, json!("running"));
        assert!(rows.iter().all(|row| row.validate().is_ok()));
    }

    #[test]
    fn missing_modem_publishes_only_that_the_owner_is_gone() {
        let reading = CellularReading {
            cp_text: None,
            owner_log: Some("camp_reg field=data registration_raw=1\n".into()),
            supervisor_log: None,
            owner_running: false,
            boot_epoch: None,
            endpoint: None,
            status_lock_busy: false,
            ifaces: vec![sample("rmnet2", false, 0, 40)],
        };
        let rows = observations_from_cellular(&reading, Utc::now(), 1);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].key, KEY_OWNER);
        assert_eq!(rows[0].value, json!("gone"));
        assert_eq!(rows[1].key, KEY_ACTION);
        assert_eq!(rows[1].value, json!("launch-once"));
        assert_eq!(rows[2].key, KEY_OPEN);
        assert_eq!(rows[2].value, json!("cp"));
    }

    #[test]
    fn online_without_registration_omits_the_registration_key() {
        let reading = CellularReading {
            cp_text: Some("ONLINE".into()),
            owner_log: Some("camp_reg field=radio radio_raw=1\n".into()),
            supervisor_log: None,
            owner_running: true,
            boot_epoch: None,
            endpoint: None,
            status_lock_busy: false,
            ifaces: Vec::new(),
        };
        let rows = observations_from_cellular(&reading, Utc::now(), 2);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].key, KEY_CP_STATE);
        assert_eq!(rows[1].key, KEY_OWNER);
        assert_eq!(rows[2].key, KEY_ACTION);
        assert_eq!(rows[2].value, json!("attend"));
        assert_eq!(rows[3].key, KEY_OPEN);
        assert_eq!(rows[3].value, json!("owner"));
    }

    #[test]
    fn supervisor_publishes_the_last_known_word_only() {
        let reading = CellularReading {
            cp_text: None,
            owner_log: None,
            supervisor_log: Some(
                "supervise=launch-once cp=missing\n\
                 supervise=handoff-exit code=0\n\
                 supervise=hold\n\
                 supervise=10.1.2.3\n"
                    .into(),
            ),
            owner_running: false,
            boot_epoch: None,
            endpoint: None,
            status_lock_busy: false,
            ifaces: Vec::new(),
        };
        let rows = observations_from_cellular(&reading, Utc::now(), 3);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].key, KEY_SUPERVISOR);
        assert_eq!(rows[0].value, json!("hold"));
        assert_eq!(rows[1].key, KEY_OWNER);
        assert_eq!(rows[1].value, json!("gone"));
        assert_eq!(rows[0].source.source_id, "camp.supervisor");
        assert_eq!(
            last_supervisor_token("supervise=cp-left\n"),
            Some("cp-left")
        );
    }

    #[test]
    fn radio_on_and_sim_ready_publish_stock_words_without_pin() {
        let reading = CellularReading {
            cp_text: None,
            owner_log: Some(
                "camp_reg field=radio radio_raw=10\n\
                 camp_reg field=radio radio_raw=1\n"
                    .into(),
            ),
            supervisor_log: None,
            owner_running: true,
            boot_epoch: None,
            endpoint: None,
            status_lock_busy: false,
            ifaces: Vec::new(),
        };
        let rows = observations_from_cellular(&reading, Utc::now(), 8);
        assert!(rows.iter().all(|row| row.key != KEY_RADIO));
        assert_eq!(rows.iter().filter(|row| row.key == KEY_OWNER).count(), 1);

        let reading = CellularReading {
            cp_text: Some("ONLINE".into()),
            owner_log: Some(
                "camp_probe field=sim card_raw=0 apps=0\n\
                 camp_reg field=radio radio_raw=10\n\
                 camp_probe field=sim card_raw=1 apps=1 app_state_raw=5 pin1_raw=3\n\
                 camp_sim=ready app_state_raw=5\n"
                    .into(),
            ),
            supervisor_log: None,
            owner_running: true,
            boot_epoch: None,
            endpoint: None,
            status_lock_busy: false,
            ifaces: Vec::new(),
        };
        let rows = observations_from_cellular(&reading, Utc::now(), 9);
        assert_eq!(rows.len(), 6);
        assert_eq!(rows[0].key, KEY_CP_STATE);
        assert_eq!(rows[1].key, KEY_RADIO);
        assert_eq!(rows[1].value, json!("on"));
        assert_eq!(rows[2].key, KEY_SIM_APP);
        assert_eq!(rows[2].value, json!("ready"));
        assert!(rows.iter().all(|row| !row.value.to_string().contains("pin")));
    }

    #[test]
    fn early_radio_line_survives_a_long_owner_log() {
        let mut log = "pad\n".repeat(80_000);
        log.push_str("camp_reg field=radio radio_raw=10\n");
        log.push_str(&"pad\n".repeat(80_000));
        log.push_str("camp_reg field=data registration_raw=1 reject_raw=0\n");
        let reading = CellularReading {
            cp_text: Some("ONLINE".into()),
            owner_log: Some(owner_fact_lines(&log)),
            supervisor_log: None,
            owner_running: true,
            boot_epoch: None,
            endpoint: None,
            status_lock_busy: false,
            ifaces: Vec::new(),
        };
        let rows = observations_from_cellular(&reading, Utc::now(), 4);
        assert_eq!(rows.len(), 6);
        assert_eq!(rows[0].key, KEY_CP_STATE);
        assert_eq!(rows[1].key, KEY_REGISTRATION_RAW);
        assert_eq!(rows[2].key, KEY_RADIO);
        assert_eq!(rows[2].value, json!("on"));
        assert_eq!(rows[3].key, KEY_OWNER);
    }

    #[test]
    fn offline_cp_drops_owner_log_facts_and_keeps_a_live_bearer() {
        let reading = CellularReading {
            cp_text: Some("OFFLINE\n".into()),
            owner_log: Some(
                "camp_reg field=data registration_raw=1\n\
                 camp_reg field=radio radio_raw=10\n\
                 camp_sim=ready\n"
                    .into(),
            ),
            supervisor_log: Some("supervise=hold\n".into()),
            owner_running: true,
            boot_epoch: None,
            endpoint: None,
            status_lock_busy: false,
            ifaces: vec![sample("rmnet1", true, 4, 4)],
        };
        let rows = observations_from_cellular(&reading, Utc::now(), 5);
        assert_eq!(rows.len(), 6);
        assert_eq!(rows[0].key, KEY_CP_STATE);
        assert_eq!(rows[0].value, json!("OFFLINE"));
        assert_eq!(rows[1].key, KEY_BEARER);
        assert_eq!(rows[1].value, json!("rmnet1"));
        assert_eq!(rows[2].key, KEY_SUPERVISOR);
        assert_eq!(rows[3].key, KEY_OWNER);
        assert_eq!(rows[3].value, json!("running"));
        assert!(rows.iter().all(|row| row.key != KEY_REGISTRATION_RAW));
        assert!(rows.iter().all(|row| row.key != KEY_RADIO));
        assert!(rows.iter().all(|row| row.key != KEY_SIM_APP));
    }

    #[test]
    fn boot_epoch_is_the_newest_numeric_archive_name() {
        assert_eq!(latest_numeric_epoch(["notes", "100", "250"]), Some(250));
        assert_eq!(latest_numeric_epoch(std::iter::empty()), None);
        let reading = CellularReading {
            cp_text: Some("OFFLINE".into()),
            owner_log: Some("camp_reg field=data registration_raw=1\n".into()),
            supervisor_log: None,
            owner_running: true,
            boot_epoch: Some(250),
            endpoint: None,
            status_lock_busy: false,
            ifaces: Vec::new(),
        };
        let rows = observations_from_cellular(&reading, Utc::now(), 6);
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].key, KEY_CP_STATE);
        assert_eq!(rows[1].key, KEY_OWNER);
        assert_eq!(rows[2].key, KEY_BOOT_EPOCH);
        assert_eq!(rows[2].value, json!(250));
        assert!(rows.iter().all(|row| row.key != KEY_REGISTRATION_RAW));
    }

    #[test]
    fn endpoint_word_names_who_holds_the_modem_device() {
        assert_eq!(endpoint_holder(true, false), Some("owner"));
        assert_eq!(endpoint_holder(false, true), Some("modemd"));
        assert_eq!(endpoint_holder(true, true), Some("shared"));
        assert_eq!(endpoint_holder(false, false), None);
        assert!(link_is_modem_endpoint("/dev/umts_ipc0"));
        assert!(link_is_modem_endpoint("/dev/umts_rfs0"));
        assert!(!link_is_modem_endpoint("/dev/umts_ipc0.bak"));
        assert!(!link_is_modem_endpoint("umts_ipc0"));
        let reading = CellularReading {
            cp_text: None,
            owner_log: None,
            supervisor_log: None,
            owner_running: true,
            boot_epoch: None,
            endpoint: Some("owner"),
            status_lock_busy: false,
            ifaces: Vec::new(),
        };
        let rows = observations_from_cellular(&reading, Utc::now(), 7);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].key, KEY_OWNER);
        assert_eq!(rows[1].key, KEY_ENDPOINT);
        assert_eq!(rows[1].value, json!("owner"));
        assert_eq!(rows[2].key, KEY_ACTION);
        assert_eq!(rows[2].value, json!("attend"));
        assert_eq!(rows[3].key, KEY_OPEN);
        assert_eq!(rows[3].value, json!("owner"));
        assert_eq!(camp_action(Some("OFFLINE"), false), "launch-once");
        assert_eq!(camp_open(Some("ONLINE"), true, Some("shared"), false), "owner");
        assert_eq!(camp_open(Some("ONLINE"), false, Some("modemd"), false), "endpoint");
        assert_eq!(camp_open(Some("ONLINE"), false, Some("pin"), true), "lock");
        assert_eq!(camp_open(Some("OFFLINE"), false, None, false), "cp");
        assert_eq!(camp_open(Some("ONLINE"), false, None, false), "ready");
        assert_eq!(
            last_selection_mode("camp_reg field=selection mode_raw=1\nfield=selection mode_raw=0\n"),
            Some("automatic")
        );
        assert_eq!(
            last_selection_mode("field=selection mode_raw=0\nfield=selection status=unknown_short\n"),
            None
        );
        assert_eq!(last_selection_mode("field=selection mode_raw=2\n"), None);
        assert_eq!(last_selection_mode("field=selection mode_raw=1\n"), Some("manual"));
        assert_eq!(camp_action(Some("ONLINE"), false), "attend");
        assert_eq!(camp_action(None, true), "attend");
    }
}
