use std::fs;
use std::path::Path;

use crate::claim::{Claim, ClaimKind};

const IGD_ID: &str = "8086:0d26";
const PCI_EVIDENCE: &str = "pci.txt";
const DRIVER_EVIDENCE: &str = "pci_drivers.txt";

/// Deterministic claims from a redacted bundle. No model and no `causal` kind.
pub fn control_from_redacted(root: &Path) -> std::io::Result<Vec<Claim>> {
    let pci = fs::read_to_string(root.join(PCI_EVIDENCE))?;
    let mut claims = pci_claims(&pci);
    let drivers = root.join(DRIVER_EVIDENCE);
    if drivers.is_file() {
        claims.extend(driver_claims(&fs::read_to_string(drivers)?));
    }
    Ok(claims)
}

fn pci_claims(pci: &str) -> Vec<Claim> {
    let pci = strip_bom(pci);
    let mut claims = Vec::new();
    let mut saw_igd = false;
    for line in pci.lines() {
        let Some((slot, id)) = device_line(line) else {
            continue;
        };
        if id == IGD_ID {
            saw_igd = true;
        }
        claims.push(Claim::new(
            ClaimKind::DevicePresent,
            format!("pci:{slot}"),
            id,
            vec![PCI_EVIDENCE.into()],
        ));
    }
    if !saw_igd {
        claims.push(Claim::new(
            ClaimKind::DeviceAbsent,
            "pci:0000:00:02.0",
            IGD_ID,
            vec![PCI_EVIDENCE.into()],
        ));
    }
    claims
}

fn driver_claims(listing: &str) -> Vec<Claim> {
    let listing = strip_bom(listing);
    let mut claims = Vec::new();
    let mut slot: Option<String> = None;
    for line in listing.lines() {
        if let Some((parsed, _)) = device_line(line) {
            slot = Some(parsed);
            continue;
        }
        let Some(driver) = line.trim().strip_prefix("Kernel driver in use:") else {
            continue;
        };
        let Some(slot) = slot.as_deref() else {
            continue;
        };
        let driver = driver.trim();
        if driver.is_empty() {
            continue;
        }
        claims.push(Claim::new(
            ClaimKind::DriverBound,
            format!("pci:{slot}"),
            driver.to_string(),
            vec![DRIVER_EVIDENCE.into()],
        ));
    }
    claims
}

fn strip_bom(text: &str) -> &str {
    text.strip_prefix('\u{feff}').unwrap_or(text)
}

fn device_line(line: &str) -> Option<(String, String)> {
    let line = strip_bom(line).trim();
    if line.is_empty() || line.starts_with('\t') || line.starts_with(' ') {
        return None;
    }
    let mut parts = line.split_whitespace();
    let slot = parts.next()?;
    let slot = normalize_slot(slot)?;
    let id = last_pci_id(line)?;
    Some((slot, id))
}

fn normalize_slot(slot: &str) -> Option<String> {
    let slot = slot.trim();
    let (domain, rest) = if slot.matches(':').count() == 2 {
        let (domain, rest) = slot.split_once(':')?;
        (domain, rest)
    } else if slot.matches(':').count() == 1 {
        ("0000", slot)
    } else {
        return None;
    };
    if !is_hex(domain, 4) {
        return None;
    }
    let (bus, devfn) = rest.split_once(':')?;
    let (device, function) = devfn.split_once('.')?;
    if is_hex(bus, 2)
        && is_hex(device, 2)
        && function.len() == 1
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

fn last_pci_id(line: &str) -> Option<String> {
    let mut found = None;
    let bytes = line.as_bytes();
    let mut index = 0;
    while index + 11 <= bytes.len() {
        if bytes[index] == b'['
            && bytes[index + 5] == b':'
            && bytes[index + 10] == b']'
            && bytes[index + 1..index + 5]
                .iter()
                .all(|b| b.is_ascii_hexdigit())
            && bytes[index + 6..index + 10]
                .iter()
                .all(|b| b.is_ascii_hexdigit())
        {
            found = Some(line[index + 1..index + 10].to_ascii_lowercase());
            index += 11;
        } else {
            index += 1;
        }
    }
    found
}

fn is_hex(text: &str, len: usize) -> bool {
    text.len() == len && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}
