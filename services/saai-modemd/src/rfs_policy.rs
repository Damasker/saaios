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

/// One RFS header after the camp owner has already chosen a channel.
/// Carrier-config keeps the id until close. Everything else is an NV command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelDecision {
    Carrier(CarrierDecision),
    Nv(RfsDecision),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RfsSession {
    open: [Option<u32>; 8],
    closing: [Option<u32>; 8],
}

impl Default for RfsSession {
    fn default() -> Self {
        Self {
            open: [None; 8],
            closing: [None; 8],
        }
    }
}

impl RfsSession {
    pub fn push(&mut self, frame: &[u8]) -> ChannelDecision {
        if let Some(decision) = self.carrier(frame) {
            return ChannelDecision::Carrier(decision);
        }
        ChannelDecision::Nv(nv_decision(frame))
    }

    fn carrier(&mut self, frame: &[u8]) -> Option<CarrierDecision> {
        if frame.len() < 8 {
            return None;
        }
        let cmd = u16::from_le_bytes([frame[0], frame[1]]);
        if cmd == 4 {
            if frame.len() < 18 || frame[16] != b'/' {
                return None;
            }
            let id = u32_at(frame, 8)?;
            if !carrier_path_ok(frame) || !self.remember_open(id) {
                return Some(CarrierDecision::Deny);
            }
            return Some(CarrierDecision::ReadCopy);
        }
        if cmd == 3 && frame.len() >= 16 {
            let id = u32_at(frame, 12)?;
            if self.contains(&self.open, id) {
                return Some(CarrierDecision::Release);
            }
            if self.take(&mut self.closing, id) {
                return Some(CarrierDecision::Release);
            }
            return None;
        }
        if cmd == 1 && frame.len() >= 16 {
            let id = u32_at(frame, 12)?;
            if self.contains(&self.open, id) {
                return Some(CarrierDecision::ReadCopy);
            }
            return None;
        }
        if (cmd != 5 && cmd != 6) || frame.len() < 12 {
            return None;
        }
        let id = u32_at(frame, 8)?;
        if !self.contains(&self.open, id) {
            return None;
        }
        if cmd == 5 {
            self.take(&mut self.open, id);
            self.remember_closing(id);
            return Some(CarrierDecision::CloseCopy);
        }
        if frame.len() < 24 {
            return Some(CarrierDecision::Deny);
        }
        let op = u32_at(frame, 20)?;
        Some(match op {
            1 => CarrierDecision::ReadCopy,
            _ => CarrierDecision::Deny,
        })
    }

    fn remember_open(&mut self, id: u32) -> bool {
        if let Some(slot) = self.closing.iter_mut().find(|slot| **slot == Some(id)) {
            *slot = None;
        }
        if let Some(slot) = self.open.iter_mut().find(|slot| **slot == Some(id)) {
            *slot = Some(id);
            return true;
        }
        if let Some(slot) = self.open.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(id);
            return true;
        }
        false
    }

    fn remember_closing(&mut self, id: u32) {
        if let Some(slot) = self.closing.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(id);
        }
    }

    fn contains(&self, slots: &[Option<u32>; 8], id: u32) -> bool {
        slots.iter().any(|slot| *slot == Some(id))
    }

    fn take(&mut self, slots: &mut [Option<u32>; 8], id: u32) -> bool {
        if let Some(slot) = slots.iter_mut().find(|slot| **slot == Some(id)) {
            *slot = None;
            true
        } else {
            false
        }
    }
}

fn u32_at(frame: &[u8], at: usize) -> Option<u32> {
    let bytes: [u8; 4] = frame.get(at..at + 4)?.try_into().ok()?;
    Some(u32::from_le_bytes(bytes))
}

fn nv_decision(frame: &[u8]) -> RfsDecision {
    if frame.len() < 12 {
        return RfsDecision::Deny;
    }
    let cmd = u16::from_le_bytes([frame[0], frame[1]]);
    let file_at = if cmd == 2 || cmd == 3 { 12 } else { 8 };
    let Some(file_id) = u32_at(frame, file_at) else {
        return RfsDecision::Deny;
    };
    decide_rfs(file_id, cmd)
}

/// Fixed headers: an open carrier-config id, then the same command number on NV.
pub fn reviewed_dispatch() -> Vec<ChannelDecision> {
    let mut session = RfsSession::default();
    let mut frames = Vec::new();
    frames.push(carrier_open(4));
    frames.push(header(6, 24, 8, 4, Some(1)));
    frames.push(header(6, 24, 8, 4, Some(2)));
    frames.push(header(6, 24, 8, 1, Some(2)));
    frames.push(header(3, 20, 12, 3, None));
    frames.push(header(7, 12, 8, 3, None));
    frames.push(header(5, 12, 8, 4, None));
    frames.push(header(6, 24, 8, 4, Some(1)));
    frames.iter().map(|frame| session.push(frame)).collect()
}

