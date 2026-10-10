use hdi_contract::{
    Campaign, Claim, ClaimKind, ContractError, Knowledge, ProbeName, ADR_PROBE_EXAMPLE,
    INVESTIGATE_HARDWARE,
};

fn orb_intent() -> String {
    r#"{"semantic_action_id":"investigate_hardware","source":"orb"}"#.to_string()
}

fn probe(pass: u32, name: &str, arguments: &str) -> String {
    format!(
        r#"{{"campaign_id":"uuid","pass":{pass},"requested_probe":"{name}","arguments":{arguments},"risk":"read_only"}}"#
    )
}

#[test]
fn campaign_requires_intent() {
    assert_eq!(
        Campaign::open("").unwrap_err(),
        ContractError::MissingIntent
    );
    assert_eq!(
        Campaign::open("   ").unwrap_err(),
        ContractError::MissingIntent
    );
    assert_eq!(
        Campaign::open("{}").unwrap_err(),
        ContractError::MissingAction
    );
    let campaign = Campaign::open(&orb_intent()).unwrap();
    assert_eq!(
        campaign.intent().semantic_action_id.as_deref(),
        Some(INVESTIGATE_HARDWARE)
    );
    assert_eq!(campaign.executed(), 0);
}

#[test]
fn campaign_rejects_other_action() {
    let document = r#"{"semantic_action_id":"delete_entity","source":"orb"}"#;
    assert_eq!(
        Campaign::open(document).unwrap_err(),
        ContractError::OtherAction {
            action: "delete_entity".into()
        }
    );
}

#[test]
fn probe_request_accepts_adr_example() {
    let mut campaign = Campaign::open(&orb_intent()).unwrap();
    let request = campaign.accept(ADR_PROBE_EXAMPLE).unwrap();
    assert_eq!(request.requested_probe, ProbeName::PciSlot);
    assert_eq!(request.pass, 1);
    assert_eq!(request.risk, "read_only");
    assert_eq!(request.device.as_deref(), Some("pci:0000:01:00.0"));
    assert_eq!(
        request
            .arguments
            .get("slot")
            .and_then(|value| value.as_str()),
        Some("0000:01:00.0")
    );
    assert_eq!(campaign.executed(), 1);
}

#[test]
fn probe_unknown_name_denied() {
    let mut campaign = Campaign::open(&orb_intent()).unwrap();
    for name in ["apple_gmux", "modprobe", "insmod"] {
        let err = campaign.accept(&probe(1, name, "{}")).unwrap_err();
        assert_eq!(err, ContractError::UnknownProbe);
    }
    assert_eq!(campaign.executed(), 0);
}

#[test]
fn probe_shell_metacharacters_denied() {
    let mut campaign = Campaign::open(&orb_intent()).unwrap();
    let samples = [
        r#"{"slot":"0000:01:00.0;id"}"#,
        r#"{"slot":"0000:01:00.0 && id"}"#,
        "{\"slot\":\"0000:01:00.0\\nid\"}",
        r#"{"slot":"0000:01:00.0\"id"}"#,
        r#"{"slot":"0000:01:00.0'id"}"#,
    ];
    for arguments in samples {
        let err = campaign
            .accept(&probe(1, "pci_slot", arguments))
            .unwrap_err();
        assert_eq!(err, ContractError::ShellMetacharacters);
    }
    assert_eq!(campaign.executed(), 0);
}

#[test]
fn probe_risk_write_denied() {
    let mut campaign = Campaign::open(&orb_intent()).unwrap();
    let raw = r#"{"campaign_id":"uuid","pass":1,"requested_probe":"pci_id","arguments":{},"risk":"write"}"#;
    assert_eq!(
        campaign.accept(raw).unwrap_err(),
        ContractError::RiskNotReadOnly
    );
    assert_eq!(campaign.executed(), 0);
}

