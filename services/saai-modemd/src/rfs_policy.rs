//! Which RFS requests a future server may answer.
//! Original EFS is not a source and not a destination. This module opens
//! nothing and returns no reply bytes.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvFile {
    Normal,
    Protected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RfsDecision {
    /// Open or status against the verified quarantine copy.
    OpenCopy(NvFile),
    /// CP write bytes land only in the quarantine candidate.
    QuarantineWrite(NvFile),
    /// Final status for a quarantine exchange. Not a promotion.
    QuarantineStatus(NvFile),
    Deny,
}

pub fn decide_rfs(file_id: u32, cmd: u16) -> RfsDecision {
    let file = match file_id {
        1 => NvFile::Normal,
        3 => NvFile::Protected,
        _ => return RfsDecision::Deny,
    };
    match cmd {
        7 => RfsDecision::OpenCopy(file),
        2 | 6 => RfsDecision::QuarantineWrite(file),
        3 => RfsDecision::QuarantineStatus(file),
        _ => RfsDecision::Deny,
    }
}

pub fn decision_word(decision: RfsDecision) -> &'static str {
    match decision {
        RfsDecision::OpenCopy(_) => "open-copy",
        RfsDecision::QuarantineWrite(_) => "quarantine-write",
        RfsDecision::QuarantineStatus(_) => "quarantine-status",
        RfsDecision::Deny => "deny",
    }
}

pub fn file_word(file: NvFile) -> &'static str {
    match file {
        NvFile::Normal => "normal",
        NvFile::Protected => "protected",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reviewed_nv_files_stay_in_quarantine() {
        assert_eq!(
            decide_rfs(3, 2),
            RfsDecision::QuarantineWrite(NvFile::Protected)
        );
        assert_eq!(decide_rfs(1, 6), RfsDecision::QuarantineWrite(NvFile::Normal));
        assert_eq!(decide_rfs(3, 7), RfsDecision::OpenCopy(NvFile::Protected));
        assert_eq!(
            decide_rfs(1, 3),
            RfsDecision::QuarantineStatus(NvFile::Normal)
        );
    }

    #[test]
    fn other_files_and_commands_are_denied() {
        assert_eq!(decide_rfs(0, 2), RfsDecision::Deny);
        assert_eq!(decide_rfs(2, 7), RfsDecision::Deny);
        assert_eq!(decide_rfs(3, 1), RfsDecision::Deny);
        assert_eq!(decide_rfs(3, 4), RfsDecision::Deny);
        assert_eq!(decide_rfs(3, 5), RfsDecision::Deny);
        assert_eq!(decide_rfs(9, 6), RfsDecision::Deny);
    }
}
