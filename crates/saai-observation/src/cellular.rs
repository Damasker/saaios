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
pub const KEY_BEARER: &str = "cellular.bearer";
pub const KEY_SUPERVISOR: &str = "cellular.supervisor";
pub const KEY_RADIO: &str = "cellular.radio";
pub const KEY_SIM_APP: &str = "cellular.sim_app";

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
    if let Some(raw) = reading
        .owner_log
        .as_deref()
        .and_then(|log| last_data_registration_raw(log))
    {
        out.push(number_observation(
            KEY_REGISTRATION_RAW,
            raw,
            "camp.owner.data_registration",
            observed_at,
            sequence,
        ));
    }
    let mut live: Vec<&str> = reading
        .ifaces
        .iter()
        .filter(|iface| is_cellular_iface(&iface.name) && bearer_is_live(iface))
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
    if let Some(token) = reading.owner_log.as_deref().and_then(last_radio_token) {
        out.push(text_observation(
            KEY_RADIO,
            token,
            "camp.owner.radio",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = reading.owner_log.as_deref().and_then(last_sim_presence) {
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
    out
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

fn bearer_is_live(iface: &IfaceSample) -> bool {
    iface.has_ipv4 || (iface.rx > 0 && iface.tx > 0)
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

fn last_data_registration_raw(log: &str) -> Option<u32> {
    const NEEDLE: &str = "field=data registration_raw=";
    log.lines().rev().find_map(|line| {
        let rest = line.split_once(NEEDLE)?.1;
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
    fn publishes_the_three_live_facts_and_drops_stubs() {
        let reading = CellularReading {
            cp_text: Some("ONLINE\n".into()),
            owner_log: Some(
                "camp_reg field=voice registration_raw=3 reject_raw=0\n\
                 camp_reg field=data registration_raw=1 reject_raw=0 lac=0\n"
                    .into(),
            ),
            supervisor_log: None,
            ifaces: vec![
                sample("usb0", true, 10, 10),
                sample("rmnet0", false, 0, 0),
                sample("rmnet1", true, 0, 0),
                sample("wlan0", true, 9, 9),
            ],
        };
        let rows = observations_from_cellular(&reading, Utc::now(), 4);
        assert_eq!(rows.len(), 3);
        let cp = rows.iter().find(|row| row.key == KEY_CP_STATE).unwrap();
        assert_eq!(cp.value, json!("ONLINE"));
        assert_eq!(cp.source.source_id, "sysfs.cpif.modem_state");
        let reg = rows
            .iter()
            .find(|row| row.key == KEY_REGISTRATION_RAW)
            .unwrap();
        assert_eq!(reg.value, json!(1));
        let bearer = rows.iter().find(|row| row.key == KEY_BEARER).unwrap();
        assert_eq!(bearer.value, json!("rmnet1"));
        assert!(rows.iter().all(|row| row.validate().is_ok()));
    }

    #[test]
    fn missing_modem_publishes_nothing() {
        let reading = CellularReading {
            cp_text: None,
            owner_log: None,
            supervisor_log: None,
            ifaces: vec![sample("rmnet2", false, 0, 40)],
        };
        assert!(observations_from_cellular(&reading, Utc::now(), 1).is_empty());
    }

    #[test]
    fn online_without_registration_omits_the_registration_key() {
        let reading = CellularReading {
            cp_text: Some("ONLINE".into()),
            owner_log: Some("camp_reg field=radio radio_raw=1\n".into()),
            supervisor_log: None,
            ifaces: Vec::new(),
        };
        let rows = observations_from_cellular(&reading, Utc::now(), 2);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].key, KEY_CP_STATE);
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
            ifaces: Vec::new(),
        };
        let rows = observations_from_cellular(&reading, Utc::now(), 3);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].key, KEY_SUPERVISOR);
        assert_eq!(rows[0].value, json!("hold"));
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
            ifaces: Vec::new(),
        };
        let rows = observations_from_cellular(&reading, Utc::now(), 8);
        assert!(rows.is_empty(), "a later radio_raw other than 10 hides the radio");

        let reading = CellularReading {
            cp_text: None,
            owner_log: Some(
                "camp_probe field=sim card_raw=0 apps=0\n\
                 camp_reg field=radio radio_raw=10\n\
                 camp_probe field=sim card_raw=1 apps=1 app_state_raw=5 pin1_raw=3\n\
                 camp_sim=ready app_state_raw=5\n"
                    .into(),
            ),
            supervisor_log: None,
            ifaces: Vec::new(),
        };
        let rows = observations_from_cellular(&reading, Utc::now(), 9);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].key, KEY_RADIO);
        assert_eq!(rows[0].value, json!("on"));
        assert_eq!(rows[1].key, KEY_SIM_APP);
        assert_eq!(rows[1].value, json!("ready"));
        assert!(rows.iter().all(|row| !row.value.to_string().contains("pin")));
    }
}
