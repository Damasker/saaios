use hdi_contract::{Claim, ClaimKind};

/// Proposes the next probe from the serialized redacted bundle.
///
/// Implementations in this crate are scripts. They do not open sockets.
pub trait Investigator {
    fn propose(&mut self, input_json: &str) -> String;

    /// Facts read from the bundle after this pass. The default records nothing.
    fn observe(&mut self, _input_json: &str) -> Vec<Claim> {
        Vec::new()
    }
}

/// Asks for `pci_id` on every pass. The initial bundle already has that file.
pub struct Repeater;

impl Investigator for Repeater {
    fn propose(&mut self, _input_json: &str) -> String {
        pci_id_request()
    }
}

/// Asks for a missing bundle file, then repeats `pci_id`.
///
/// Order: `drm.txt`, then `modules.txt`, then `pci_drivers.txt`, then `platform.txt`,
/// then `platform_drivers.txt`.
/// A DRM driver is recorded only when
/// the same name is in both files.
pub struct Hunter;

impl Hunter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Hunter {
    fn default() -> Self {
        Self::new()
    }
}

impl Investigator for Hunter {
    fn propose(&mut self, input_json: &str) -> String {
        if !bundle_has(input_json, "drm.txt") {
            return drm_class_request();
        }
        if !bundle_has(input_json, "modules.txt") {
            return modules_request();
        }
        if !bundle_has(input_json, "pci_drivers.txt") {
            return pci_drivers_request();
        }
        if !bundle_has(input_json, "platform.txt") {
            return platform_request();
        }
        if !bundle_has(input_json, "platform_drivers.txt") {
            return platform_drivers_request();
        }
        pci_id_request()
    }

    fn observe(&mut self, input_json: &str) -> Vec<Claim> {
        let mut claims = Vec::new();
        claims.extend(drm_node_claims(input_json));
        if let Some(claim) = pci_vga_absent_claim(input_json) {
            claims.push(claim);
        }
        claims.extend(platform_claims(input_json));
        claims.extend(platform_driver_claims(input_json));
        claims
    }
}

fn pci_id_request() -> String {
    r#"{"campaign_id":"hdi","pass":1,"requested_probe":"pci_id","arguments":{},"risk":"read_only"}"#
        .to_string()
}

fn drm_class_request() -> String {
    r#"{"campaign_id":"hdi","pass":1,"requested_probe":"drm_class","arguments":{},"risk":"read_only"}"#
        .to_string()
}

fn pci_drivers_request() -> String {
    r#"{"campaign_id":"hdi","pass":1,"requested_probe":"pci_drivers","arguments":{},"risk":"read_only"}"#
        .to_string()
}

fn platform_request() -> String {
    r#"{"campaign_id":"hdi","pass":1,"requested_probe":"platform_nodes","arguments":{},"risk":"read_only"}"#
        .to_string()
}

fn platform_drivers_request() -> String {
    r#"{"campaign_id":"hdi","pass":1,"requested_probe":"platform_drivers","arguments":{},"risk":"read_only"}"#
        .to_string()
}

fn modules_request() -> String {
    r#"{"campaign_id":"hdi","pass":1,"requested_probe":"modules","arguments":{},"risk":"read_only"}"#
        .to_string()
}

fn bundle_has(input_json: &str, name: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(input_json)
        .ok()
        .and_then(|value| value.get("files")?.get(name).map(|_| ()))
        .is_some()
}

/// Each DRM node is its own device. A driver is bound only from tokens on
/// that same line, so a loaded module that the path does not name is not
/// attached.
fn drm_node_claims(input_json: &str) -> Vec<Claim> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(input_json) else {
        return Vec::new();
    };
    let Some(drm) = value
        .get("files")
        .and_then(|files| files.get("drm.txt"))
        .and_then(|text| text.as_str())
        .map(|text| text.strip_prefix('\u{feff}').unwrap_or(text))
    else {
        return Vec::new();
    };
    let modules = value
        .get("files")
        .and_then(|files| files.get("modules.txt"))
        .and_then(|text| text.as_str())
        .unwrap_or("");
    let loaded = module_names(modules);
    let pci_drivers = value
        .get("files")
        .and_then(|files| files.get("pci_drivers.txt"))
        .and_then(|text| text.as_str())
        .unwrap_or("");
    let slot_drivers = pci_slot_drivers(pci_drivers);
    let mut claims = Vec::new();
    for line in drm.lines() {
        let Some(name) = drm_node_name(line) else {
            continue;
        };
        claims.push(Claim::new(
            ClaimKind::DevicePresent,
            format!("drm:{name}"),
            name,
            vec!["drm.txt".into()],
        ));
        if let Some(driver) = line_driver(line, &loaded) {
            claims.push(Claim::new(
                ClaimKind::DriverBound,
                format!("drm:{name}"),
                driver,
                vec!["drm.txt".into(), "modules.txt".into()],
            ));
        } else if let Some(driver) = slot_driver(line, &slot_drivers, &loaded) {
            claims.push(Claim::new(
                ClaimKind::DriverBound,
                format!("drm:{name}"),
                driver,
                vec![
                    "drm.txt".into(),
                    "pci_drivers.txt".into(),
                    "modules.txt".into(),
                ],
            ));
        }
    }
    claims
}

