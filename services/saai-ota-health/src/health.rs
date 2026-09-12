//! S12 Change 5: process-presence health checks. Deliberately simple --
//! "is a process whose argv[0] basename matches `name` currently
//! running" -- not a protocol-level round-trip into each service. That
//! is enough to distinguish "saaios-runtime is up" from "saaios-runtime
//! exited/never started" for this Change's purpose (ADR-048); a deeper,
//! per-service liveness probe is a reasonable future improvement, not
//! claimed here.

use std::path::Path;

/// Default core services checked after a boot. Order does not matter --
/// `check_processes` reports every missing one, not just the first.
pub fn default_processes() -> Vec<String> {
    [
        "saai-entityd",
        "saai-appd",
        "saai-displayd",
        "saai-shell",
        "saaios-runtime",
    ]
    .iter()
    .map(|name| name.to_string())
    .collect()
}

/// Returns the names in `processes` that are NOT currently running,
/// scanning `proc_root` (real device: `/proc`, injectable for tests).
pub fn missing_processes(proc_root: &Path, processes: &[String]) -> Vec<String> {
    processes
        .iter()
        .filter(|name| !is_process_running(proc_root, name))
        .cloned()
        .collect()
}

fn is_process_running(proc_root: &Path, name: &str) -> bool {
    let entries = match std::fs::read_dir(proc_root) {
        Ok(entries) => entries,
        Err(_) => return false,
    };
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(pid_str) = file_name.to_str() else {
            continue;
        };
        if pid_str.is_empty() || !pid_str.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let cmdline_path = entry.path().join("cmdline");
        let Ok(cmdline) = std::fs::read(&cmdline_path) else {
            continue;
        };
        let first_arg = cmdline.split(|&byte| byte == 0).next().unwrap_or(&[]);
        let first_arg = String::from_utf8_lossy(first_arg);
        let basename = first_arg.rsplit('/').next().unwrap_or(&first_arg);
        if basename == name {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_fake_process(proc_root: &Path, pid: u32, argv0: &str) {
        let pid_dir = proc_root.join(pid.to_string());
        fs::create_dir_all(&pid_dir).unwrap();
        // Real /proc/<pid>/cmdline is NUL-separated with a trailing NUL.
        let mut bytes = argv0.as_bytes().to_vec();
        bytes.push(0);
        fs::write(pid_dir.join("cmdline"), bytes).unwrap();
    }

    #[test]
    fn finds_process_by_argv0_basename() {
        let root = tempdir().unwrap();
        write_fake_process(root.path(), 100, "/saaios/saaios-runtime");
        write_fake_process(root.path(), 101, "/data/saaios/system/saai-appd");

        let processes = vec!["saaios-runtime".to_string(), "saai-appd".to_string()];
        assert!(missing_processes(root.path(), &processes).is_empty());
    }

    #[test]
    fn reports_every_missing_process_not_just_the_first() {
        let root = tempdir().unwrap();
        write_fake_process(root.path(), 100, "/saaios/saai-shell");

        let processes = default_processes();
        let missing = missing_processes(root.path(), &processes);
        assert_eq!(missing.len(), processes.len() - 1);
        assert!(!missing.contains(&"saai-shell".to_string()));
        assert!(missing.contains(&"saaios-runtime".to_string()));
    }

    #[test]
    fn ignores_non_pid_entries() {
        let root = tempdir().unwrap();
        fs::create_dir_all(root.path().join("self")).unwrap();
        fs::create_dir_all(root.path().join("bus")).unwrap();
        write_fake_process(root.path(), 42, "/saaios/saai-entityd");

        let missing = missing_processes(root.path(), &["saai-entityd".to_string()]);
        assert!(missing.is_empty());
    }

    #[test]
    fn all_missing_when_proc_root_does_not_exist() {
        let missing = missing_processes(Path::new("/no/such/proc"), &default_processes());
        assert_eq!(missing.len(), default_processes().len());
    }
}
