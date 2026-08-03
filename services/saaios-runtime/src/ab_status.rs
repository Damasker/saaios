//! Read A/B slot layout from disk for runtime status.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbSlotStatus {
    pub name: String,
    pub active: bool,
    pub binary_present: bool,
    pub boot_ok: bool,
    pub boot_ok_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbStatus {
    pub enabled: bool,
    pub root: String,
    pub current: Option<String>,
    pub slots: Vec<AbSlotStatus>,
}

pub fn ab_root_from_env() -> PathBuf {
    std::env::var_os("SAAIOS_AB_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/var/lib/saaios/ab"))
}

pub fn read_ab_status(root: &Path) -> AbStatus {
    if !root.is_dir() {
        return AbStatus {
            enabled: false,
            root: root.display().to_string(),
            current: None,
            slots: vec![],
        };
    }

    let current = read_current_slot(root);
    let mut slots = Vec::new();
    for name in ["A", "B"] {
        let slot_dir = root.join(name);
        let binary_present = slot_dir.join("bin/saaios-runtime").is_file();
        let boot_ok_path = slot_dir.join("BOOT_OK");
        let (boot_ok, boot_ok_at) = if boot_ok_path.is_file() {
            let at = std::fs::read_to_string(&boot_ok_path)
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            (true, at)
        } else {
            (false, None)
        };
        slots.push(AbSlotStatus {
            name: name.into(),
            active: current.as_deref() == Some(name),
            binary_present,
            boot_ok,
            boot_ok_at,
        });
    }

    AbStatus {
        enabled: true,
        root: root.display().to_string(),
        current,
        slots,
    }
}

fn read_current_slot(root: &Path) -> Option<String> {
    if let Ok(target) = std::fs::read_link(root.join("current")) {
        let name = target
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .or_else(|| {
                let s = target.to_string_lossy();
                if s == "A" || s == "B" {
                    Some(s.into_owned())
                } else {
                    None
                }
            });
        if name.is_some() {
            return name;
        }
    }
    std::fs::read_to_string(root.join("current_slot"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| s == "A" || s == "B")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn missing_root_disabled() {
        let st = read_ab_status(Path::new("/no/such/saaios-ab"));
        assert!(!st.enabled);
        assert!(st.slots.is_empty());
    }

    #[test]
    fn reads_slots_and_boot_ok() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("A/bin")).unwrap();
        fs::create_dir_all(root.join("B/bin")).unwrap();
        fs::write(root.join("A/bin/saaios-runtime"), b"x").unwrap();
        fs::write(root.join("current_slot"), "A\n").unwrap();
        fs::write(root.join("A/BOOT_OK"), "2026-01-01T00:00:00Z\n").unwrap();
        std::os::unix::fs::symlink("A", root.join("current")).unwrap();

        let st = read_ab_status(root);
        assert!(st.enabled);
        assert_eq!(st.current.as_deref(), Some("A"));
        let a = st.slots.iter().find(|s| s.name == "A").unwrap();
        assert!(a.active && a.binary_present && a.boot_ok);
        assert_eq!(a.boot_ok_at.as_deref(), Some("2026-01-01T00:00:00Z"));
        let b = st.slots.iter().find(|s| s.name == "B").unwrap();
        assert!(!b.active && !b.binary_present && !b.boot_ok);
    }
}
