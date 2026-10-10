use std::path::Path;

use hdi_exec::{probe_templates, ExecError, Executor, Fault, JournalVerdict};

fn fixture() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/exec-root")
}

fn campaign() -> tempfile::TempDir {
    tempfile::tempdir().expect("campaign dir")
}

fn request(pass: u32, name: &str, arguments: &str) -> String {
    format!(
        r#"{{"campaign_id":"uuid","pass":{pass},"requested_probe":"{name}","arguments":{arguments},"risk":"read_only"}}"#
    )
}

fn open(dir: &Path) -> Executor {
    Executor::open(fixture(), dir).expect("executor")
}

#[test]
fn allowlisted_pci_slot_reads_fixture() {
    let dir = campaign();
    let mut executor = open(dir.path());
    let evidence = executor
        .execute(&request(1, "pci_slot", r#"{"slot":"0000:01:00.0"}"#))
        .unwrap();
    let text = String::from_utf8(evidence.bytes).unwrap();
    assert!(text.contains("0000:01:00.0"));
    assert!(text.contains("10de:0fe9"));
    let journal = executor.journal().unwrap();
    assert_eq!(journal.len(), 1);
    assert_eq!(journal[0].verdict, JournalVerdict::Allow);
    assert_eq!(journal[0].sha256.as_deref(), Some(evidence.sha256.as_str()));
    assert_eq!(executor.evidence_files().unwrap().len(), 1);
}

#[test]
fn sysfs_read_prefix_only() {
    let dir = campaign();
    let mut executor = open(dir.path());
    let evidence = executor
        .execute(&request(
            1,
            "sysfs_read",
            r#"{"path":"/sys/class/dmi/id/product_name"}"#,
        ))
        .unwrap();
    assert_eq!(
        String::from_utf8(evidence.bytes).unwrap().trim(),
        "MacBookPro11,3"
    );
    assert_eq!(
        executor.journal().unwrap()[0].verdict,
        JournalVerdict::Allow
    );
}

#[test]
fn sysfs_read_parent_denied() {
    let dir = campaign();
    let mut executor = open(dir.path());
    let raw = request(1, "sysfs_read", r#"{"path":"../../etc/shadow"}"#);
    let error = executor.execute(&raw).unwrap_err();
    assert!(matches!(error, ExecError::Denied { reason } if reason == "path_escape"));
    assert!(executor.evidence_files().unwrap().is_empty());
    let journal = std::fs::read_to_string(dir.path().join("journal.jsonl")).unwrap();
    assert!(!journal.contains(".."));
    assert!(!journal.contains("etc/shadow"));
    assert!(!journal.contains("/etc/"));
}

#[test]
fn denied_probe_is_journaled() {
    let dir = campaign();
    let mut executor = open(dir.path());
    executor
        .execute(&request(4, "sysfs_read", r#"{"path":"/etc/passwd"}"#))
        .unwrap_err();
    let journal = executor.journal().unwrap();
    let denies: Vec<_> = journal
        .iter()
        .filter(|entry| entry.verdict == JournalVerdict::Deny)
        .collect();
    assert_eq!(denies.len(), 1);
    assert_eq!(denies[0].pass, 4);
    assert_eq!(denies[0].probe, "sysfs_read");
    assert_eq!(denies[0].reason.as_deref(), Some("path_escape"));
    assert!(denies[0].sha256.is_none());
}

#[test]
fn executor_crash_does_not_widen_next() {
    let dir = campaign();
    let mut executor = open(dir.path());
    let interrupted = executor.execute_fault(
        &request(
            1,
            "sysfs_read",
            r#"{"path":"/sys/class/dmi/id/product_name"}"#,
        ),
        Fault::AfterAllow,
    );
    assert!(matches!(interrupted, Err(ExecError::Interrupted)));
    assert!(executor.evidence_files().unwrap().is_empty());
    drop(executor);

    let mut executor = open(dir.path());
    let denied = executor.execute(&request(2, "sysfs_read", r#"{"path":"../../etc/shadow"}"#));
    assert!(matches!(denied, Err(ExecError::Denied { reason }) if reason == "path_escape"));
    let journal = executor.journal().unwrap();
    assert!(journal
        .iter()
        .all(|entry| entry.verdict != JournalVerdict::Allow));
    assert!(journal
        .iter()
        .any(|entry| entry.verdict == JournalVerdict::Deny));
    assert!(executor.evidence_files().unwrap().is_empty());
}

#[test]
fn crate_manifest_has_no_runtime_services() {
    let manifest = include_str!("../Cargo.toml");
    for forbidden in [
        "saai-modemd",
        "saai-taskd",
        "saai-shell",
        "saai-displayd",
        "policy-engine",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "{forbidden} must not be a dependency"
        );
    }
}

#[test]
fn no_write_syscall_templates() {
    let templates = probe_templates();
    assert_eq!(templates.len(), 14);
    let mut names = Vec::new();
    for template in templates {
        names.push(template.name);
        let joined = template.argv.join(" ");
        assert!(!joined.contains('>'), "{joined}");
        assert!(!joined.contains("modprobe"), "{joined}");
        assert!(!joined.contains("insmod"), "{joined}");
        assert!(!joined.contains("rmmod"), "{joined}");
        assert!(!joined.contains("setpci"), "{joined}");
    }
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 14);
}

#[test]
fn whitelist_executes_and_replay_keeps_one_file() {
    let dir = campaign();
    let mut executor = open(dir.path());
    let probes = [
        ("pci_id", "{}"),
        ("pci_drivers", "{}"),
        ("pci_tree", "{}"),
        ("pci_slot", r#"{"slot":"0000:01:00.0"}"#),
        ("usb_brief", "{}"),
        ("usb_tree", "{}"),
        ("modules", "{}"),
        ("drm_class", "{}"),
        ("pci_sysfs_list", "{}"),
        ("dmi_allowlist", "{}"),
        ("platform_nodes", "{}"),
        ("platform_drivers", "{}"),
        ("kernel_warnings_redacted", "{}"),
        ("sysfs_read", r#"{"path":"/sys/class/drm/card0/uevent"}"#),
    ];
    let mut hashes = Vec::new();
    for (name, arguments) in probes {
        let evidence = executor
            .execute(&request(1, name, arguments))
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        hashes.push(evidence.sha256);
        if name == "dmi_allowlist" {
            let text = String::from_utf8(evidence.bytes).unwrap();
            assert!(text.contains("product_name:"));
            assert!(!text.contains("SHOULD-NOT-LEAK"));
        }
    }
    let files_after_whitelist = executor.evidence_files().unwrap().len();
    assert_eq!(files_after_whitelist, hashes.len());
    let replay = executor.execute(&request(1, "pci_id", "{}")).unwrap();
    assert_eq!(replay.sha256, hashes[0]);
    assert_eq!(
        executor.evidence_files().unwrap().len(),
        files_after_whitelist
    );
    let unknown = executor.execute(&request(1, "modprobe", "{}")).unwrap_err();
    assert!(matches!(unknown, ExecError::Denied { reason } if reason == "unknown_probe"));
    assert_eq!(
        executor.evidence_files().unwrap().len(),
        files_after_whitelist
    );
    let journal = executor.journal().unwrap();
    assert!(journal
        .iter()
        .any(|entry| entry.verdict == JournalVerdict::Allow));
    assert!(journal
        .iter()
        .any(|entry| entry.verdict == JournalVerdict::Deny));
}
