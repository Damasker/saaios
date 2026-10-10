use hdi_campaign::{propose_display_adaptation, run_campaign, DisplayAdaptation, Hunter};
use hdi_contract::{Claim, ClaimKind};

fn fixture(name: &str) -> std::path::PathBuf {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    if name == "bound" {
        root.join("tests/fixtures/drm-bound")
    } else {
        root.join("../hdi-exec/tests/fixtures/exec-root")
    }
}

fn hunt(name: &str) -> hdi_campaign::RunReport {
    let campaign = tempfile::tempdir().expect("campaign");
    run_campaign(
        fixture(name).as_path(),
        campaign.path(),
        &["pci.txt"],
        &mut Hunter::new(),
    )
    .expect("campaign")
}

#[test]
fn bound_driver_does_not_propose_a_gap() {
    let report = hunt("bound");
    let proposal = propose_display_adaptation(&report.claims);
    assert_eq!(
        proposal.display,
        DisplayAdaptation::AlreadyBound {
            driver: "vendor_drm".into()
        }
    );
    assert!(!proposal.loaded_module);
    assert!(proposal.gap.is_none());
    assert_eq!(report.metrics.discovery_gain, 3);
}

#[test]
fn unbound_card_is_a_gap_without_a_load() {
    let report = hunt("exec");
    let proposal = propose_display_adaptation(&report.claims);
    assert_eq!(proposal.display, DisplayAdaptation::DriverGap);
    assert!(!proposal.loaded_module);
    let gap = proposal.gap.expect("gap");
    assert_eq!(gap.kind, ClaimKind::DriverGap);
    assert_eq!(gap.subject, "drm:card0");
    assert_eq!(gap.value, "unbound");
    assert_eq!(gap.evidence_ids, vec!["drm.txt".to_string()]);
    let text = format!("{} {}", gap.subject, gap.value);
    assert!(!text.contains("insmod"));
    assert!(!text.contains("modprobe"));
    assert!(!text.contains(".ko"));
}

#[test]
fn no_card_is_insufficient() {
    let proposal = propose_display_adaptation(&[]);
    assert_eq!(proposal.display, DisplayAdaptation::Insufficient);
    assert!(!proposal.loaded_module);
    assert!(proposal.gap.is_none());
}

#[test]
fn executable_driver_name_is_not_bound() {
    let claim = Claim::new(
        ClaimKind::DriverBound,
        "drm:card0",
        "exynos.ko",
        vec!["modules.txt".into()],
    );
    let proposal = propose_display_adaptation(&[claim]);
    assert_eq!(proposal.display, DisplayAdaptation::Insufficient);
    assert!(proposal.gap.is_none());
}