#[test]
fn probe_path_escape_denied() {
    let mut campaign = Campaign::open(&orb_intent()).unwrap();
    let paths = [
        "../../etc/shadow",
        "/etc/passwd",
        "/sys/bus/pci/devices/../../etc/passwd",
        "/sys/class/dmi/id/product_serial",
    ];
    for path in paths {
        let arguments = format!(r#"{{"path":"{path}"}}"#);
        let err = campaign
            .accept(&probe(1, "sysfs_read", &arguments))
            .unwrap_err();
        assert_eq!(err, ContractError::PathEscape, "{path}");
    }
    let allowed = campaign
        .accept(&probe(
            1,
            "sysfs_read",
            r#"{"path":"/sys/class/dmi/id/product_name"}"#,
        ))
        .unwrap();
    assert_eq!(allowed.requested_probe, ProbeName::SysfsRead);
}

#[test]
fn sixth_pass_not_executable() {
    let mut campaign = Campaign::open(&orb_intent()).unwrap();
    let rejected = campaign.accept(&probe(1, "pci_slot", r#"{"slot":"0000:01:00.0;id"}"#));
    assert_eq!(rejected.unwrap_err(), ContractError::ShellMetacharacters);
    assert_eq!(campaign.executed(), 0);

    for pass in 1..=5 {
        campaign.accept(&probe(pass, "pci_id", "{}")).unwrap();
    }
    assert_eq!(campaign.executed(), 5);
    assert_eq!(
        campaign.accept(&probe(6, "pci_id", "{}")).unwrap_err(),
        ContractError::PassLimit
    );
    assert_eq!(
        campaign.accept(&probe(5, "modules", "{}")).unwrap_err(),
        ContractError::PassLimit
    );
    assert_eq!(campaign.executed(), 5);

    let mut fresh = Campaign::open(&orb_intent()).unwrap();
    assert_eq!(
        fresh.accept(&probe(6, "pci_id", "{}")).unwrap_err(),
        ContractError::PassLimit
    );
    assert_eq!(fresh.executed(), 0);
}

#[test]
fn claim_without_evidence_not_knowledge() {
    let mut knowledge = Knowledge::default();
    let claim = Claim::new(ClaimKind::Causal, "pci:0000:00:02.0", "firmware", vec![]);
    assert_eq!(
        knowledge.try_record(claim).unwrap_err(),
        ContractError::NoEvidence
    );
    assert!(knowledge.records().is_empty());
    let blank = Claim::new(
        ClaimKind::DeviceAbsent,
        "pci:0000:00:02.0",
        "8086:0d26",
        vec![String::new()],
    );
    assert_eq!(
        knowledge.try_record(blank).unwrap_err(),
        ContractError::NoEvidence
    );
    knowledge
        .try_record(Claim::new(
            ClaimKind::DeviceAbsent,
            "pci:0000:00:02.0",
            "8086:0d26",
            vec!["ev-pci".into()],
        ))
        .unwrap();
    assert_eq!(knowledge.records().len(), 1);
}

#[test]
fn claim_equality_is_kind_subject_value() {
    let firmware = Claim::new(
        ClaimKind::DeviceAbsent,
        "pci:0000:00:02.0",
        "8086:0d26",
        vec!["ev-pci".into()],
    )
    .with_hypothesis("firmware hid the function");
    let driver = Claim::new(
        ClaimKind::DeviceAbsent,
        "pci:0000:00:02.0",
        "8086:0d26",
        vec!["ev-other".into()],
    )
    .with_hypothesis("i915 is missing");
    assert!(firmware.same_fact(&driver));

    let present = Claim::new(
        ClaimKind::DevicePresent,
        "pci:0000:00:02.0",
        "8086:0d26",
        vec!["ev-pci".into()],
    );
    assert!(!firmware.same_fact(&present));
}

#[test]
fn worker_intent_requires_precondition() {
    let missing = r#"{"semantic_action_id":"investigate_hardware","source":"worker"}"#;
    assert_eq!(
        Campaign::open(missing).unwrap_err(),
        ContractError::WorkerRequiresPrecondition
    );
    let present = r#"{"semantic_action_id":"investigate_hardware","source":"worker","precondition_id":"graphics_class_incomplete"}"#;
    assert_eq!(
        Campaign::open(present)
            .unwrap()
            .intent()
            .precondition_id
            .as_deref(),
        Some("graphics_class_incomplete")
    );
}

#[test]
fn crate_manifest_has_no_runtime_services() {
    let manifest = include_str!("../Cargo.toml");
    for forbidden in [
        "saai-modemd",
        "saai-taskd",
        "saai-shell",
        "saai-displayd",
        "native-init",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "{forbidden} must not be a dependency"
        );
    }
}
