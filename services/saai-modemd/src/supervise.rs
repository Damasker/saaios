//! One camp per boot. The diagnostic owner keeps the modem file descriptors.
//! This decision only says whether a supervisor may start that owner once.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstAction {
    /// CP is OFFLINE or not loaded yet, and no owner is running.
    LaunchOnce,
    /// Someone already owns the CP, or it is not OFFLINE. Do not start another.
    Attend,
}

pub fn first_action(cp_state: Option<&str>, owner_running: bool) -> FirstAction {
    match saai_observation::camp_action(cp_state, owner_running) {
        "launch-once" => FirstAction::LaunchOnce,
        _ => FirstAction::Attend,
    }
}

/// A fact noticed while the supervisor is already holding.
/// Neither value starts another camp or powers the CP off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldNote {
    OwnerGone,
    CpLeft,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoldWatch {
    saw_owner: bool,
    saw_online: bool,
    owner_noted: bool,
    cp_noted: bool,
}

impl HoldWatch {
    pub fn start(cp_state: Option<&str>, owner_running: bool) -> Self {
        Self {
            saw_owner: owner_running,
            saw_online: cp_state.map(str::trim) == Some("ONLINE"),
            owner_noted: false,
            cp_noted: false,
        }
    }

    /// Edge notes, once each. A later return of the owner or CP stays quiet.
    pub fn poll(&mut self, cp_state: Option<&str>, owner_running: bool) -> Vec<HoldNote> {
        let mut notes = Vec::new();
        if self.saw_owner && !owner_running && !self.owner_noted {
            notes.push(HoldNote::OwnerGone);
            self.owner_noted = true;
        }
        if owner_running {
            self.saw_owner = true;
        }
        let online = cp_state.map(str::trim) == Some("ONLINE");
        if self.saw_online && !online && !self.cp_noted {
            notes.push(HoldNote::CpLeft);
            self.cp_noted = true;
        }
        if online {
            self.saw_online = true;
        }
        notes
    }
}

/// Controlled exit never powers the CP off. While the owner is running the
/// plan names one future `SIGTERM` and does not send it. Mount words are only
/// `persist` and `efs`.
pub fn exit_plan_lines(owner_running: bool, mountinfo: &str) -> Vec<String> {
    let mounts = sensitive_mounts(mountinfo);
    let (exit, signal) = if owner_running {
        ("planned", "term-once")
    } else {
        ("idle", "none")
    };
    let mount_word = if mounts.is_empty() {
        "clear".to_string()
    } else {
        mounts.join(",")
    };
    vec![
        format!("exit={exit}"),
        format!("signal={signal}"),
        "power_off=no".to_string(),
        "performed=no".to_string(),
        format!("mounts={mount_word}"),
        "hardware_actions=none".to_string(),
    ]
}

fn sensitive_mounts(mountinfo: &str) -> Vec<&'static str> {
    let mut persist = false;
    let mut efs = false;
    for line in mountinfo.lines() {
        let point = line.split(' ').nth(4);
        match point {
            Some("/mnt/vendor/persist") => persist = true,
            Some("/mnt/vendor/efs") => efs = true,
            _ => {}
        }
    }
    let mut names = Vec::new();
    if persist {
        names.push("persist");
    }
    if efs {
        names.push("efs");
    }
    names
}

pub fn owner_in_cmdline(bytes: &[u8]) -> bool {
    saai_observation::cmdline_is_camp_owner(bytes)
}

