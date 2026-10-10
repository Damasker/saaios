use hdi_campaign::{run_campaign, Hunter};
use hdi_contract::ClaimKind;

#[test]
fn platform_driver_comes_from_the_driver_file() {
    let root = tempfile::tempdir().expect("fixture");
    let path = root.path();
    std::fs::write(
        path.join("pci.txt"),
        "0000:00:00.0 PCI bridge [0604]: [1234:0001]\n",
    )
    .unwrap();
    std::fs::write(path.join("drm.txt"), "card0\n").unwrap();
    std::fs::write(path.join("modules.txt"), "mali_kbase\nmali_pixel\n").unwrap();
    std::fs::write(
        path.join("pci_drivers.txt"),
        "0000:00:00.0 PCI bridge [0604]: [1234:0001]\n        Kernel driver in use: pcieport\n",
    )
    .unwrap();
    std::fs::write(
        path.join("platform.txt"),
        "28000000.mali\n100b0000.TPU\n100a0000.ISP\n19000000.aoc\nudc:\n",
    )
    .unwrap();
    std::fs::write(
        path.join("platform_drivers.txt"),
        "28000000.mali mali\n100b0000.TPU unbound\n100a0000.ISP unbound\n19000000.aoc aoc\n28000000.mali mali_kbase extra\ndbgdev-pd-tpu mali_kbase\n",
    )
    .unwrap();
    let campaign = tempfile::tempdir().expect("campaign");
    let report = run_campaign(path, campaign.path(), &["pci.txt"], &mut Hunter::new()).unwrap();
    assert!(report.requests[4].contains("platform_drivers"));
    assert_eq!(report.pass_gain[3] + 4, report.pass_gain[4]);
    let bound = report
        .claims
        .iter()
        .find(|claim| claim.kind == ClaimKind::DriverBound && claim.subject == "platform:mali")
        .expect("mali binding");
    assert_eq!(bound.value, "mali");
    assert_eq!(bound.evidence_ids, vec!["platform_drivers.txt".to_string()]);
    assert!(report.claims.iter().any(|claim| {
        claim.kind == ClaimKind::DriverBound
            && claim.subject == "platform:aoc"
            && claim.value == "aoc"
    }));
    for subject in ["platform:tpu", "platform:isp"] {
        assert!(report.claims.iter().any(|claim| {
            claim.kind == ClaimKind::DriverUnbound
                && claim.subject == subject
                && claim.value == "unbound"
                && claim.evidence_ids == ["platform_drivers.txt"]
        }));
    }
    assert!(report.claims.iter().all(|claim| {
        claim.value != "mali_kbase"
            && claim.value != "mali_pixel"
            && claim.subject != "dbgdev-pd-tpu"
    }));
}
