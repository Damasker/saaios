//! USB HID keyboard source for the Field-bound IME (ADR-222).
//!
//! Reads raw `EV_KEY` from `/dev/input/eventN` named by
//! `/proc/bus/input/devices`. Does not open volume, power, touch, or
//! haptic nodes. Does not use libxkbcommon. Missing device is OnScreen.

use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Read};
use std::os::unix::fs::OpenOptionsExt;

use saai_ui_core::{hardware_keyboard_present, hardware_keyboards, KeyboardSource};

const INPUT_EVENT_SIZE: usize = 24;
const EV_KEY: u16 = 1;

pub fn detect_keyboard_source() -> KeyboardSource {
    match std::fs::read_to_string("/proc/bus/input/devices") {
        Ok(text) if hardware_keyboard_present(&text) => KeyboardSource::Hardware,
        _ => KeyboardSource::OnScreen,
    }
}

pub fn open_hardware_keyboard() -> Option<File> {
    let text = std::fs::read_to_string("/proc/bus/input/devices").ok()?;
    let device = hardware_keyboards(&text).into_iter().next()?;
    let path = format!("/dev/input/{}", device.event_node);
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
        .ok()
}

pub fn read_key_presses(file: &mut File) -> Vec<u16> {
    let mut buf = [0u8; INPUT_EVENT_SIZE * 32];
    let n = match file.read(&mut buf) {
        Ok(n) => n,
        Err(error) if error.kind() == ErrorKind::WouldBlock => return Vec::new(),
        Err(_) => return Vec::new(),
    };
    buf[..n]
        .chunks_exact(INPUT_EVENT_SIZE)
        .filter_map(parse_ev_key_press)
        .collect()
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
    use super::parse_ev_key_press;
    use saai_ui_core::EVDEV_KEY_A;

    #[test]
    fn ev_key_press_is_parsed_from_a_64bit_input_event() {
        let mut event = [0u8; 24];
        event[16] = 1;
        event[18] = EVDEV_KEY_A as u8;
        event[20] = 1;
        assert_eq!(parse_ev_key_press(&event), Some(EVDEV_KEY_A));
        event[20] = 0;
        assert_eq!(parse_ev_key_press(&event), None);
    }
}