/// Newest numeric directory under the boot archive. Other names are ignored.
pub fn latest_boot_epoch(root: &std::path::Path) -> Option<u64> {
    std::fs::read_dir(root)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_str()?;
            if name.is_empty() || !name.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            name.parse().ok()
        })
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_without_owner_launches_once() {
        assert_eq!(first_action(Some("OFFLINE"), false), FirstAction::LaunchOnce);
        assert_eq!(first_action(None, false), FirstAction::LaunchOnce);
    }

    #[test]
    fn running_owner_or_online_cp_does_not_launch() {
        assert_eq!(first_action(Some("ONLINE"), true), FirstAction::Attend);
        assert_eq!(first_action(Some("ONLINE"), false), FirstAction::Attend);
        assert_eq!(first_action(Some("BOOTING"), false), FirstAction::Attend);
        assert_eq!(first_action(Some("OFFLINE"), true), FirstAction::Attend);
        assert_eq!(first_action(None, true), FirstAction::Attend);
    }

    #[test]
    fn cmdline_matches_the_owner_binary_only() {
        assert!(owner_in_cmdline(
            b"/data/saaios/bin/modem-rfs-camp-combined-owner\0--ipc-fd\0"
        ));
        assert!(owner_in_cmdline(
            b"/data/saaios/bin/saai-modemd\0--ipc-fd\05\0--rfs-fd\06\0"
        ));
        assert!(!owner_in_cmdline(b"/data/saaios/bin/saai-modemd\0supervise\0"));
        assert!(!owner_in_cmdline(b"grep\0modem-rfs-camp-combined-owner\0"));
    }

    #[test]
    fn latest_numeric_archive_is_the_boot_epoch() {
        let root = std::env::temp_dir().join(format!("saai-boot-epoch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("100")).unwrap();
        std::fs::create_dir_all(root.join("250")).unwrap();
        std::fs::create_dir_all(root.join("notes")).unwrap();
        assert_eq!(latest_boot_epoch(&root), Some(250));
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(latest_boot_epoch(&root), None);
    }

    #[test]
    fn hold_notes_each_departure_once_and_does_not_relaunch() {
        let mut watch = HoldWatch::start(Some("ONLINE"), true);
        assert!(watch.poll(Some("ONLINE"), true).is_empty());

        assert_eq!(
            watch.poll(Some("ONLINE"), false),
            vec![HoldNote::OwnerGone]
        );
        assert!(watch.poll(Some("ONLINE"), false).is_empty());
        assert!(watch.poll(Some("ONLINE"), true).is_empty());

        assert_eq!(watch.poll(Some("OFFLINE"), true), vec![HoldNote::CpLeft]);
        assert!(watch.poll(None, false).is_empty());
    }

    #[test]
    fn controlled_exit_names_one_signal_and_never_powers_off() {
        let running = exit_plan_lines(true, "1 2 8:1 / /mnt/vendor/persist rw - ext4\n");
        assert_eq!(
            running,
            vec![
                "exit=planned".to_string(),
                "signal=term-once".to_string(),
                "power_off=no".to_string(),
                "performed=no".to_string(),
                "mounts=persist".to_string(),
                "hardware_actions=none".to_string(),
            ]
        );
        let idle = exit_plan_lines(
            false,
            "1 2 8:5 / /mnt/vendor/efs ro - ext4\n9 1 0:0 / /data rw - ext4\n",
        );
        assert_eq!(idle[0], "exit=idle");
        assert_eq!(idle[1], "signal=none");
        assert_eq!(idle[2], "power_off=no");
        assert_eq!(idle[3], "performed=no");
        assert_eq!(idle[4], "mounts=efs");
        let clear = exit_plan_lines(false, "1 2 0:0 / /data rw - ext4\n");
        assert_eq!(clear[4], "mounts=clear");
        let both = exit_plan_lines(
            true,
            "1 1 8:1 / /mnt/vendor/persist rw\n2 1 8:5 / /mnt/vendor/efs ro\n",
        );
        assert_eq!(both[4], "mounts=persist,efs");
    }

    #[test]
    fn arriving_online_is_not_a_departure() {
        let mut watch = HoldWatch::start(None, false);
        assert!(watch.poll(Some("BOOTING"), false).is_empty());
        assert!(watch.poll(Some("ONLINE"), true).is_empty());
        assert_eq!(watch.poll(None, true), vec![HoldNote::CpLeft]);
    }
}