/// `card0 -> …` keeps the node as the first token. `ls -l` puts the mode
/// first and the node immediately before `->`.
fn drm_node_name(line: &str) -> Option<&str> {
    let line = line.trim();
    if line.is_empty() || line.starts_with("total ") {
        return None;
    }
    let name = if let Some((left, _)) = line.split_once("->") {
        left.split_whitespace().next_back()?
    } else {
        line.split_whitespace().next()?
    };
    is_drm_node(name).then_some(name)
}

fn is_drm_node(name: &str) -> bool {
    is_card_node(name) || is_render_node(name)
}

fn is_card_node(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("card") else {
        return false;
    };
    let (index, connector) = match rest.split_once('-') {
        Some((index, connector)) => (index, Some(connector)),
        None => (rest, None),
    };
    if index.is_empty() || !index.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    connector.is_none_or(|connector| {
        !connector.is_empty()
            && connector
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    })
}

fn is_render_node(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("renderD") else {
        return false;
    };
    !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_digit())
}

fn line_driver(line: &str, loaded: &[String]) -> Option<String> {
    let mut best: Option<String> = None;
    for token in line.split(|ch: char| ch.is_whitespace() || ch == '/' || ch == '\\') {
        if token.contains('.') {
            continue;
        }
        let normalized = token.replace('-', "_");
        // `…/drm/card0` names the class directory, not the device driver.
        if normalized.is_empty()
            || normalized == "drm"
            || !loaded.iter().any(|name| name == &normalized)
        {
            continue;
        }
        if best
            .as_ref()
            .is_none_or(|current| normalized.len() > current.len())
        {
            best = Some(normalized);
        }
    }
    best
}

/// The last PCI slot on the line is the DRM device. An earlier slot is the
/// bridge. The driver name comes from `pci_drivers.txt` and must be loaded.
fn slot_driver(
    line: &str,
    slot_drivers: &std::collections::BTreeMap<String, String>,
    loaded: &[String],
) -> Option<String> {
    let mut found = None;
    for token in line.split(|ch: char| ch.is_whitespace() || ch == '/') {
        let Some(slot) = normalize_bdf(token) else {
            continue;
        };
        if let Some(driver) = slot_drivers.get(&slot) {
            if loaded.iter().any(|name| name == driver) {
                found = Some(driver.clone());
            }
        }
    }
    found
}

fn pci_slot_drivers(listing: &str) -> std::collections::BTreeMap<String, String> {
    let mut drivers = std::collections::BTreeMap::new();
    let mut slot: Option<String> = None;
    for line in listing.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !line.starts_with([' ', '\t']) {
            slot = trimmed.split_whitespace().next().and_then(normalize_bdf);
            continue;
        }
        let Some(driver) = trimmed.strip_prefix("Kernel driver in use:") else {
            continue;
        };
        let driver = driver.trim();
        if driver.is_empty() {
            continue;
        }
        if let Some(slot) = slot.clone() {
            drivers.insert(slot, driver.to_string());
        }
    }
    drivers
}

fn normalize_bdf(token: &str) -> Option<String> {
    let token = token.trim_matches(|ch: char| ch == ',' || ch == ':');
    let (domain, rest) = if token.matches(':').count() == 2 {
        let (domain, rest) = token.split_once(':')?;
        (domain, rest)
    } else if token.matches(':').count() == 1 {
        ("0000", token)
    } else {
        return None;
    };
    if domain.len() != 4 || !domain.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let (bus, devfn) = rest.split_once(':')?;
    let (device, function) = devfn.split_once('.')?;
    if bus.len() == 2
        && device.len() == 2
        && function.len() == 1
        && bus.bytes().all(|byte| byte.is_ascii_hexdigit())
        && device.bytes().all(|byte| byte.is_ascii_hexdigit())
        && function.as_bytes()[0].is_ascii_digit()
        && function.as_bytes()[0] <= b'7'
    {
        Some(format!(
            "{}:{}:{}.{}",
            domain.to_ascii_lowercase(),
            bus.to_ascii_lowercase(),
            device.to_ascii_lowercase(),
            function
        ))
    } else {
        None
    }
}

