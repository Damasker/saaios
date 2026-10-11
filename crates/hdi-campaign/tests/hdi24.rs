use std::fs;
use std::path::{Path, PathBuf};

use hdi_campaign::{run_campaign, verify, Investigator, RunStatus};
use hdi_contract::{Claim, ClaimKind};

fn contract_fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../hdi-contract/tests/fixtures")
        .join(name)
}

fn redacted(name: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("redacted");
    hdi_contract::normalize(&contract_fixture(name), dir.path()).expect("normalize");
    dir
}

struct UnreadAbsence {
    calls: u32,
}

impl Investigator for UnreadAbsence {
    fn propose(&mut self, _input_json: &str) -> String {
        self.calls += 1;
        "not json".to_string()
    }

    fn observe(&mut self, _input_json: &str) -> Vec<Claim> {
        if self.calls != 1 {
            return Vec::new();
        }
        vec![
            Claim::new(
                ClaimKind::DeviceAbsent,
                "pci:0000:00:02.0",
                "8086:0d26",
                vec!["not-read.txt".into()],
            ),
            Claim::new(
                ClaimKind::DeviceAbsent,
                "pci:class:0300",
                "0300",
                vec!["pci.txt".into()],
            ),
        ]
    }
}

struct RewriteOnSecondPass<'a> {
    root: &'a Path,
    calls: u32,
}

impl Investigator for RewriteOnSecondPass<'_> {
    fn propose(&mut self, _input_json: &str) -> String {
        self.calls += 1;
        if self.calls == 2 {
            fs::write(self.root.join("pci.txt"), "changed-after-open\n").expect("rewrite");
        }
        r#"{"campaign_id":"hdi","pass":1,"requested_probe":"pci_id","arguments":{},"risk":"read_only"}"#
            .to_string()
    }
}

#[test]
fn unread_file_is_not_device_absent() {
    let root = tempfile::tempdir().expect("fixture");
    fs::write(root.path().join("pci.txt"), "00:00.0 Host bridge\n").unwrap();
    let campaign = tempfile::tempdir().expect("campaign");
    let report = run_campaign(
        root.path(),
        campaign.path(),
        &["pci.txt"],
        &mut UnreadAbsence { calls: 0 },
    )
    .expect("campaign");
    assert_eq!(report.status, RunStatus::Completed);
    assert!(report.claims.iter().any(|claim| {
        claim.kind == ClaimKind::InsufficientEvidence
            && claim.subject == "pci:0000:00:02.0"
            && claim.value == "unread"
    }));
    assert!(report.claims.iter().any(|claim| {
        claim.kind == ClaimKind::DeviceAbsent
            && claim.subject == "pci:class:0300"
            && claim.evidence_ids == ["pci.txt"]
    }));
    assert!(report.claims.iter().all(|claim| {
        claim.kind != ClaimKind::DeviceAbsent || claim.evidence_ids.iter().all(|id| id == "pci.txt")
    }));
}

#[test]
fn changed_package_stops_before_the_next_probe() {
    let root = tempfile::tempdir().expect("fixture");
    fs::write(root.path().join("pci.txt"), "00:00.0 Host bridge\n").unwrap();
    let campaign = tempfile::tempdir().expect("campaign");
    let report = run_campaign(
        root.path(),
        campaign.path(),
        &["pci.txt"],
        &mut RewriteOnSecondPass {
            root: root.path(),
            calls: 0,
        },
    )
    .expect("campaign");
    assert_eq!(report.status, RunStatus::PackageChanged);
    assert_eq!(report.executed, 1);
    assert!(report.executed < 5);
    let journal = fs::read_to_string(campaign.path().join("journal.jsonl")).unwrap_or_default();
    assert!(!journal.contains("changed-after-open"));
}

#[test]
fn negative_recipe_closes_only_outcome_a() {
    let nvidia = redacted("nvidia_only");
    let unknown = redacted("unknown_product");
    let intel = redacted("intel_present");
    let mut outcome_a = verify(
        nvidia.path(),
        &contract_fixture("nvidia_only").join("key.txt"),
    )
    .unwrap();
    assert!(outcome_a.negative_recipe_closed());
    let gmux = Claim::new(
        ClaimKind::DriverBound,
        "pci:0000:00:02.0",
        "gmux",
        vec!["pci.txt".into()],
    );
    assert_eq!(
        outcome_a.judge(gmux).unwrap(),
        hdi_campaign::RecordStatus::Refuted
    );
    assert!(outcome_a.negative_recipe_closed());
    assert!(outcome_a
        .confirmed_for("machine-a")
        .iter()
        .all(|claim| claim.value != "gmux"));
    let outcome_b = verify(
        intel.path(),
        &contract_fixture("intel_present").join("key.txt"),
    )
    .unwrap();
    let outcome_c = verify(
        unknown.path(),
        &contract_fixture("unknown_product").join("key.txt"),
    )
    .unwrap();
    assert!(!outcome_b.negative_recipe_closed());
    assert!(!outcome_c.negative_recipe_closed());
}
