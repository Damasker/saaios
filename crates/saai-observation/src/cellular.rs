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
pub const KEY_STACK: &str = "cellular.stack";
pub const KEY_DEVICE_SERVICE: &str = "cellular.device_service";
pub const KEY_VOICE_OPERATION: &str = "cellular.voice_operation";
pub const KEY_ALLOW_DATA: &str = "cellular.allow_data";
pub const KEY_INITIAL_ATTACH: &str = "cellular.initial_attach";
pub const KEY_DNS: &str = "cellular.dns";
pub const KEY_DNS6: &str = "cellular.dns6";
pub const KEY_CONFIG: &str = "cellular.config";
pub const KEY_SGC: &str = "cellular.sgc";
pub const KEY_POWER: &str = "cellular.power";
pub const KEY_VOICE_SET: &str = "cellular.voice_set";
pub const KEY_IPV4: &str = "cellular.ipv4";
pub const KEY_IPV6: &str = "cellular.ipv6";
pub const KEY_SETUP: &str = "cellular.setup";
pub const KEY_PROFILE: &str = "cellular.profile";
pub const KEY_ACTIVITY: &str = "cellular.activity";
pub const KEY_FASTDORM: &str = "cellular.fastdorm";
pub const KEY_ENDC: &str = "cellular.endc";
pub const KEY_THROTTLE: &str = "cellular.throttle";
pub const KEY_UNSOLFF: &str = "cellular.unsolff";
pub const KEY_UNSOL: &str = "cellular.unsol";
pub const KEY_SCREEN: &str = "cellular.screen";
pub const KEY_CELLINFO: &str = "cellular.cellinfo";
pub const KEY_SMSC: &str = "cellular.smsc";
pub const KEY_VONRGET: &str = "cellular.vonrget";
pub const KEY_APTIME: &str = "cellular.aptime";
pub const KEY_DBGTRACE: &str = "cellular.dbgtrace";
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
    if let Some(token) = log.and_then(last_stack_mode) {
        out.push(text_observation(
            KEY_STACK,
            token,
            "camp.owner.stack",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_device_service) {
        out.push(text_observation(
            KEY_DEVICE_SERVICE,
            token,
            "camp.owner.device_service",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_voice_operation) {
        out.push(text_observation(
            KEY_VOICE_OPERATION,
            token,
            "camp.owner.voice_operation",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_allow_data) {
        out.push(text_observation(
            KEY_ALLOW_DATA,
            token,
            "camp.owner.allow_data",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_initial_attach) {
        out.push(text_observation(
            KEY_INITIAL_ATTACH,
            token,
            "camp.owner.initial_attach",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_dns) {
        out.push(text_observation(
            KEY_DNS,
            token,
            "camp.owner.dns",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_dns6) {
        out.push(text_observation(
            KEY_DNS6,
            token,
            "camp.owner.dns6",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_modem_config) {
        out.push(text_observation(
            KEY_CONFIG,
            token,
            "camp.owner.config",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_sgc) {
        out.push(text_observation(
            KEY_SGC,
            token,
            "camp.owner.sgc",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_radio_power) {
        out.push(text_observation(
            KEY_POWER,
            token,
            "camp.owner.power",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_voice_set) {
        out.push(text_observation(
            KEY_VOICE_SET,
            token,
            "camp.owner.voice_set",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_ipv4) {
        out.push(text_observation(
            KEY_IPV4,
            token,
            "camp.owner.ipv4",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_ipv6) {
        out.push(text_observation(
            KEY_IPV6,
            token,
            "camp.owner.ipv6",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_data_setup) {
        out.push(text_observation(
            KEY_SETUP,
            token,
            "camp.owner.setup",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_data_profile) {
        out.push(text_observation(
            KEY_PROFILE,
            token,
            "camp.owner.profile",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_activity) {
        out.push(text_observation(
            KEY_ACTIVITY,
            token,
            "camp.owner.activity",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_fastdorm) {
        out.push(text_observation(
            KEY_FASTDORM,
            token,
            "camp.owner.fastdorm",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_endc) {
        out.push(text_observation(
            KEY_ENDC,
            token,
            "camp.owner.endc",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_throttle) {
        out.push(text_observation(
            KEY_THROTTLE,
            token,
            "camp.owner.throttle",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_unsolff) {
        out.push(text_observation(
            KEY_UNSOLFF,
            token,
            "camp.owner.unsolff",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_unsol) {
        out.push(text_observation(
            KEY_UNSOL,
            token,
            "camp.owner.unsol",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_screen) {
        out.push(text_observation(
            KEY_SCREEN,
            token,
            "camp.owner.screen",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_cellinfo) {
        out.push(text_observation(
            KEY_CELLINFO,
            token,
            "camp.owner.cellinfo",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_smsc) {
        out.push(text_observation(
            KEY_SMSC,
            token,
            "camp.owner.smsc",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_vonrget) {
        out.push(text_observation(
            KEY_VONRGET,
            token,
            "camp.owner.vonrget",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_aptime) {
        out.push(text_observation(
            KEY_APTIME,
            token,
            "camp.owner.aptime",
            observed_at,
            sequence,
        ));
    }
    if let Some(token) = log.and_then(last_dbgtrace) {
        out.push(text_observation(
            KEY_DBGTRACE,
            token,
            "camp.owner.dbgtrace",
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
    let mut stack = None;
    let mut device_service = None;
    let mut voice_operation = None;
    let mut allow_data = None;
    let mut initial_attach = None;
    let mut dns = None;
    let mut dns6 = None;
    let mut config = None;
    let mut sgc = None;
    let mut power = None;
    let mut voice_set = None;
    let mut ipv4 = None;
    let mut ipv6 = None;
    let mut setup = None;
    let mut profile = None;
    let mut activity = None;
    let mut fastdorm = None;
    let mut endc = None;
    let mut throttle = None;
    let mut unsolff = None;
    let mut unsol = None;
    let mut screen = None;
    let mut cellinfo = None;
    let mut smsc = None;
    let mut vonrget = None;
    let mut aptime = None;
    let mut dbgtrace = None;
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
        } else if line.contains("get=stack_status ") {
            stack = Some(line);
        } else if line.contains("get=device_service ") {
            device_service = Some(line);
        } else if line.contains("get=voice_operation ") {
            voice_operation = Some(line);
        } else if line.contains("set=allow_data ") {
            allow_data = Some(line);
        } else if line.contains("set=initial_attach_apn ") {
            initial_attach = Some(line);
        } else if line.contains("camp_setup dns6=") {
            dns6 = Some(line);
        } else if line.contains("camp_setup dns=") {
            dns = Some(line);
        } else if line.contains("cmd=0x093f ") {
            config = Some(line);
        } else if line.contains("cmd=0x0404 ") {
            sgc = Some(line);
        } else if line.contains("cmd=0x0800 ") {
            power = Some(line);
        } else if line.contains("set=set_voice_operation ") {
            voice_set = Some(line);
        } else if line.contains("ipv6=yes prefix=64 ") {
            ipv6 = Some(line);
        } else if line.contains("ipv4=yes prefix=32 ") {
            ipv4 = Some(line);
        } else if line.contains("camp_setup response=yes ") {
            setup = Some(line);
        } else if line.contains("camp_profile response=yes ") {
            profile = Some(line);
        } else if line.contains("camp_activity response=yes ") {
            activity = Some(line);
        } else if line.contains("camp_fastdorm response=yes ") {
            fastdorm = Some(line);
        } else if line.contains("camp_endc response=yes ") {
            endc = Some(line);
        } else if line.contains("camp_throttle response=yes ") {
            throttle = Some(line);
        } else if line.contains("camp_unsolff response=yes ") {
            unsolff = Some(line);
        } else if line.contains("camp_unsol response=yes ") {
            unsol = Some(line);
        } else if line.contains("camp_screen response=yes ") {
            screen = Some(line);
        } else if line.contains("camp_cellinfo response=yes ") {
            cellinfo = Some(line);
        } else if line.contains("camp_smsc response=yes ") {
            smsc = Some(line);
        } else if line.contains("camp_vonrget response=yes ") {
            vonrget = Some(line);
        } else if line.contains("camp_aptime response=yes ") {
            aptime = Some(line);
        } else if line.contains("camp_dbgtrace response=yes ") {
            dbgtrace = Some(line);
        } else if line.contains("camp_sim=") || line.contains("field=sim ") {
            sim = Some(line);
        }
    }
    let mut out = String::new();
    for line in [
        data,
        voice,
        radio,
        selection,
        stack,
        device_service,
        voice_operation,
        allow_data,
        initial_attach,
        dns,
        dns6,
        config,
        sgc,
        power,
        voice_set,
        ipv4,
        ipv6,
        setup,
        profile,
        activity,
        fastdorm,
        endc,
        throttle,
        unsolff,
        unsol,
        screen,
        cellinfo,
        smsc,
        vonrget,
        aptime,
        dbgtrace,
        sim,
    ]
        .into_iter()
        .flatten()
    {
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

/// Last debug-trace acknowledgement. Stock error `0` is accepted.
/// The trace body stays out. The AP-time read is a different fact.
pub fn last_dbgtrace(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some((_, rest)) = line.split_once("camp_dbgtrace response=yes error_raw=") {
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            return match digits.parse::<u32>().ok() {
                Some(0) => Some("accepted"),
                _ => None,
            };
        }
    }
    None
}

/// Last AP-time acknowledgement. Stock error `0` is accepted.
/// The clock and any duration stay out. The VoNR read is a different fact.
pub fn last_aptime(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some((_, rest)) = line.split_once("camp_aptime response=yes error_raw=") {
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            return match digits.parse::<u32>().ok() {
                Some(0) => Some("accepted"),
                _ => None,
            };
        }
    }
    None
}

/// Last VoNR-capability read acknowledgement. Stock error `0` is accepted.
/// The capability stays out. The SMSC read and the VoNR set are different facts.
pub fn last_vonrget(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some((_, rest)) = line.split_once("camp_vonrget response=yes error_raw=") {
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            return match digits.parse::<u32>().ok() {
                Some(0) => Some("accepted"),
                _ => None,
            };
        }
    }
    None
}

/// Last SMSC-address read acknowledgement. Stock error `0` is accepted.
/// The address stays out. The cell-info list and the VoNR read are different facts.
pub fn last_smsc(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some((_, rest)) = line.split_once("camp_smsc response=yes error_raw=") {
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            return match digits.parse::<u32>().ok() {
                Some(0) => Some("accepted"),
                _ => None,
            };
        }
    }
    None
}

/// Last cell-info list acknowledgement. Stock error `0` is accepted.
/// The list stays out. The SMSC read and the screen state are different facts.
pub fn last_cellinfo(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some((_, rest)) = line.split_once("camp_cellinfo response=yes error_raw=") {
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            return match digits.parse::<u32>().ok() {
                Some(0) => Some("accepted"),
                _ => None,
            };
        }
    }
    None
}

/// Last screen-state acknowledgement. Stock error `0` is accepted.
/// The screen state stays out. The indication filters are different facts.
pub fn last_screen(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some((_, rest)) = line.split_once("camp_screen response=yes error_raw=") {
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            return match digits.parse::<u32>().ok() {
                Some(0) => Some("accepted"),
                _ => None,
            };
        }
    }
    None
}

/// Last settled indication-filter acknowledgement. Stock error `0` is accepted.
/// The filter word stays out. The wide filter and screen state are different facts.
pub fn last_unsol(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some((_, rest)) = line.split_once("camp_unsol response=yes error_raw=") {
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            return match digits.parse::<u32>().ok() {
                Some(0) => Some("accepted"),
                _ => None,
            };
        }
    }
    None
}

/// Last wide indication-filter acknowledgement. Stock error `0` is accepted.
/// The filter word stays out. The settled filter and screen state are different facts.
pub fn last_unsolff(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some((_, rest)) = line.split_once("camp_unsolff response=yes error_raw=") {
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            return match digits.parse::<u32>().ok() {
                Some(0) => Some("accepted"),
                _ => None,
            };
        }
    }
    None
}

/// Last data-throttling acknowledgement. Stock error `0` is accepted.
/// The duration stays out. Screen state and the indication filter are different facts.
pub fn last_throttle(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some((_, rest)) = line.split_once("camp_throttle response=yes error_raw=") {
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            return match digits.parse::<u32>().ok() {
                Some(0) => Some("accepted"),
                _ => None,
            };
        }
    }
    None
}

/// Last ENDC-mode read acknowledgement. Stock error `0` is accepted.
/// The mode stays out. VoNR capability and the RC network type are different facts.
pub fn last_endc(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some((_, rest)) = line.split_once("camp_endc response=yes error_raw=") {
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            return match digits.parse::<u32>().ok() {
                Some(0) => Some("accepted"),
                _ => None,
            };
        }
    }
    None
}

/// Last fast-dormancy acknowledgement. Stock error `0` is accepted.
/// The reply length stays out. Activity is a different fact.
pub fn last_fastdorm(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some((_, rest)) = line.split_once("camp_fastdorm response=yes error_raw=") {
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            return match digits.parse::<u32>().ok() {
                Some(0) => Some("accepted"),
                _ => None,
            };
        }
    }
    None
}

/// Last modem-activity acknowledgement. Stock error `0` is accepted.
/// The reply length stays out. Profile and fast-dormancy are different facts.
pub fn last_activity(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some((_, rest)) = line.split_once("camp_activity response=yes error_raw=") {
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            return match digits.parse::<u32>().ok() {
                Some(0) => Some("accepted"),
                _ => None,
            };
        }
    }
    None
}

/// Last internet data-profile acknowledgement. Stock error `0` is accepted.
/// The profile body stays out. Setup, IMS, and SOS are different facts.
pub fn last_data_profile(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some((_, rest)) = line.split_once("camp_profile response=yes error_raw=") {
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            return match digits.parse::<u32>().ok() {
                Some(0) => Some("accepted"),
                _ => None,
            };
        }
    }
    None
}

/// Last SetupDataCall acknowledgement. Stock error `0` is accepted.
/// The address stays out. The IPv4 and IPv6 apply lines are different facts.
pub fn last_data_setup(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some((_, rest)) = line.split_once("camp_setup response=yes error_raw=") {
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            return match digits.parse::<u32>().ok() {
                Some(0) => Some("accepted"),
                _ => None,
            };
        }
    }
    None
}

/// Last IPv6 apply. `yes` only when link, address, and route all succeeded.
/// The address stays out. The IPv4 line is a different fact.
pub fn last_ipv6(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some((_, rest)) = line.split_once("ipv6=yes prefix=64 up=") {
            return match apply_steps(rest) {
                Some(true) => Some("yes"),
                Some(false) => Some("no"),
                None => None,
            };
        }
    }
    None
}

/// Last IPv4 apply. `yes` only when link, address, and route all succeeded.
/// The address stays out. The IPv6 line is a different fact.
pub fn last_ipv4(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some((_, rest)) = line.split_once("ipv4=yes prefix=32 up=") {
            return match apply_steps(rest) {
                Some(true) => Some("yes"),
                Some(false) => Some("no"),
                None => None,
            };
        }
    }
    None
}

fn apply_steps(rest: &str) -> Option<bool> {
    let (up, rest) = rest.split_once(" add=")?;
    let (add, rest) = rest.split_once(" route=")?;
    let up = up.parse::<u32>().ok()?;
    let add = add.parse::<u32>().ok()?;
    let route: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    let route = route.parse::<u32>().ok()?;
    if up > 1 || add > 1 || route > 1 {
        return None;
    }
    Some(up == 1 && add == 1 && route == 1)
}

/// Last voice-operation SET acknowledgement. Stock error `0` is accepted.
/// The GET mode stays on its own fact.
pub fn last_voice_set(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some(rest) =
            line.split_once("camp_opx set=set_voice_operation response=yes error_raw=")
        {
            let digits: String = rest.1.chars().take_while(|c| c.is_ascii_digit()).collect();
            return match digits.parse::<u32>().ok() {
                Some(0) => Some("accepted"),
                _ => None,
            };
        }
    }
    None
}

/// Last radio-power acknowledgement. Stock `BuildRadioPower` error `0`
/// is accepted. The power word stays out.
pub fn last_radio_power(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some(rest) = line.split_once("cmd=0x0800 response=yes error_raw=") {
            let digits: String = rest.1.chars().take_while(|c| c.is_ascii_digit()).collect();
            return match digits.parse::<u32>().ok() {
                Some(0) => Some("accepted"),
                _ => None,
            };
        }
    }
    None
}

/// Last SGC acknowledgement. Stock `SendSGCValue` error `0` is accepted.
/// The carried value stays out.
pub fn last_sgc(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some(rest) = line.split_once("cmd=0x0404 response=yes error_raw=") {
            let digits: String = rest.1.chars().take_while(|c| c.is_ascii_digit()).collect();
            return match digits.parse::<u32>().ok() {
                Some(0) => Some("accepted"),
                _ => None,
            };
        }
    }
    None
}

/// Last modem-config acknowledgement. Stock error `0` is accepted.
pub fn last_modem_config(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some(rest) = line.split_once("cmd=0x093f response=yes error_raw=") {
            let digits: String = rest.1.chars().take_while(|c| c.is_ascii_digit()).collect();
            return match digits.parse::<u32>().ok() {
                Some(0) => Some("accepted"),
                _ => None,
            };
        }
    }
    None
}

/// Last IPv6 resolver fact. The owner names `yes` or `no`. Addresses stay out.
pub fn last_dns6(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some(rest) = line.split_once("camp_setup dns6=") {
            let word: String = rest.1.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
            return match word.as_str() {
                "yes" => Some("yes"),
                "no" => Some("no"),
                _ => None,
            };
        }
    }
    None
}

/// Last IPv4 resolver fact. The owner names `yes` or `no`. Addresses stay out.
pub fn last_dns(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some(rest) = line.split_once("camp_setup dns=") {
            let word: String = rest.1.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
            return match word.as_str() {
                "yes" => Some("yes"),
                "no" => Some("no"),
                _ => None,
            };
        }
    }
    None
}

/// Last initial-attach result. Stock error `0` is accepted. Any other code stays out.
/// The access-point name stays out of the log line and out of this word.
pub fn last_initial_attach(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some(rest) = line.split_once("set=initial_attach_apn response=yes error_raw=") {
            let digits: String = rest.1.chars().take_while(|c| c.is_ascii_digit()).collect();
            return match digits.parse::<u32>().ok() {
                Some(0) => Some("accepted"),
                _ => None,
            };
        }
    }
    None
}

/// Last allow-data result. Stock error `0` is accepted. Any other code stays out.
pub fn last_allow_data(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some(rest) = line.split_once("set=allow_data response=yes error_raw=") {
            let digits: String = rest.1.chars().take_while(|c| c.is_ascii_digit()).collect();
            return match digits.parse::<u32>().ok() {
                Some(0) => Some("accepted"),
                _ => None,
            };
        }
    }
    None
}

/// Last voice-operation mode. Stock names `3` enabled. Anything else stays out.
pub fn last_voice_operation(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some(rest) = line.split_once("get=voice_operation mode_raw=") {
            let digits: String = rest.1.chars().take_while(|c| c.is_ascii_digit()).collect();
            return match digits.parse::<u32>().ok() {
                Some(3) => Some("enabled"),
                _ => None,
            };
        }
        if line.contains("get=voice_operation status=unknown_short") {
            return None;
        }
    }
    None
}

/// Last device-service mode. Stock names `1` voice-centric and `2` data-centric.
pub fn last_device_service(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some(rest) = line.split_once("get=device_service mode_raw=") {
            let digits: String = rest.1.chars().take_while(|c| c.is_ascii_digit()).collect();
            return match digits.parse::<u32>().ok() {
                Some(1) => Some("voice-centric"),
                Some(2) => Some("data-centric"),
                _ => None,
            };
        }
        if line.contains("get=device_service status=unknown_short") {
            return None;
        }
    }
    None
}

/// Last logical-stack mode. Stock reads byte +12: `0` is disabled, `1` is enabled.
pub fn last_stack_mode(log: &str) -> Option<&'static str> {
    for line in log.lines().rev() {
        if let Some(rest) = line.split_once("get=stack_status mode_raw=") {
            let digits: String = rest.1.chars().take_while(|c| c.is_ascii_digit()).collect();
            return match digits.parse::<u32>().ok() {
                Some(0) => Some("disabled"),
                Some(1) => Some("enabled"),
                _ => None,
            };
        }
        if line.contains("get=stack_status status=unknown_short") {
            return None;
        }
    }
    None
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
                 camp_reg field=selection mode_raw=0\n\
                 camp_opx get=stack_status mode_raw=1\n\
                 camp_opx get=device_service mode_raw=1\n\
                 camp_opx get=voice_operation mode_raw=3\n\
                 camp_reg set=allow_data response=yes error_raw=0\n\
                 camp_reg set=initial_attach_apn response=yes error_raw=0\n\
                 camp_setup dns=yes count=2\n\
                 camp_setup dns6=yes count=2\n\
                 camp_ack cmd=0x093f response=yes error_raw=0\n\
                 camp_ack cmd=0x0404 response=yes error_raw=0\n\
                 camp_ack cmd=0x0800 response=yes error_raw=0\n\
                 camp_opx set=set_voice_operation response=yes error_raw=0\n\
                 camp_setup if=rmnet1 ipv4=yes prefix=32 up=1 add=1 route=1\n\
                 camp_setup if=rmnet1 ipv6=yes prefix=64 up=1 add=1 route=1\n\
                 camp_setup response=yes error_raw=0 len=100\n\
                 camp_profile response=yes error_raw=0 len=40\n\
                 camp_activity response=yes error_raw=0 len=16\n\
                 camp_fastdorm response=yes error_raw=0 len=12\n\
                 camp_endc response=yes error_raw=0 len=16\n\
                 camp_throttle response=yes error_raw=0 len=16\n\
                 camp_unsolff response=yes error_raw=0 len=16\n\
                 camp_unsol response=yes error_raw=0 len=16\n\
                 camp_screen response=yes error_raw=0 len=16\n\
                 camp_cellinfo response=yes error_raw=0 len=16\n\
                 camp_smsc response=yes error_raw=0 len=16\n\
                 camp_vonrget response=yes error_raw=0 len=16\n\
                 camp_aptime response=yes error_raw=0 len=16\n\
                 camp_dbgtrace response=yes error_raw=0 len=16\n"
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
        assert_eq!(rows.len(), 35);
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
        let stack = rows.iter().find(|row| row.key == KEY_STACK).unwrap();
        assert_eq!(stack.value, json!("enabled"));
        assert_eq!(stack.source.source_id, "camp.owner.stack");
        let service = rows.iter().find(|row| row.key == KEY_DEVICE_SERVICE).unwrap();
        assert_eq!(service.value, json!("voice-centric"));
        assert_eq!(service.source.source_id, "camp.owner.device_service");
        let operation = rows.iter().find(|row| row.key == KEY_VOICE_OPERATION).unwrap();
        assert_eq!(operation.value, json!("enabled"));
        assert_eq!(operation.source.source_id, "camp.owner.voice_operation");
        let allow = rows.iter().find(|row| row.key == KEY_ALLOW_DATA).unwrap();
        assert_eq!(allow.value, json!("accepted"));
        assert_eq!(allow.source.source_id, "camp.owner.allow_data");
        let attach = rows.iter().find(|row| row.key == KEY_INITIAL_ATTACH).unwrap();
        assert_eq!(attach.value, json!("accepted"));
        assert_eq!(attach.source.source_id, "camp.owner.initial_attach");
        let dns = rows.iter().find(|row| row.key == KEY_DNS).unwrap();
        assert_eq!(dns.value, json!("yes"));
        assert_eq!(dns.source.source_id, "camp.owner.dns");
        let dns6 = rows.iter().find(|row| row.key == KEY_DNS6).unwrap();
        assert_eq!(dns6.value, json!("yes"));
        assert_eq!(dns6.source.source_id, "camp.owner.dns6");
        let config = rows.iter().find(|row| row.key == KEY_CONFIG).unwrap();
        assert_eq!(config.value, json!("accepted"));
        assert_eq!(config.source.source_id, "camp.owner.config");
        let sgc = rows.iter().find(|row| row.key == KEY_SGC).unwrap();
        assert_eq!(sgc.value, json!("accepted"));
        assert_eq!(sgc.source.source_id, "camp.owner.sgc");
        let power = rows.iter().find(|row| row.key == KEY_POWER).unwrap();
        assert_eq!(power.value, json!("accepted"));
        assert_eq!(power.source.source_id, "camp.owner.power");
        let voice_set = rows.iter().find(|row| row.key == KEY_VOICE_SET).unwrap();
        assert_eq!(voice_set.value, json!("accepted"));
        assert_eq!(voice_set.source.source_id, "camp.owner.voice_set");
        let ipv4 = rows.iter().find(|row| row.key == KEY_IPV4).unwrap();
        assert_eq!(ipv4.value, json!("yes"));
        assert_eq!(ipv4.source.source_id, "camp.owner.ipv4");
        let ipv6 = rows.iter().find(|row| row.key == KEY_IPV6).unwrap();
        assert_eq!(ipv6.value, json!("yes"));
        assert_eq!(ipv6.source.source_id, "camp.owner.ipv6");
        let setup = rows.iter().find(|row| row.key == KEY_SETUP).unwrap();
        assert_eq!(setup.value, json!("accepted"));
        assert_eq!(setup.source.source_id, "camp.owner.setup");
        let profile = rows.iter().find(|row| row.key == KEY_PROFILE).unwrap();
        assert_eq!(profile.value, json!("accepted"));
        assert_eq!(profile.source.source_id, "camp.owner.profile");
        let activity = rows.iter().find(|row| row.key == KEY_ACTIVITY).unwrap();
        assert_eq!(activity.value, json!("accepted"));
        assert_eq!(activity.source.source_id, "camp.owner.activity");
        let fastdorm = rows.iter().find(|row| row.key == KEY_FASTDORM).unwrap();
        assert_eq!(fastdorm.value, json!("accepted"));
        assert_eq!(fastdorm.source.source_id, "camp.owner.fastdorm");
        let endc = rows.iter().find(|row| row.key == KEY_ENDC).unwrap();
        assert_eq!(endc.value, json!("accepted"));
        assert_eq!(endc.source.source_id, "camp.owner.endc");
        let throttle = rows.iter().find(|row| row.key == KEY_THROTTLE).unwrap();
        assert_eq!(throttle.value, json!("accepted"));
        assert_eq!(throttle.source.source_id, "camp.owner.throttle");
        let unsolff = rows.iter().find(|row| row.key == KEY_UNSOLFF).unwrap();
        assert_eq!(unsolff.value, json!("accepted"));
        assert_eq!(unsolff.source.source_id, "camp.owner.unsolff");
        let unsol = rows.iter().find(|row| row.key == KEY_UNSOL).unwrap();
        assert_eq!(unsol.value, json!("accepted"));
        assert_eq!(unsol.source.source_id, "camp.owner.unsol");
        let screen = rows.iter().find(|row| row.key == KEY_SCREEN).unwrap();
        assert_eq!(screen.value, json!("accepted"));
        assert_eq!(screen.source.source_id, "camp.owner.screen");
        let cellinfo = rows.iter().find(|row| row.key == KEY_CELLINFO).unwrap();
        assert_eq!(cellinfo.value, json!("accepted"));
        assert_eq!(cellinfo.source.source_id, "camp.owner.cellinfo");
        let smsc = rows.iter().find(|row| row.key == KEY_SMSC).unwrap();
        assert_eq!(smsc.value, json!("accepted"));
        assert_eq!(smsc.source.source_id, "camp.owner.smsc");
        let vonrget = rows.iter().find(|row| row.key == KEY_VONRGET).unwrap();
        assert_eq!(vonrget.value, json!("accepted"));
        assert_eq!(vonrget.source.source_id, "camp.owner.vonrget");
        let aptime = rows.iter().find(|row| row.key == KEY_APTIME).unwrap();
        assert_eq!(aptime.value, json!("accepted"));
        assert_eq!(aptime.source.source_id, "camp.owner.aptime");
        let dbgtrace = rows.iter().find(|row| row.key == KEY_DBGTRACE).unwrap();
        assert_eq!(dbgtrace.value, json!("accepted"));
        assert_eq!(dbgtrace.source.source_id, "camp.owner.dbgtrace");
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
        assert_eq!(
            last_stack_mode("camp_opx get=stack_status mode_raw=0\n"),
            Some("disabled")
        );
        assert_eq!(
            last_stack_mode("camp_opx get=stack_status mode_raw=1\n"),
            Some("enabled")
        );
        assert_eq!(last_stack_mode("camp_opx get=stack_status mode_raw=2\n"), None);
        assert_eq!(
            last_device_service("camp_opx get=device_service mode_raw=1\n"),
            Some("voice-centric")
        );
        assert_eq!(
            last_device_service("camp_opx get=device_service mode_raw=2\n"),
            Some("data-centric")
        );
        assert_eq!(last_device_service("camp_opx get=device_service mode_raw=0\n"), None);
        assert_eq!(
            last_voice_operation("camp_opx get=voice_operation mode_raw=3\n"),
            Some("enabled")
        );
        assert_eq!(last_voice_operation("camp_opx get=voice_operation mode_raw=1\n"), None);
        assert_eq!(
            last_voice_operation(
                "camp_opx get=voice_operation mode_raw=3\nget=voice_operation status=unknown_short\n"
            ),
            None
        );
        assert_eq!(
            last_allow_data("camp_reg set=allow_data response=yes error_raw=0\n"),
            Some("accepted")
        );
        assert_eq!(
            last_allow_data("camp_reg set=allow_data response=yes error_raw=2\n"),
            None
        );
        assert_eq!(
            last_allow_data(
                "camp_reg set=allow_data response=yes error_raw=0\n\
                 camp_reg set=allow_data response=yes error_raw=1\n"
            ),
            None
        );
        assert_eq!(
            last_initial_attach(
                "camp_reg set=initial_attach_apn response=yes error_raw=0\n"
            ),
            Some("accepted")
        );
        assert_eq!(
            last_initial_attach(
                "camp_reg set=initial_attach_apn response=yes error_raw=2\n"
            ),
            None
        );
        assert_eq!(
            last_initial_attach(
                "camp_reg set=initial_attach_apn response=yes error_raw=0\n\
                 camp_reg set=initial_attach_apn response=yes error_raw=1\n"
            ),
            None
        );
        assert_eq!(last_dns("camp_setup dns=yes count=2\n"), Some("yes"));
        assert_eq!(last_dns("camp_setup dns=no\n"), Some("no"));
        assert_eq!(last_dns("camp_setup dns6=yes count=2\n"), None);
        assert_eq!(
            last_dns("camp_setup dns=yes count=2\ncamp_setup dns=no\n"),
            Some("no")
        );
        assert_eq!(last_dns6("camp_setup dns6=yes count=2\n"), Some("yes"));
        assert_eq!(last_dns6("camp_setup dns6=no\n"), Some("no"));
        assert_eq!(last_dns6("camp_setup dns=yes count=2\n"), None);
        assert_eq!(
            last_dns6("camp_setup dns6=yes count=2\ncamp_setup dns6=no\n"),
            Some("no")
        );
        assert_eq!(
            last_modem_config("camp_ack cmd=0x093f response=yes error_raw=0\n"),
            Some("accepted")
        );
        assert_eq!(
            last_modem_config("camp_ack cmd=0x093f response=yes error_raw=2\n"),
            None
        );
        assert_eq!(
            last_modem_config("camp_ack cmd=0x0404 response=yes error_raw=0\n"),
            None
        );
        assert_eq!(
            last_modem_config(
                "camp_ack cmd=0x093f response=yes error_raw=0\n\
                 camp_ack cmd=0x093f response=yes error_raw=1\n"
            ),
            None
        );
        assert_eq!(
            last_sgc("camp_ack cmd=0x0404 response=yes error_raw=0\n"),
            Some("accepted")
        );
        assert_eq!(
            last_sgc("camp_ack cmd=0x0404 response=yes error_raw=2\n"),
            None
        );
        assert_eq!(
            last_sgc("camp_ack cmd=0x093f response=yes error_raw=0\n"),
            None
        );
        assert_eq!(
            last_sgc("camp_ack cmd=0x0800 response=yes error_raw=0\n"),
            None
        );
        assert_eq!(
            last_sgc(
                "camp_ack cmd=0x0404 response=yes error_raw=0\n\
                 camp_ack cmd=0x0404 response=yes error_raw=1\n"
            ),
            None
        );
        assert_eq!(
            last_radio_power("camp_ack cmd=0x0800 response=yes error_raw=0\n"),
            Some("accepted")
        );
        assert_eq!(
            last_radio_power("camp_ack cmd=0x0800 response=yes error_raw=2\n"),
            None
        );
        assert_eq!(
            last_radio_power("camp_ack cmd=0x0404 response=yes error_raw=0\n"),
            None
        );
        assert_eq!(
            last_radio_power("camp_ack cmd=0x093f response=yes error_raw=0\n"),
            None
        );
        assert_eq!(
            last_radio_power("camp_dereg radio_on response=yes error_raw=0\n"),
            None
        );
        assert_eq!(
            last_radio_power(
                "camp_ack cmd=0x0800 response=yes error_raw=0\n\
                 camp_ack cmd=0x0800 response=yes error_raw=1\n"
            ),
            None
        );
        assert_eq!(
            last_voice_set(
                "camp_opx set=set_voice_operation response=yes error_raw=0\n"
            ),
            Some("accepted")
        );
        assert_eq!(
            last_voice_set(
                "camp_opx set=set_voice_operation response=yes error_raw=2\n"
            ),
            None
        );
        assert_eq!(
            last_voice_set("camp_opx get=voice_operation mode_raw=3\n"),
            None
        );
        assert_eq!(
            last_voice_set(
                "camp_reg set=preferred_lte_wcdma response=yes error_raw=0\n"
            ),
            None
        );
        assert_eq!(
            last_voice_set("camp_reg set=allow_data response=yes error_raw=0\n"),
            None
        );
        assert_eq!(
            last_voice_set(
                "camp_opx set=set_voice_operation response=yes error_raw=0\n\
                 camp_opx set=set_voice_operation response=yes error_raw=1\n"
            ),
            None
        );
        assert_eq!(
            last_ipv4("camp_setup if=rmnet1 ipv4=yes prefix=32 up=1 add=1 route=1\n"),
            Some("yes")
        );
        assert_eq!(
            last_ipv4("camp_setup if=rmnet1 ipv4=yes prefix=32 up=1 add=0 route=1\n"),
            Some("no")
        );
        assert_eq!(
            last_ipv4("camp_setup if=rmnet1 ipv6=yes prefix=64 up=1 add=1 route=1\n"),
            None
        );
        assert_eq!(
            last_ipv4(
                "camp_setup if=rmnet1 ipv4=yes prefix=32 up=1 add=1 route=1\n\
                 camp_setup if=rmnet1 ipv4=yes prefix=32 up=0 add=1 route=1\n"
            ),
            Some("no")
        );
        assert_eq!(
            last_ipv6("camp_setup if=rmnet1 ipv6=yes prefix=64 up=1 add=1 route=1\n"),
            Some("yes")
        );
        assert_eq!(
            last_ipv6("camp_setup if=rmnet1 ipv6=yes prefix=64 up=1 add=0 route=1\n"),
            Some("no")
        );
        assert_eq!(
            last_ipv6("camp_setup if=rmnet1 ipv4=yes prefix=32 up=1 add=1 route=1\n"),
            None
        );
        assert_eq!(
            last_ipv6(
                "camp_setup if=rmnet1 ipv6=yes prefix=64 up=1 add=1 route=1\n\
                 camp_setup if=rmnet1 ipv6=yes prefix=64 up=0 add=1 route=1\n"
            ),
            Some("no")
        );
        assert_eq!(
            last_data_setup("camp_setup response=yes error_raw=0 len=100\n"),
            Some("accepted")
        );
        assert_eq!(
            last_data_setup("camp_setup response=yes error_raw=2 len=100\n"),
            None
        );
        assert_eq!(
            last_data_setup("camp_setup if=rmnet1 ipv6=yes prefix=64 up=1 add=1 route=1\n"),
            None
        );
        assert_eq!(
            last_data_setup("camp_setup dns=yes count=2\n"),
            None
        );
        assert_eq!(
            last_data_setup("camp_profile response=yes error_raw=0 len=40\n"),
            None
        );
        assert_eq!(
            last_data_setup("camp_ims response=yes error_raw=0 len=40\n"),
            None
        );
        assert_eq!(
            last_data_setup(
                "camp_setup response=yes error_raw=0 len=100\n\
                 camp_setup response=yes error_raw=2 len=100\n"
            ),
            None
        );
        assert_eq!(
            last_data_profile("camp_profile response=yes error_raw=0 len=40\n"),
            Some("accepted")
        );
        assert_eq!(
            last_data_profile("camp_profile response=yes error_raw=2 len=40\n"),
            None
        );
        assert_eq!(
            last_data_profile("camp_setup response=yes error_raw=0 len=100\n"),
            None
        );
        assert_eq!(
            last_data_profile("camp_ims response=yes error_raw=2 len=40\n"),
            None
        );
        assert_eq!(
            last_data_profile("camp_sos response=yes error_raw=2 len=40\n"),
            None
        );
        assert_eq!(
            last_data_profile(
                "camp_profile response=yes error_raw=0 len=40\n\
                 camp_profile response=yes error_raw=2 len=40\n"
            ),
            None
        );
        assert_eq!(
            last_activity("camp_activity response=yes error_raw=0 len=16\n"),
            Some("accepted")
        );
        assert_eq!(
            last_activity("camp_activity response=yes error_raw=2 len=16\n"),
            None
        );
        assert_eq!(
            last_activity("camp_profile response=yes error_raw=0 len=40\n"),
            None
        );
        assert_eq!(
            last_activity("camp_fastdorm response=yes error_raw=0 len=12\n"),
            None
        );
        assert_eq!(
            last_activity(
                "camp_activity response=yes error_raw=0 len=16\n\
                 camp_activity response=yes error_raw=2 len=16\n"
            ),
            None
        );
        assert_eq!(
            last_fastdorm("camp_fastdorm response=yes error_raw=0 len=12\n"),
            Some("accepted")
        );
        assert_eq!(
            last_fastdorm("camp_fastdorm response=yes error_raw=2 len=12\n"),
            None
        );
        assert_eq!(
            last_fastdorm("camp_activity response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_fastdorm("camp_profile response=yes error_raw=0 len=40\n"),
            None
        );
        assert_eq!(
            last_fastdorm(
                "camp_fastdorm response=yes error_raw=0 len=12\n\
                 camp_fastdorm response=yes error_raw=2 len=12\n"
            ),
            None
        );
        assert_eq!(
            last_endc("camp_endc response=yes error_raw=0 len=16\n"),
            Some("accepted")
        );
        assert_eq!(
            last_endc("camp_endc response=yes error_raw=2 len=16\n"),
            None
        );
        assert_eq!(
            last_endc("camp_vonrcapa response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_endc("camp_rcnet response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_endc("camp_ims response=yes error_raw=2 len=40\n"),
            None
        );
        assert_eq!(
            last_endc(
                "camp_endc response=yes error_raw=0 len=16\n\
                 camp_endc response=yes error_raw=2 len=16\n"
            ),
            None
        );
        assert_eq!(
            last_throttle("camp_throttle response=yes error_raw=0 len=16\n"),
            Some("accepted")
        );
        assert_eq!(
            last_throttle("camp_throttle response=yes error_raw=2 len=16\n"),
            None
        );
        assert_eq!(
            last_throttle("camp_screen response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_throttle("camp_unsol response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_throttle("camp_endc response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_throttle(
                "camp_throttle response=yes error_raw=0 len=16\n\
                 camp_throttle response=yes error_raw=2 len=16\n"
            ),
            None
        );
        assert_eq!(
            last_unsolff("camp_unsolff response=yes error_raw=0 len=16\n"),
            Some("accepted")
        );
        assert_eq!(
            last_unsolff("camp_unsolff response=yes error_raw=2 len=16\n"),
            None
        );
        assert_eq!(
            last_unsolff("camp_unsol response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_unsolff("camp_screen response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_unsolff("camp_throttle response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_unsolff(
                "camp_unsolff response=yes error_raw=0 len=16\n\
                 camp_unsolff response=yes error_raw=2 len=16\n"
            ),
            None
        );
        assert_eq!(
            last_unsol("camp_unsol response=yes error_raw=0 len=16\n"),
            Some("accepted")
        );
        assert_eq!(
            last_unsol("camp_unsol response=yes error_raw=2 len=16\n"),
            None
        );
        assert_eq!(
            last_unsol("camp_unsolff response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_unsol("camp_screen response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_unsol("camp_throttle response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_unsol(
                "camp_unsol response=yes error_raw=0 len=16\n\
                 camp_unsol response=yes error_raw=2 len=16\n"
            ),
            None
        );
        assert_eq!(
            last_screen("camp_screen response=yes error_raw=0 len=16\n"),
            Some("accepted")
        );
        assert_eq!(
            last_screen("camp_screen response=yes error_raw=2 len=16\n"),
            None
        );
        assert_eq!(
            last_screen("camp_unsol response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_screen("camp_unsolff response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_screen("camp_throttle response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_screen(
                "camp_screen response=yes error_raw=0 len=16\n\
                 camp_screen response=yes error_raw=2 len=16\n"
            ),
            None
        );
        assert_eq!(
            last_cellinfo("camp_cellinfo response=yes error_raw=0 len=16\n"),
            Some("accepted")
        );
        assert_eq!(
            last_cellinfo("camp_cellinfo response=yes error_raw=2 len=16\n"),
            None
        );
        assert_eq!(
            last_cellinfo("camp_smsc response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_cellinfo("camp_screen response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_cellinfo("camp_vonrget response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_cellinfo(
                "camp_cellinfo response=yes error_raw=0 len=16\n\
                 camp_cellinfo response=yes error_raw=2 len=16\n"
            ),
            None
        );
        assert_eq!(
            last_smsc("camp_smsc response=yes error_raw=0 len=16\n"),
            Some("accepted")
        );
        assert_eq!(
            last_smsc("camp_smsc response=yes error_raw=2 len=16\n"),
            None
        );
        assert_eq!(
            last_smsc("camp_cellinfo response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_smsc("camp_vonrget response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_smsc("camp_screen response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_smsc(
                "camp_smsc response=yes error_raw=0 len=16\n\
                 camp_smsc response=yes error_raw=2 len=16\n"
            ),
            None
        );
        assert_eq!(
            last_vonrget("camp_vonrget response=yes error_raw=0 len=16\n"),
            Some("accepted")
        );
        assert_eq!(
            last_vonrget("camp_vonrget response=yes error_raw=2 len=16\n"),
            None
        );
        assert_eq!(
            last_vonrget("camp_vonrcapa response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_vonrget("camp_smsc response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_vonrget("camp_cellinfo response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_vonrget(
                "camp_vonrget response=yes error_raw=0 len=16\n\
                 camp_vonrget response=yes error_raw=2 len=16\n"
            ),
            None
        );
        assert_eq!(
            last_aptime("camp_aptime response=yes error_raw=0 len=16\n"),
            Some("accepted")
        );
        assert_eq!(
            last_aptime("camp_aptime response=yes error_raw=2 len=16\n"),
            None
        );
        assert_eq!(
            last_aptime("camp_aptime=sent elapsed_ms=12\n"),
            None
        );
        assert_eq!(
            last_aptime("camp_vonrget response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_aptime("camp_dbgtrace response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_aptime(
                "camp_aptime response=yes error_raw=0 len=16\n\
                 camp_aptime response=yes error_raw=2 len=16\n"
            ),
            None
        );
        assert_eq!(
            last_dbgtrace("camp_dbgtrace response=yes error_raw=0 len=16\n"),
            Some("accepted")
        );
        assert_eq!(
            last_dbgtrace("camp_dbgtrace response=yes error_raw=2 len=16\n"),
            None
        );
        assert_eq!(
            last_dbgtrace("camp_dbgtrace=sent elapsed_ms=12\n"),
            None
        );
        assert_eq!(
            last_dbgtrace("camp_aptime response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_dbgtrace("camp_tty response=yes error_raw=0 len=16\n"),
            None
        );
        assert_eq!(
            last_dbgtrace(
                "camp_dbgtrace response=yes error_raw=0 len=16\n\
                 camp_dbgtrace response=yes error_raw=2 len=16\n"
            ),
            None
        );
        assert_eq!(
            last_device_service(
                "camp_opx get=device_service mode_raw=1\nget=device_service status=unknown_short\n"
            ),
            None
        );
        assert_eq!(
            last_stack_mode("camp_opx get=stack_status mode_raw=1\nget=stack_status status=unknown_short\n"),
            None
        );
        assert_eq!(camp_action(Some("ONLINE"), false), "attend");
        assert_eq!(camp_action(None, true), "attend");
    }
}
