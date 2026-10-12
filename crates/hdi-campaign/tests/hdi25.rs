use hdi_campaign::{Hunter, Investigator};
use hdi_contract::ClaimKind;

fn json_string(text: &str) -> String {
    let mut out = String::from("\"");
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

const LS_LINE: &str = "\
lrwxrwxrwx 1 root root 0 Jan 1 2021 card1 -> ../../devices/pci0000:00/0000:00:01.0/0000:00:02.0/drm/card1\n\
lrwxrwxrwx 1 root root 0 Jan 1 2021 card0 -> ../../devices/pci0000:00/0000:00:01.0/0000:01:00.0/drm/card0\n\
lrwxrwxrwx 1 root root 0 Jan 1 2021 renderD129 -> ../../devices/pci0000:00/0000:00:02.0/drm/renderD129\n\
-r--r--r-- 1 root root 4096 Apr 13 23:27 version\n\
total 0\n";

#[test]
fn ls_long_names_the_node_before_the_arrow() {
    let input = format!(
        r#"{{"files":{{"drm.txt":{drm},"modules.txt":"drm\ni915\nnouveau\n"}}}}"#,
        drm = json_string(LS_LINE)
    );
    let claims = Hunter::new().observe(&input);
    assert!(claims
        .iter()
        .any(|claim| { claim.kind == ClaimKind::DevicePresent && claim.subject == "drm:card1" }));
    assert!(claims.iter().any(|claim| {
        claim.kind == ClaimKind::DevicePresent && claim.subject == "drm:renderD129"
    }));
    assert!(claims
        .iter()
        .all(|claim| claim.subject != "drm:version" && claim.value != "drm"));
}

#[test]
fn class_directory_drm_is_not_the_device_driver() {
    let input = r#"{"files":{"drm.txt":"lrwxrwxrwx 1 root root 0 Jan 1 2021 card0 -> ../../devices/pci0000:00/0000:01:00.0/drm/card0\n","modules.txt":"drm\n"}}"#;
    let claims = Hunter::new().observe(input);
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].kind, ClaimKind::DevicePresent);
}

#[test]
fn last_pci_slot_binds_the_loaded_driver() {
    let pci = "00:01.0 PCI bridge [0604]: [8086:0d01]\n\tKernel driver in use: pcieport\n00:02.0 VGA compatible controller [0300]: [8086:0d26]\n\tKernel driver in use: i915\n01:00.0 VGA compatible controller [0300]: [10de:0fe9]\n\tKernel driver in use: nouveau\n";
    let input = format!(
        r#"{{"files":{{"drm.txt":{drm},"modules.txt":"drm\ni915\nnouveau\npcieport\napple_gmux\n","pci_drivers.txt":{pci}}}}}"#,
        drm = json_string(LS_LINE),
        pci = json_string(pci)
    );
    let claims = Hunter::new().observe(&input);
    let bound = |subject: &str| {
        claims
            .iter()
            .find(|claim| claim.kind == ClaimKind::DriverBound && claim.subject == subject)
            .map(|claim| claim.value.as_str())
    };
    assert_eq!(bound("drm:card1"), Some("i915"));
    assert_eq!(bound("drm:card0"), Some("nouveau"));
    assert_eq!(bound("drm:renderD129"), Some("i915"));
    assert!(claims.iter().all(|claim| claim.value != "apple_gmux"
        && claim.value != "pcieport"
        && claim.value != "drm"));
    let card1 = claims
        .iter()
        .find(|claim| claim.kind == ClaimKind::DriverBound && claim.subject == "drm:card1")
        .unwrap();
    assert_eq!(
        card1.evidence_ids,
        ["drm.txt", "pci_drivers.txt", "modules.txt"]
    );
}
