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

/// Carrier-config on the RFS channel, as the camp owner already answers it.
/// Reads stay on a reviewed copy. Writes are refused. This is not an NV file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarrierDecision {
    ReadCopy,
    CloseCopy,
    Release,
    Deny,
}

pub fn decide_carrier_config(cmd: u16, op: Option<u32>) -> CarrierDecision {
    match cmd {
        4 | 1 if op.is_none() => CarrierDecision::ReadCopy,
        5 if op.is_none() => CarrierDecision::CloseCopy,
        3 if op.is_none() => CarrierDecision::Release,
        6 => match op {
            Some(1) => CarrierDecision::ReadCopy,
            _ => CarrierDecision::Deny,
        },
        _ => CarrierDecision::Deny,
    }
}

pub fn carrier_word(decision: CarrierDecision) -> &'static str {
    match decision {
        CarrierDecision::ReadCopy => "read-copy",
        CarrierDecision::CloseCopy => "close-copy",
        CarrierDecision::Release => "release",
        CarrierDecision::Deny => "deny",
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

    #[test]
    fn carrier_config_stays_a_read_only_copy() {
        assert_eq!(decide_carrier_config(4, None), CarrierDecision::ReadCopy);
        assert_eq!(decide_carrier_config(1, None), CarrierDecision::ReadCopy);
        assert_eq!(decide_carrier_config(6, Some(1)), CarrierDecision::ReadCopy);
        assert_eq!(decide_carrier_config(5, None), CarrierDecision::CloseCopy);
        assert_eq!(decide_carrier_config(3, None), CarrierDecision::Release);
        assert_eq!(decide_carrier_config(6, Some(2)), CarrierDecision::Deny);
        assert_eq!(decide_carrier_config(6, None), CarrierDecision::Deny);
        assert_eq!(decide_carrier_config(4, Some(1)), CarrierDecision::Deny);
        assert_eq!(decide_carrier_config(7, None), CarrierDecision::Deny);
        assert_eq!(decide_rfs(1, 4), RfsDecision::Deny);
        assert_eq!(decide_rfs(3, 4), RfsDecision::Deny);
    }
}