/// No PCI VGA-compatible function. A present `[0300]` class records nothing:
/// control already lists that device.
fn pci_vga_absent_claim(input_json: &str) -> Option<Claim> {
    let value: serde_json::Value = serde_json::from_str(input_json).ok()?;
    let pci = value.get("files")?.get("pci.txt")?.as_str()?;
    if pci.contains("[0300]") {
        return None;
    }
    Some(Claim::new(
        ClaimKind::DeviceAbsent,
        "pci:class:0300",
        "0300",
        vec!["pci.txt".into()],
    ))
}

/// Names already filtered to public platform nodes. A loaded Mali module
/// is not inferred from this list.
fn platform_claims(input_json: &str) -> Vec<Claim> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(input_json) else {
        return Vec::new();
    };
    let Some(text) = value
        .get("files")
        .and_then(|files| files.get("platform.txt"))
        .and_then(|text| text.as_str())
    else {
        return Vec::new();
    };
    let mut claims = Vec::new();
    for (subject, value_name) in platform_nodes(text) {
        claims.push(Claim::new(
            ClaimKind::DevicePresent,
            subject,
            value_name,
            vec!["platform.txt".into()],
        ));
    }
    if text.lines().any(|line| line.trim() == "udc:") {
        claims.push(Claim::new(
            ClaimKind::DeviceAbsent,
            "usb:udc",
            "none",
            vec!["platform.txt".into()],
        ));
    }
    claims
}

fn platform_nodes(text: &str) -> Vec<(String, String)> {
    let mut mali = None;
    let mut tpu = None;
    let mut isp = None;
    let mut aoc = None;
    for line in text.lines() {
        let name = line.trim().trim_start_matches('\u{feff}');
        if name.is_empty() {
            continue;
        }
        if name == "mali" || name.ends_with(".mali") {
            mali = prefer_longer(mali, name);
        } else if name.ends_with(".TPU") {
            tpu = prefer_longer(tpu, name);
        } else if name.ends_with(".ISP") {
            isp = prefer_longer(isp, name);
        } else if name == "aoc" || name.ends_with(".aoc") {
            aoc = prefer_longer(aoc, name);
        }
    }
    let mut nodes = Vec::new();
    for (subject, value) in [
        ("platform:mali", mali),
        ("platform:tpu", tpu),
        ("platform:isp", isp),
        ("platform:aoc", aoc),
    ] {
        if let Some(value) = value {
            nodes.push((subject.to_string(), value));
        }
    }
    nodes
}

/// The second field is the sysfs driver name. An optional third field is the
/// module behind that driver, and only when `modules.txt` lists it. The module
/// does not replace the driver name.
fn platform_driver_claims(input_json: &str) -> Vec<Claim> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(input_json) else {
        return Vec::new();
    };
    let Some(files) = value.get("files") else {
        return Vec::new();
    };
    let Some(platform) = files.get("platform.txt").and_then(|text| text.as_str()) else {
        return Vec::new();
    };
    let Some(drivers) = files
        .get("platform_drivers.txt")
        .and_then(|text| text.as_str())
    else {
        return Vec::new();
    };
    let loaded = module_names(
        files
            .get("modules.txt")
            .and_then(|text| text.as_str())
            .unwrap_or(""),
    );
    let nodes = platform_nodes(platform);
    let mut claims = Vec::new();
    for line in drivers.lines() {
        let line = line.trim().trim_start_matches('\u{feff}');
        let mut parts = line.split_whitespace();
        let Some(node) = parts.next() else {
            continue;
        };
        let Some(driver) = parts.next() else {
            continue;
        };
        let module = parts.next();
        if parts.next().is_some() || (driver == "unbound" && module.is_some()) {
            continue;
        }
        let Some((subject, _)) = nodes.iter().find(|(_, name)| name == node) else {
            continue;
        };
        if driver == "unbound" {
            claims.push(Claim::new(
                ClaimKind::DriverUnbound,
                subject.clone(),
                "unbound",
                vec!["platform_drivers.txt".into()],
            ));
            continue;
        }
        claims.push(Claim::new(
            ClaimKind::DriverBound,
            subject.clone(),
            driver.to_string(),
            vec!["platform_drivers.txt".into()],
        ));
        if let Some(module) = module {
            if loaded.iter().any(|name| name == module) {
                claims.push(Claim::new(
                    ClaimKind::DriverBound,
                    format!("driver:{driver}"),
                    module.to_string(),
                    vec!["platform_drivers.txt".into(), "modules.txt".into()],
                ));
            }
        }
    }
    claims
}

fn prefer_longer(current: Option<String>, name: &str) -> Option<String> {
    match current {
        Some(existing) if existing.len() >= name.len() => Some(existing),
        _ => Some(name.to_string()),
    }
}

fn module_names(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| !name.is_empty() && !name.contains('.'))
        .map(str::to_string)
        .collect()
}
