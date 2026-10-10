use std::fs;
use std::path::Path;

use hdi_contract::{control_from_redacted, discovery_gain, normalize, ClaimKind};

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn redact(name: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("temp dir");
    normalize(&fixture(name), dir.path()).expect("normalize");
    dir
}

fn bundle_text(root: &Path) -> String {
    let mut all = String::new();
    for entry in fs::read_dir(root).expect("redacted dir") {
        let entry = entry.expect("entry");
        all.push_str(&fs::read_to_string(entry.path()).expect("redacted file"));
        all.push('\n');
    }
    all
}

#[test]
fn redaction_strips_mac_and_iserial() {
    let raw = fs::read_to_string(fixture("secrets").join("usb.txt")).unwrap();
    assert!(raw.contains("FAKE-ISERIAL-9f3a2c"));
    assert!(raw.to_ascii_lowercase().contains("iserial"));
    let redacted = redact("secrets");
    let text = bundle_text(redacted.path());
    let forbidden = [
        "aa:bb:cc:dd:ee:ff",
        "FAKE-ISERIAL-9f3a2c",
        "FAKE-ISERIAL-usb2",
        "FAKE-SERIAL-001",
        "FAKE-BOARD-777",
        "FAKE-DISK-SERIAL",
        "12345678-1234-1234-1234-123456789abc",
        "12345678-abcd-1234-abcd-1234567890ab",
        "iSerial",
        "iserial",
    ];
    for token in forbidden {
        assert!(!text.contains(token), "leaked {token}");
    }
    assert!(!mac_present(&text), "mac pattern still present:\n{text}");
    assert!(!uuid_present(&text), "uuid pattern still present:\n{text}");
}

#[test]
fn redaction_omits_usb_verbose_and_full_dmesg() {
    let redacted = redact("secrets");
    let names: Vec<String> = fs::read_dir(redacted.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(!names.iter().any(|name| name == "dmesg.txt"));
    assert!(!names.iter().any(|name| name == "usb_verbose.txt"));
    let text = bundle_text(redacted.path());
    assert!(!text.contains("UNIQUE-DMESG-SECRET"));
    assert!(!text.contains("UNIQUE-USB-VERBOSE-SECRET"));
    assert!(!text.contains("FAKE-ISERIAL-verbose"));
}

#[test]
fn control_sees_nvidia_and_absent_igd() {
    let redacted = redact("nvidia_only");
    let claims = control_from_redacted(redacted.path()).unwrap();
    assert!(claims.iter().any(|claim| {
        claim.kind == ClaimKind::DevicePresent
            && claim.value == "10de:0fe9"
            && claim.evidence_ids == ["pci.txt"]
    }));
    assert!(claims.iter().any(|claim| {
        claim.kind == ClaimKind::DriverBound
            && claim.subject == "pci:0000:01:00.0"
            && claim.value == "nouveau"
            && claim.evidence_ids == ["pci_drivers.txt"]
    }));
    let igd: Vec<_> = claims
        .iter()
        .filter(|claim| claim.value == "8086:0d26")
        .collect();
    assert_eq!(igd.len(), 1);
    assert_eq!(igd[0].kind, ClaimKind::DeviceAbsent);
    assert_eq!(igd[0].evidence_ids, ["pci.txt"]);
    assert_eq!(discovery_gain(&claims, &claims), 0);
}

#[test]
fn control_sees_igd_when_enumerated() {
    let nvidia_dir = redact("nvidia_only");
    let intel_dir = redact("intel_present");
    let nvidia = control_from_redacted(nvidia_dir.path()).unwrap();
    let intel = control_from_redacted(intel_dir.path()).unwrap();
    assert!(intel.iter().any(|claim| {
        claim.kind == ClaimKind::DevicePresent
            && claim.subject == "pci:0000:00:02.0"
            && claim.value == "8086:0d26"
            && claim.evidence_ids == ["pci.txt"]
    }));
    assert!(intel
        .iter()
        .all(|claim| claim.kind != ClaimKind::DeviceAbsent || claim.value != "8086:0d26"));
    let nvidia_absent = nvidia
        .iter()
        .any(|claim| claim.kind == ClaimKind::DeviceAbsent && claim.value == "8086:0d26");
    let intel_present = intel
        .iter()
        .any(|claim| claim.kind == ClaimKind::DevicePresent && claim.value == "8086:0d26");
    assert!(nvidia_absent && intel_present);
    assert_eq!(discovery_gain(&intel, &intel), 0);
}

#[test]
fn control_does_not_emit_causal() {
    for name in ["nvidia_only", "intel_present", "unknown_product", "secrets"] {
        let redacted = redact(name);
        let claims = control_from_redacted(redacted.path()).unwrap();
        assert!(
            claims.iter().all(|claim| claim.kind != ClaimKind::Causal),
            "{name}"
        );
    }
}

#[test]
fn unknown_product_has_no_mbp_claim() {
    let redacted = redact("unknown_product");
    let claims = control_from_redacted(redacted.path()).unwrap();
    for claim in &claims {
        let blob = format!("{} {} {:?}", claim.subject, claim.value, claim.hypothesis);
        assert!(!blob.contains("11,3"), "{blob}");
        assert!(!blob.contains("MacBookPro"), "{blob}");
    }
}

#[test]
fn bom_does_not_hide_the_first_pci_device() {
    let dir = tempfile::tempdir().expect("temp");
    let pci = "\u{feff}0000:00:00.0 PCI bridge [0604]: [144d:ecec]\n0000:01:00.0 Unclassified device [0000]: [144d:a5a5]\n";
    let drivers = "\u{feff}0000:00:00.0 PCI bridge [0604]: [144d:ecec]\n        Kernel driver in use: pcieport\n";
    fs::write(dir.path().join("pci.txt"), pci).unwrap();
    fs::write(dir.path().join("pci_drivers.txt"), drivers).unwrap();
    let claims = control_from_redacted(dir.path()).unwrap();
    assert!(claims.iter().any(|claim| {
        claim.kind == ClaimKind::DevicePresent
            && claim.subject == "pci:0000:00:00.0"
            && claim.value == "144d:ecec"
    }));
    assert!(claims.iter().any(|claim| {
        claim.kind == ClaimKind::DriverBound
            && claim.subject == "pci:0000:00:00.0"
            && claim.value == "pcieport"
    }));
}

fn mac_present(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.windows(17).any(|window| {
        window.len() == 17
            && window.iter().enumerate().all(|(index, byte)| {
                if index % 3 == 2 {
                    *byte == b':'
                } else {
                    byte.is_ascii_hexdigit()
                }
            })
    })
}

fn uuid_present(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.windows(36).any(|window| {
        const DASHES: [usize; 4] = [8, 13, 18, 23];
        window.len() == 36
            && DASHES.iter().all(|index| window[*index] == b'-')
            && window
                .iter()
                .enumerate()
                .all(|(index, byte)| DASHES.contains(&index) || byte.is_ascii_hexdigit())
    })
}
