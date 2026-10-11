use std::fs;
use std::path::Path;

use hdi_campaign::{
    confirmation_required, propose_display_adaptation, run_campaign, DisplayAdaptation, Hunter,
    GRAPHICS_PRECONDITION, SHIPPED_PRECONDITIONS,
};

fn hunt(root: &Path) -> hdi_campaign::RunReport {
    let campaign = tempfile::tempdir().expect("campaign");
    run_campaign(root, campaign.path(), &["pci.txt"], &mut Hunter::new()).expect("campaign")
}

#[test]
fn driver_gap_waits_and_does_not_load() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../hdi-exec/tests/fixtures/exec-root");
    let report = hunt(&root);
    let proposal = propose_display_adaptation(&report.claims);
    assert_eq!(proposal.display, DisplayAdaptation::DriverGap);
    assert!(confirmation_required(&proposal));
    assert!(!proposal.loaded_module);
    let gap = proposal.gap.expect("gap");
    let text = format!("{} {}", gap.subject, gap.value);
    assert!(!text.contains("insmod"));
    assert!(!text.contains("modprobe"));
    assert!(!text.contains(".ko"));
}

#[test]
fn pixel_bound_driver_does_not_ask() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pixel-graphics");
    let dir = tempfile::tempdir().expect("pixel");
    for name in ["pci.txt", "modules.txt", "devicetree-model.txt"] {
        fs::copy(source.join(name), dir.path().join(name)).unwrap();
    }
    let drm = fs::read_to_string(source.join("drm.txt")).unwrap();
    fs::write(
        dir.path().join("drm.txt"),
        format!("card0 -> ../../devices/platform/exynos-drm/drm/card0\n{drm}"),
    )
    .unwrap();
    let report = hunt(dir.path());
    let proposal = propose_display_adaptation(&report.claims);
    assert_eq!(
        proposal.display,
        DisplayAdaptation::AlreadyBound {
            driver: "exynos_drm".into()
        }
    );
    assert!(!confirmation_required(&proposal));
    assert!(!proposal.loaded_module);
    assert!(proposal.gap.is_none());
}

#[test]
fn shipped_graphics_precondition_stays_off() {
    assert!(SHIPPED_PRECONDITIONS
        .iter()
        .any(|rule| rule.id == GRAPHICS_PRECONDITION && !rule.enabled));
    assert!(SHIPPED_PRECONDITIONS.iter().all(|rule| !rule.enabled));
}
