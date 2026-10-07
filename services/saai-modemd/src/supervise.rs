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
    fn arriving_online_is_not_a_departure() {
        let mut watch = HoldWatch::start(None, false);
        assert!(watch.poll(Some("BOOTING"), false).is_empty());
        assert!(watch.poll(Some("ONLINE"), true).is_empty());
        assert_eq!(watch.poll(None, true), vec![HoldNote::CpLeft]);
    }
}
