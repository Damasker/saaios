use std::fs;
use std::path::{Path, PathBuf};

use hdi_campaign::{verify, RecordStatus, StandVerdict, VerifyError};
use hdi_contract::{control_from_redacted, discovery_gain, normalize, Claim, ClaimKind};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../hdi-contract/tests/fixtures")
        .join(name)
}

fn redacted(name: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("redacted");
    normalize(&fixture(name), dir.path()).expect("normalize");
    dir
}

#[test]
fn outcome_a_negative_recipe() {
    let bundle = redacted("nvidia_only");
    assert!(!bundle.path().join("key.txt").exists());
    let report = verify(bundle.path(), &fixture("nvidia_only").join("key.txt")).unwrap();
    assert_eq!(report.verdict, StandVerdict::FirmwareUnenumerated);
    let confirmed = report.confirmed_for("machine-a");
    assert!(confirmed.iter().any(|claim| {
        claim.kind == ClaimKind::NegativeRecipe
            && claim.value == "8086:0d26"
            && claim.evidence_ids == ["pci.txt", "dmi.txt"]
    }));
    let control = control_from_redacted(bundle.path()).unwrap();
    assert!(control
        .iter()
        .all(|claim| claim.kind != ClaimKind::NegativeRecipe));
    assert!(discovery_gain(&confirmed, &control) > 0);
}

#[test]
fn outcome_a_rejects_driver_gap() {
    let bundle = redacted("nvidia_only");
    let mut report = verify(bundle.path(), &fixture("nvidia_only").join("key.txt")).unwrap();
    let proposal = Claim::new(
        ClaimKind::DriverGap,
        "pci:0000:00:02.0",
        "8086:0d26",
        vec!["pci.txt".into()],
    );
    assert_eq!(report.judge(proposal).unwrap(), RecordStatus::Refuted);
    assert!(report
        .confirmed_for("machine-a")
        .iter()
        .all(|claim| claim.kind != ClaimKind::DriverGap));
}

#[test]
fn outcome_b_refutes_hidden_gpu() {
    let bundle = redacted("intel_present");
    let mut report = verify(bundle.path(), &fixture("intel_present").join("key.txt")).unwrap();
    assert_eq!(report.verdict, StandVerdict::IgpuPresent);
    let proposal = Claim::new(
        ClaimKind::NegativeRecipe,
        "pci:0000:00:02.0",
        "8086:0d26",
        vec!["pci.txt".into()],
    );
    assert_eq!(report.judge(proposal).unwrap(), RecordStatus::Refuted);
    assert!(report
        .confirmed_for("machine-intel")
        .iter()
        .all(|claim| claim.kind != ClaimKind::NegativeRecipe));
}

#[test]
fn outcome_c_insufficient_identity() {
    let nvidia = redacted("nvidia_only");
    let intel = redacted("intel_present");
    let unknown = redacted("unknown_product");
    let outcome_a = verify(nvidia.path(), &fixture("nvidia_only").join("key.txt")).unwrap();
    let outcome_b = verify(intel.path(), &fixture("intel_present").join("key.txt")).unwrap();
    let outcome_c = verify(unknown.path(), &fixture("unknown_product").join("key.txt")).unwrap();
    assert_eq!(outcome_c.verdict, StandVerdict::IdentityInsufficient);
    assert_ne!(outcome_a.verdict, outcome_b.verdict);
    assert_ne!(outcome_b.verdict, outcome_c.verdict);
    assert_ne!(outcome_a.verdict, outcome_c.verdict);
    for claim in outcome_c.confirmed_for("machine-b") {
        let blob = format!("{} {}", claim.subject, claim.value);
        assert_ne!(claim.kind, ClaimKind::NegativeRecipe);
        assert!(!blob.contains("11,3"), "{blob}");
        assert!(!blob.contains("MacBookPro"), "{blob}");
    }
}

#[test]
fn causal_without_evidence_not_confirmed() {
    let bundle = redacted("nvidia_only");
    let mut report = verify(bundle.path(), &fixture("nvidia_only").join("key.txt")).unwrap();
    let proposal = Claim::new(ClaimKind::Causal, "pci:0000:00:02.0", "gmux", vec![]);
    assert!(matches!(
        report.judge(proposal),
        Err(VerifyError::NoEvidence)
    ));
    assert!(report
        .confirmed_for("machine-a")
        .iter()
        .all(|claim| claim.kind != ClaimKind::Causal));
}

#[test]
fn knowledge_does_not_transfer() {
    let nvidia = redacted("nvidia_only");
    let unknown = redacted("unknown_product");
    let source = verify(nvidia.path(), &fixture("nvidia_only").join("key.txt")).unwrap();
    let mut control = control_from_redacted(unknown.path()).unwrap();
    let facts = source.confirmed_for("machine-a");
    assert!(!facts.is_empty());
    source.merge_into_control("machine-b", &mut control);
    for fact in &facts {
        assert!(
            control.iter().all(|claim| !claim.same_fact(fact)),
            "machine-a fact transferred"
        );
    }
}

#[test]
fn driver_gap_has_no_insmod() {
    let bundle = redacted("nvidia_only");
    let mut report = verify(bundle.path(), &fixture("nvidia_only").join("key.txt")).unwrap();
    let proposal = Claim::new(
        ClaimKind::DriverGap,
        "pci:0000:00:02.0",
        "insmod i915.ko",
        vec!["pci.txt".into()],
    )
    .with_hypothesis("modprobe i915");
    assert!(matches!(
        report.judge(proposal),
        Err(VerifyError::Executable)
    ));
    let text = report.knowledge_text();
    assert!(!text.contains("insmod"));
    assert!(!text.contains("modprobe"));
    assert!(!text.contains(".ko"));
    let key = fs::read_to_string(fixture("nvidia_only").join("key.txt")).unwrap();
    assert!(!key.contains("insmod"));
}
