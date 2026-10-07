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
    if owner_running {
        return FirstAction::Attend;
    }
    match cp_state.map(str::trim) {
        None | Some("OFFLINE") => FirstAction::LaunchOnce,
        Some(_) => FirstAction::Attend,
    }
}

pub fn owner_in_cmdline(bytes: &[u8]) -> bool {
    let argv0 = bytes.split(|byte| *byte == 0).next().unwrap_or(b"");
    let text = String::from_utf8_lossy(argv0);
    text.ends_with("/modem-rfs-camp-combined-owner")
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
        assert!(!owner_in_cmdline(b"/data/saaios/bin/saai-modemd\0supervise\0"));
        assert!(!owner_in_cmdline(b"grep\0modem-rfs-camp-combined-owner\0"));
    }
}
