use hdi_campaign::{run_campaign, Hunter, Investigator};
use hdi_contract::ClaimKind;

fn bound_fixture() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/drm-bound")
}

fn exec_fixture() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../hdi-exec/tests/fixtures/exec-root")
}

#[test]
fn hunter_reads_modules_before_repeating_pci() {
    let campaign = tempfile::tempdir().expect("campaign");
    let report = run_campaign(
        exec_fixture().as_path(),
        campaign.path(),
        &["pci.txt"],
        &mut Hunter::new(),
    )
    .expect("campaign");
    assert!(report.requests[0].contains("drm_class"));
    assert!(report.requests[1].contains("\"modules\""));
    assert!(report.requests[2].contains("pci_drivers"));
    assert!(report.requests[3].contains("platform_nodes"));
    assert!(report.requests[4].contains("platform_drivers"));
    assert_eq!(report.metrics.discovery_gain, 1);
    assert!(report
        .claims
        .iter()
        .all(|claim| claim.kind != ClaimKind::DriverBound));
}

#[test]
fn hunter_binds_drm_driver_from_module() {
    let campaign = tempfile::tempdir().expect("campaign");
    let report = run_campaign(
        bound_fixture().as_path(),
        campaign.path(),
        &["pci.txt"],
        &mut Hunter::new(),
    )
    .expect("campaign");
    assert_eq!(report.pass_gain, vec![2, 3, 3, 3, 3]);
    assert_eq!(report.metrics.discovery_gain, 3);
    assert_eq!(report.metrics.probe_efficiency, 0.6);
    assert_eq!(report.claims.len(), 3);
    let bound = report
        .claims
        .iter()
        .find(|claim| claim.kind == ClaimKind::DriverBound)
        .expect("driver");
    assert_eq!(bound.subject, "drm:card0");
    assert_eq!(bound.value, "vendor_drm");
    assert_eq!(
        bound.evidence_ids,
        vec!["drm.txt".to_string(), "modules.txt".to_string()]
    );
    assert!(!bound.value.contains(".ko"));
}

#[test]
fn path_without_loaded_module_does_not_bind() {
    let input = r#"{"files":{"drm.txt":"card0 -> ../../devices/platform/vendor-drm/drm/card0\n","modules.txt":"other\n"}}"#;
    let claims = Hunter::new().observe(input);
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].kind, ClaimKind::DevicePresent);
}

#[test]
fn longer_module_name_wins() {
    let input = r#"{"files":{"drm.txt":"card0 -> ../../devices/platform/vendor-drm/drm/card0\n","modules.txt":"drm\nvendor_drm\n"}}"#;
    let claims = Hunter::new().observe(input);
    let bound = claims
        .iter()
        .find(|claim| claim.kind == ClaimKind::DriverBound)
        .expect("bound");
    assert_eq!(bound.value, "vendor_drm");
}