fn carrier_path_ok(frame: &[u8]) -> bool {
    let rest = &frame[16..];
    let Some(end) = rest.iter().position(|byte| *byte == 0) else {
        return false;
    };
    let path = &rest[..end];
    let marker = b"carrierconfig/";
    let Some(at) = path.windows(marker.len()).position(|window| window == marker) else {
        return false;
    };
    let rel = &path[at + marker.len()..];
    !rel.is_empty()
        && rel.len() < 192
        && !rel.windows(2).any(|window| window == b"..")
        && rel.iter().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(*byte, b'/' | b'.' | b'_' | b'-')
        })
}

fn carrier_open(id: u32) -> Vec<u8> {
    let path = b"/carrierconfig/a\0";
    let mut frame = vec![0u8; 16 + path.len()];
    frame[0] = 4;
    frame[8..12].copy_from_slice(&id.to_le_bytes());
    frame[16..].copy_from_slice(path);
    frame
}

fn header(cmd: u16, len: usize, id_at: usize, id: u32, op: Option<u32>) -> Vec<u8> {
    let mut frame = vec![0u8; len];
    frame[0..2].copy_from_slice(&cmd.to_le_bytes());
    frame[id_at..id_at + 4].copy_from_slice(&id.to_le_bytes());
    if let Some(op) = op {
        frame[20..24].copy_from_slice(&op.to_le_bytes());
    }
    frame
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

    #[test]
    fn an_open_carrier_id_is_not_an_nv_write() {
        let mut session = RfsSession::default();
        let open = carrier_open(4);
        assert_eq!(
            session.push(&open),
            ChannelDecision::Carrier(CarrierDecision::ReadCopy)
        );
        let read = header(6, 24, 8, 4, Some(1));
        assert_eq!(
            session.push(&read),
            ChannelDecision::Carrier(CarrierDecision::ReadCopy)
        );
        let write = header(6, 24, 8, 4, Some(2));
        assert_eq!(
            session.push(&write),
            ChannelDecision::Carrier(CarrierDecision::Deny)
        );
        let nv = header(6, 24, 8, 1, Some(2));
        assert_eq!(
            session.push(&nv),
            ChannelDecision::Nv(RfsDecision::QuarantineWrite(NvFile::Normal))
        );
        let mut chunk = vec![0u8; 20];
        chunk[0] = 2;
        chunk[12..16].copy_from_slice(&3u32.to_le_bytes());
        assert_eq!(
            session.push(&chunk),
            ChannelDecision::Nv(RfsDecision::QuarantineWrite(NvFile::Protected))
        );
        let short = header(6, 12, 8, 4, None);
        assert_eq!(
            session.push(&short),
            ChannelDecision::Carrier(CarrierDecision::Deny)
        );
        assert_eq!(
            reviewed_dispatch(),
            vec![
                ChannelDecision::Carrier(CarrierDecision::ReadCopy),
                ChannelDecision::Carrier(CarrierDecision::ReadCopy),
                ChannelDecision::Carrier(CarrierDecision::Deny),
                ChannelDecision::Nv(RfsDecision::QuarantineWrite(NvFile::Normal)),
                ChannelDecision::Nv(RfsDecision::QuarantineStatus(NvFile::Protected)),
                ChannelDecision::Nv(RfsDecision::OpenCopy(NvFile::Protected)),
                ChannelDecision::Carrier(CarrierDecision::CloseCopy),
                ChannelDecision::Nv(RfsDecision::Deny),
            ]
        );
    }

    #[test]
    fn the_ninth_carrier_open_stays_on_that_channel() {
        let mut session = RfsSession::default();
        for id in 10..18 {
            assert_eq!(
                session.push(&carrier_open(id)),
                ChannelDecision::Carrier(CarrierDecision::ReadCopy)
            );
        }
        assert_eq!(
            session.push(&carrier_open(18)),
            ChannelDecision::Carrier(CarrierDecision::Deny)
        );
        let mut bare = header(4, 18, 8, 3, None);
        bare[16] = 0;
        assert_eq!(
            session.push(&bare),
            ChannelDecision::Nv(RfsDecision::Deny)
        );
        let mut rejected = header(4, 20, 8, 9, None);
        rejected[16] = b'/';
        rejected[17] = b'a';
        assert_eq!(
            session.push(&rejected),
            ChannelDecision::Carrier(CarrierDecision::Deny)
        );
        assert_eq!(
            session.push(&header(6, 24, 8, 9, Some(1))),
            ChannelDecision::Nv(RfsDecision::Deny)
        );
    }
}
