use hdi_campaign::{run_campaign, Hunter, Investigator};
use hdi_contract::ClaimKind;

fn split_fixture() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/drm-split")
}

#[test]
fn display_and_render_are_separate_nodes() {
    let campaign = tempfile::tempdir().expect("campaign");
    let report = run_campaign(
        split_fixture().as_path(),
        campaign.path(),
        &["pci.txt"],
        &mut Hunter::new(),
    )
    .expect("campaign");
    assert_eq!(report.pass_gain, vec![4, 7, 7, 7, 7]);
    assert_eq!(report.metrics.discovery_gain, 7);
    assert_eq!(report.metrics.probe_efficiency, 1.4);
    for (subject, value) in [
        ("drm:card0-DSI-1", "card0-DSI-1"),
        ("drm:renderD128", "renderD128"),
    ] {
        assert!(report.claims.iter().any(|claim| {
            claim.kind == ClaimKind::DevicePresent
                && claim.subject == subject
                && claim.value == value
                && claim.evidence_ids == ["drm.txt"]
        }));
        assert!(report.claims.iter().any(|claim| {
            claim.kind == ClaimKind::DriverBound
                && claim.subject == subject
                && claim.value == "vendor_drm"
                && claim.evidence_ids == ["drm.txt", "modules.txt"]
        }));
    }
}

#[test]
fn bom_does_not_hide_card0() {
    let input = "{\"files\":{\"drm.txt\":\"\u{feff}card0 -> ../../devices/platform/vendor-drm/drm/card0\\n\"}}";
    let claims = Hunter::new().observe(input);
    assert!(claims
        .iter()
        .any(|claim| claim.kind == ClaimKind::DevicePresent && claim.subject == "drm:card0"));
}

#[test]
fn loaded_mali_is_not_the_render_driver() {
    let input = r#"{"files":{"drm.txt":"renderD128 -> ../../devices/platform/vendor-drm/drm/renderD128\n","modules.txt":"mali_kbase\nvendor_drm\n"}}"#;
    let claims = Hunter::new().observe(input);
    assert!(claims.iter().any(|claim| {
        claim.kind == ClaimKind::DriverBound
            && claim.subject == "drm:renderD128"
            && claim.value == "vendor_drm"
    }));
    assert!(claims.iter().all(|claim| claim.value != "mali_kbase"));
}
