use hdi_campaign::{run_campaign, Hunter, Investigator, Repeater};
use hdi_contract::{Claim, ClaimKind};

fn fixture() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../hdi-exec/tests/fixtures/exec-root")
}

fn run(investigator: &mut dyn Investigator) -> hdi_campaign::RunReport {
    let campaign = tempfile::tempdir().expect("campaign");
    run_campaign(
        fixture().as_path(),
        campaign.path(),
        &["pci.txt"],
        investigator,
    )
    .expect("campaign")
}

struct LoadClaim;

impl Investigator for LoadClaim {
    fn propose(&mut self, _input_json: &str) -> String {
        r#"{"campaign_id":"hdi","pass":1,"requested_probe":"pci_id","arguments":{},"risk":"read_only"}"#
            .to_string()
    }

    fn observe(&mut self, _input_json: &str) -> Vec<Claim> {
        vec![Claim::new(
            ClaimKind::DevicePresent,
            "drm:card0",
            "insmod card.ko",
            vec!["drm.txt".into()],
        )]
    }
}

struct BareClaim;

impl Investigator for BareClaim {
    fn propose(&mut self, _input_json: &str) -> String {
        r#"{"campaign_id":"hdi","pass":1,"requested_probe":"pci_id","arguments":{},"risk":"read_only"}"#
            .to_string()
    }

    fn observe(&mut self, _input_json: &str) -> Vec<Claim> {
        vec![Claim::new(
            ClaimKind::DevicePresent,
            "drm:card0",
            "card0",
            Vec::new(),
        )]
    }
}

#[test]
fn repeater_still_has_zero_gain_each_pass() {
    let report = run(&mut Repeater);
    assert_eq!(report.pass_gain, vec![0, 0, 0, 0, 0]);
    assert!(report.claims.is_empty());
}

#[test]
fn hunter_drm_claim_gains_once() {
    let report = run(&mut Hunter::new());
    assert_eq!(report.pass_gain, vec![1, 1, 1, 1, 1]);
    assert_eq!(report.metrics.discovery_gain, 1);
    assert_eq!(report.metrics.probe_efficiency, 0.2);
    assert_eq!(report.metrics.evidence_coverage, 1.0);
    assert_eq!(report.claims.len(), 1);
    assert_eq!(report.claims[0].kind, ClaimKind::DevicePresent);
    assert_eq!(report.claims[0].subject, "drm:card0");
    assert_eq!(report.claims[0].value, "card0");
    assert_eq!(report.claims[0].evidence_ids, vec!["drm.txt".to_string()]);
    assert!(report
        .inputs
        .iter()
        .all(|input| !input.contains("Iris Pro 5200")));
}

#[test]
fn executable_claim_is_not_stored() {
    let report = run(&mut LoadClaim);
    assert!(report.claims.is_empty());
    assert_eq!(report.metrics.discovery_gain, 0);
}

#[test]
fn claim_without_evidence_is_not_stored() {
    let report = run(&mut BareClaim);
    assert!(report.claims.is_empty());
    assert_eq!(report.metrics.discovery_gain, 0);
}
