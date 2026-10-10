use std::path::Path;

use hdi_campaign::{
    run_campaign, Hunter, Investigator, Metrics, Repeater, RunStatus, Transferability,
};

fn fixture() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../hdi-exec/tests/fixtures/exec-root")
}

fn run(investigator: &mut dyn Investigator) -> hdi_campaign::RunReport {
    let campaign = tempfile::tempdir().expect("campaign");
    let report = run_campaign(
        fixture().as_path(),
        campaign.path(),
        &["pci.txt"],
        investigator,
    )
    .expect("campaign");
    assert_eq!(report.status, RunStatus::Completed);
    report
}

struct Forever {
    calls: u32,
}

impl Investigator for Forever {
    fn propose(&mut self, _input_json: &str) -> String {
        self.calls += 1;
        r#"{"campaign_id":"hdi","pass":1,"requested_probe":"pci_id","arguments":{},"risk":"read_only"}"#
            .to_string()
    }
}

struct Garbage;

impl Investigator for Garbage {
    fn propose(&mut self, _input_json: &str) -> String {
        "not json".to_string()
    }
}

#[test]
fn repeater_gain_is_zero() {
    let report = run(&mut Repeater);
    assert_eq!(report.investigator_calls, 5);
    assert_eq!(report.executed, 5);
    assert_eq!(report.metrics.discovery_gain, 0);
    assert_eq!(report.metrics.probe_efficiency, 0.0);
    assert!(report
        .requests
        .iter()
        .all(|request| request.contains("pci_id")));
}

#[test]
fn hunter_requests_unseen_probe() {
    let report = run(&mut Hunter::new());
    assert!(report.requests[0].contains("drm_class"));
    assert!(!report.inputs[0].contains("\"drm.txt\""));
    assert!(report.investigator_calls <= 5);
    assert_eq!(report.executed, 5);
}

#[test]
fn pass_cap_five() {
    let mut forever = Forever { calls: 0 };
    let report = run(&mut forever);
    assert_eq!(forever.calls, 5);
    assert_eq!(report.investigator_calls, 5);
}

#[test]
fn sealed_key_is_absent_from_investigator_input() {
    let key_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/os/sprints/HDI-SEALED-KEY-mbp113.md");
    let key = std::fs::read_to_string(key_path).expect("sealed key");
    assert!(key.contains("Iris Pro 5200"));
    let report = run(&mut Repeater);
    for input in &report.inputs {
        assert!(!input.contains("sealed_key"), "{input}");
        assert!(!input.contains("Iris Pro 5200"), "{input}");
        let value: serde_json::Value = serde_json::from_str(input).unwrap();
        assert!(value.get("sealed_key").is_none());
        assert!(value.get("files").is_some());
    }
}

#[test]
fn protocol_fault_stops_pass() {
    let campaign = tempfile::tempdir().expect("campaign");
    let report = run_campaign(
        fixture().as_path(),
        campaign.path(),
        &["pci.txt"],
        &mut Garbage,
    )
    .expect("campaign");
    assert_eq!(report.status, RunStatus::Completed);
    assert_eq!(report.executed, 0);
    assert!(report.requests.iter().all(|request| request == "not json"));
    let evidence = std::fs::read_dir(campaign.path().join("evidence")).unwrap();
    assert_eq!(evidence.count(), 0);
}

#[test]
fn metrics_cover_definitions() {
    let report = run(&mut Repeater);
    let metrics = &report.metrics;
    assert!(metrics.is_finite());
    assert_eq!(metrics.transferability, Transferability::NotTransferred);
    let value = serde_json::to_value(Metrics::from_run(&[], &[], 0, &[])).unwrap();
    for key in [
        "identification_accuracy",
        "evidence_coverage",
        "discovery_gain",
        "false_hypotheses",
        "probe_efficiency",
        "transferability",
    ] {
        assert!(value.get(key).is_some(), "{key}");
        assert!(!value[key].is_null(), "{key}");
    }
    assert_eq!(value["transferability"], "not_transferred");
    assert_eq!(value["probe_efficiency"], 0.0);
    assert!(value["identification_accuracy"]
        .as_f64()
        .unwrap()
        .is_finite());
    assert!(value["evidence_coverage"].as_f64().unwrap().is_finite());
}
