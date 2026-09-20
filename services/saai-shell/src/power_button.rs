//! Pixel 7 power button (`s2mpg12-power-keys` → `/dev/input/power-button`).
//!
//! Not a typing keyboard. Volume keys stay out (separate E slice).
//! `dev-no-lock` does not lock — same policy as idle timeout.

use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Read};
use std::os::unix::fs::OpenOptionsExt;

pub const POWER_BUTTON_PATH: &str = "/dev/input/power-button";
pub const KEY_POWER: u16 = 116;

const INPUT_EVENT_SIZE: usize = 24;
const EV_KEY: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerPressAction {
    IgnoreDevNoLock,
    Lock,
    Sleep,
    WakeToLock,
}

pub fn power_press_action(dev_no_lock: bool, locked: bool, sleeping: bool) -> PowerPressAction {
    if dev_no_lock {
        return PowerPressAction::IgnoreDevNoLock;
    }
    if sleeping {
        PowerPressAction::WakeToLock
    } else if locked {
        PowerPressAction::Sleep
    } else {
        PowerPressAction::Lock
    }
}

pub fn open_power_button() -> Option<File> {
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(POWER_BUTTON_PATH)
        .ok()
}

pub fn read_power_presses(file: &mut File) -> bool {
    let mut buf = [0u8; INPUT_EVENT_SIZE * 32];
    let n = match file.read(&mut buf) {
        Ok(n) => n,
        Err(error) if error.kind() == ErrorKind::WouldBlock => return false,
        Err(_) => return false,
    };
    buf[..n]
        .chunks_exact(INPUT_EVENT_SIZE)
        .any(|event| parse_ev_key_press(event) == Some(KEY_POWER))
}

fn parse_ev_key_press(event: &[u8]) -> Option<u16> {
    if event.len() < INPUT_EVENT_SIZE {
        return None;
    }
    let type_ = u16::from_le_bytes([event[16], event[17]]);
    let code = u16::from_le_bytes([event[18], event[19]]);
    let value = i32::from_le_bytes([event[20], event[21], event[22], event[23]]);
    (type_ == EV_KEY && value == 1).then_some(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_power_press_is_parsed_and_letters_are_not() {
        let mut event = [0u8; 24];
        event[16] = 1;
        event[18] = KEY_POWER as u8;
        event[20] = 1;
        assert_eq!(parse_ev_key_press(&event), Some(KEY_POWER));
        event[18] = 30;
        assert_eq!(parse_ev_key_press(&event), Some(30));
        event[18] = KEY_POWER as u8;
        event[20] = 0;
        assert_eq!(parse_ev_key_press(&event), None);
    }

    #[test]
    fn power_press_respects_dev_no_lock_and_sleep_cycle() {
        assert_eq!(
            power_press_action(true, false, false),
            PowerPressAction::IgnoreDevNoLock
        );
        assert_eq!(
            power_press_action(false, false, false),
            PowerPressAction::Lock
        );
        assert_eq!(
            power_press_action(false, true, false),
            PowerPressAction::Sleep
        );
        assert_eq!(
            power_press_action(false, true, true),
            PowerPressAction::WakeToLock
        );
    }
}
