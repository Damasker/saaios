use std::path::Path;

use hdi_campaign::{verify, RecordStatus, StandVerdict};
use hdi_contract::{Claim, ClaimKind};

fn pixel() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pixel-graphics")
}

#[test]
fn pixel_key_matches_display_and_render() {
    let root = pixel();
    let report = verify(
        root.as_path(),
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pixel-key.txt"),
    )
    .unwrap();
    assert_eq!(report.verdict, StandVerdict::PlatformGraphicsMatched);
    let confirmed = report.confirmed_for("pixel-7");
    assert!(confirmed.iter().any(|claim| {
        claim.kind == ClaimKind::DriverBound
            && claim.subject == "drm:card0-DSI-1"
            && claim.value == "exynos_drm"
    }));
    assert!(confirmed.iter().any(|claim| {
        claim.kind == ClaimKind::DriverBound
            && claim.subject == "drm:renderD128"
            && claim.value == "exynos_drm"
    }));
    assert!(confirmed.iter().any(|claim| {
        claim.kind == ClaimKind::DeviceAbsent
            && claim.subject == "pci:class:0300"
            && claim.value == "0300"
    }));
    assert!(confirmed.iter().all(|claim| claim.value != "mali_kbase"));
}

#[test]
fn wrong_model_stays_insufficient() {
    let dir = tempfile::tempdir().expect("redacted");
    std::fs::write(dir.path().join("devicetree-model.txt"), "Other Board\n").unwrap();
    std::fs::copy(pixel().join("drm.txt"), dir.path().join("drm.txt")).unwrap();
    std::fs::copy(pixel().join("modules.txt"), dir.path().join("modules.txt")).unwrap();
    std::fs::copy(pixel().join("pci.txt"), dir.path().join("pci.txt")).unwrap();
    let report = verify(dir.path(), &pixel().join("key.txt")).unwrap();
    assert_eq!(report.verdict, StandVerdict::IdentityInsufficient);
    let confirmed = report.confirmed_for("pixel-7");
    assert!(confirmed.iter().any(|claim| {
        claim.kind == ClaimKind::InsufficientEvidence
            && claim.subject == "dt:model"
            && claim.value == "Other Board"
    }));
    assert!(confirmed
        .iter()
        .all(|claim| claim.kind != ClaimKind::DriverBound));
}

#[test]
fn mali_render_driver_mismatches() {
    let dir = tempfile::tempdir().expect("redacted");
    for name in ["devicetree-model.txt", "drm.txt", "modules.txt", "pci.txt"] {
        std::fs::copy(pixel().join(name), dir.path().join(name)).unwrap();
    }
    let key = dir.path().join("key.txt");
    std::fs::write(
        &key,
        "machine_id: pixel-7\nexpected_model: GS201 PANTHER MP based on GS201\ndisplay_node: card0-DSI-1\ndisplay_driver: exynos_drm\nrender_node: renderD128\nrender_driver: mali_kbase\n",
    )
    .unwrap();
    let mut report = verify(dir.path(), &key).unwrap();
    assert_eq!(report.verdict, StandVerdict::GraphicsMismatch);
    let proposal = Claim::new(
        ClaimKind::DriverBound,
        "drm:renderD128",
        "mali_kbase",
        vec!["drm.txt".into(), "modules.txt".into()],
    );
    assert_eq!(report.judge(proposal).unwrap(), RecordStatus::Refuted);
}
