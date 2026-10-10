use hdi_campaign::{run_campaign, Hunter, Investigator};
use hdi_contract::ClaimKind;

#[test]
fn platform_nodes_are_named_once() {
    let root = tempfile::tempdir().expect("fixture");
    let path = root.path();
    std::fs::write(path.join("pci.txt"), "0000:00:00.0 PCI bridge [0604]: [1234:0001]\n").unwrap();
    std::fs::write(path.join("drm.txt"), "card0\n").unwrap();
    std::fs::write(path.join("modules.txt"), "other\n").unwrap();
    std::fs::write(path.join("pci_drivers.txt"), "0000:00:00.0 PCI bridge [0604]: [1234:0001]\n        Kernel driver in use: pcieport\n").unwrap();
    std::fs::write(
        path.join("platform.txt"),
        "mali\n28000000.mali\n100b0000.TPU\n100a0000.ISP\n19000000.aoc\ndbgdev-pd-tpu\nudc:\n",
    )
    .unwrap();
    let campaign = tempfile::tempdir().expect("campaign");
    let report = run_campaign(path, campaign.path(), &["pci.txt"], &mut Hunter::new()).unwrap();
    assert!(report.requests[3].contains("platform_nodes"));
    assert_eq!(report.pass_gain[2], report.pass_gain[3] - 5);
    for (subject, value) in [
        ("platform:mali", "28000000.mali"),
        ("platform:tpu", "100b0000.TPU"),
        ("platform:isp", "100a0000.ISP"),
        ("platform:aoc", "19000000.aoc"),
    ] {
        assert!(report.claims.iter().any(|claim| {
            claim.kind == ClaimKind::DevicePresent
                && claim.subject == subject
                && claim.value == value
                && claim.evidence_ids == ["platform.txt"]
        }));
    }
    assert!(report.claims.iter().any(|claim| {
        claim.kind == ClaimKind::DeviceAbsent
            && claim.subject == "usb:udc"
            && claim.value == "none"
    }));
    assert!(report
        .claims
        .iter()
        .all(|claim| claim.value != "dbgdev-pd-tpu" && claim.value != "mali"));
}
