use std::fs;
use std::path::Path;

use hdi_exec::{probe_templates, ExecError, Executor, JournalVerdict};

fn request(pass: u32, arguments: &str) -> String {
    format!(
        r#"{{"campaign_id":"uuid","pass":{pass},"requested_probe":"sysfs_read","arguments":{arguments},"risk":"read_only"}}"#
    )
}

#[test]
fn temp_root_allows_prefix_and_denies_escape() {
    let root = tempfile::tempdir().expect("sys root");
    let campaign = tempfile::tempdir().expect("campaign");
    assert_ne!(root.path(), Path::new("/"));
    assert_ne!(root.path(), Path::new("/sys"));
    let uevent = root.path().join("sys/class/drm/card0/uevent");
    fs::create_dir_all(uevent.parent().expect("parent")).expect("sysfs dirs");
    fs::write(&uevent, "DRIVER=exynos-drm\n").expect("uevent");

    let mut executor =
        Executor::open_sys_root(root.path(), root.path(), campaign.path()).expect("executor");
    let evidence = executor
        .execute(&request(
            1,
            r#"{"path":"/sys/class/drm/card0/uevent"}"#,
        ))
        .expect("allow");
    assert_eq!(
        String::from_utf8(evidence.bytes).unwrap().trim(),
        "DRIVER=exynos-drm"
    );

    let parent = executor.execute(&request(2, r#"{"path":"/sys/class/drm/../../etc/passwd"}"#));
    assert!(matches!(parent, Err(ExecError::Denied { reason }) if reason == "path_escape"));

    let outside = executor.execute(&request(3, r#"{"path":"/sys/class/net/wlan0/address"}"#));
    assert!(matches!(outside, Err(ExecError::Denied { reason }) if reason == "path_escape"));

    let shell = executor.execute(&request(4, r#"{"path":"/sys/class/drm/card0;id"}"#));
    assert!(
        matches!(shell, Err(ExecError::Denied { reason }) if reason == "shell_metacharacters")
    );

    let journal = executor.journal().unwrap();
    assert!(journal
        .iter()
        .any(|entry| entry.verdict == JournalVerdict::Allow && entry.sha256.is_some()));
    assert!(journal.iter().any(|entry| {
        entry.verdict == JournalVerdict::Deny && entry.reason.as_deref() == Some("path_escape")
    }));
    assert!(journal.iter().any(|entry| {
        entry.verdict == JournalVerdict::Deny
            && entry.reason.as_deref() == Some("shell_metacharacters")
    }));
    let text = fs::read_to_string(campaign.path().join("journal.jsonl")).unwrap();
    assert!(!text.contains(".."));
    assert!(!text.contains("/etc/"));
    assert_eq!(probe_templates().len(), 14);
    assert_eq!(executor.evidence_files().unwrap().len(), 1);
}

#[cfg(unix)]
#[test]
fn symlink_leaving_the_root_is_denied() {
    let root = tempfile::tempdir().expect("sys root");
    let outside = tempfile::tempdir().expect("outside");
    let campaign = tempfile::tempdir().expect("campaign");
    fs::write(outside.path().join("secret"), "secret\n").expect("secret");
    let link_dir = root.path().join("sys/class/drm/card0");
    fs::create_dir_all(&link_dir).expect("link dir");
    std::os::unix::fs::symlink(outside.path().join("secret"), link_dir.join("uevent"))
        .expect("symlink");

    let mut executor =
        Executor::open_sys_root(root.path(), root.path(), campaign.path()).expect("executor");
    let denied = executor.execute(&request(1, r#"{"path":"/sys/class/drm/card0/uevent"}"#));
    assert!(matches!(denied, Err(ExecError::Denied { reason }) if reason == "path_escape"));
    assert!(executor.evidence_files().unwrap().is_empty());
    let journal = executor.journal().unwrap();
    assert_eq!(journal.len(), 1);
    assert_eq!(journal[0].verdict, JournalVerdict::Deny);
}
