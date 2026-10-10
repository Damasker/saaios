use hdi_campaign::{run_campaign, Hunter};
use hdi_contract::ClaimKind;

fn exec_fixture() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../hdi-exec/tests/fixtures/exec-root")
}

#[test]
fn third_pass_reads_pci_drivers_without_raising_gain() {
    let campaign = tempfile::tempdir().expect("campaign");
    let report = run_campaign(
        exec_fixture().as_path(),
        campaign.path(),
        &["pci.txt"],
        &mut Hunter::new(),
    )
    .expect("campaign");
    assert!(report.requests[2].contains("pci_drivers"));
    assert_eq!(report.executed, 5);
    assert_eq!(report.pass_gain, vec![1, 1, 1, 1, 1]);
    assert_eq!(report.metrics.discovery_gain, 1);
    assert!(report
        .claims
        .iter()
        .all(|claim| { claim.kind != ClaimKind::DriverBound || claim.subject == "drm:card0" }));
    assert!(report.control_facts >= 4);
    let copied = std::fs::read_to_string(campaign.path().join("bundle").join("pci_drivers.txt"))
        .expect("bundle copy");
    assert!(copied.contains("Kernel driver in use: nouveau"));
}

#[test]
fn pci_driver_fact_stays_in_control() {
    let campaign = tempfile::tempdir().expect("campaign");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/drm-bound");
    let report = run_campaign(
        fixture.as_path(),
        campaign.path(),
        &["pci.txt"],
        &mut Hunter::new(),
    )
    .expect("campaign");
    assert_eq!(report.metrics.discovery_gain, 3);
    assert_eq!(report.pass_gain, vec![2, 3, 3, 3, 3]);
    assert!(report.claims.iter().all(|claim| claim.value != "pcieport"));
    assert!(report.control_facts >= 3);
}
