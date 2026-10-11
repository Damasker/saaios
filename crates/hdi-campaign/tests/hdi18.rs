use std::path::Path;

use hdi_campaign::{campaign_from_action, propose_display_adaptation, INVESTIGATE_HARDWARE};

fn exec_fixture() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../hdi-exec/tests/fixtures/exec-root")
}

#[test]
fn explicit_action_runs_one_campaign() {
    let campaign = tempfile::tempdir().expect("campaign");
    let report = campaign_from_action(INVESTIGATE_HARDWARE, &exec_fixture(), campaign.path())
        .unwrap()
        .expect("report");
    assert_eq!(report.executed, 5);
    assert_eq!(report.investigator_calls, 5);
    assert!(campaign.path().join("journal.jsonl").is_file());
    assert!(!propose_display_adaptation(&report.claims).loaded_module);
}

#[test]
fn other_actions_leave_no_journal() {
    for action in ["adapt_hardware", "boot", "look at the gpu"] {
        let campaign = tempfile::tempdir().expect("campaign");
        let report = campaign_from_action(action, &exec_fixture(), campaign.path()).unwrap();
        assert!(report.is_none(), "{action}");
        assert!(!campaign.path().join("journal.jsonl").exists(), "{action}");
        assert!(!campaign.path().join("bundle").exists(), "{action}");
    }
}
