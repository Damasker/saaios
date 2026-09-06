//! Protocol state shared by the headless compositor and its tests.

use sha2::{Digest, Sha256};
use std::fmt::Write;

pub const FRAME_WIDTH: u32 = 64;
pub const FRAME_HEIGHT: u32 = 48;
pub const FRAME_STRIDE: u32 = FRAME_WIDTH * 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolFault {
    BufferBeforeConfigure,
    InvalidConfigureSerial,
    RoleAlreadyAssigned,
}

#[derive(Debug, Default)]
pub struct SurfaceLifecycle {
    role_assigned: bool,
    configure_serial: Option<u32>,
    acked_serial: Option<u32>,
}

impl SurfaceLifecycle {
    pub fn assign_toplevel_role(&mut self) -> Result<(), ProtocolFault> {
        if self.role_assigned {
            return Err(ProtocolFault::RoleAlreadyAssigned);
        }
        self.role_assigned = true;
        Ok(())
    }

    pub fn configure(&mut self, serial: u32) {
        self.configure_serial = Some(serial);
        self.acked_serial = None;
    }

    pub fn ack_configure(&mut self, serial: u32) -> Result<(), ProtocolFault> {
        if self.configure_serial != Some(serial) {
            return Err(ProtocolFault::InvalidConfigureSerial);
        }
        self.acked_serial = Some(serial);
        Ok(())
    }

    pub fn validate_buffer_commit(&self) -> Result<(), ProtocolFault> {
        match (self.configure_serial, self.acked_serial) {
            (Some(configured), Some(acked)) if configured == acked => Ok(()),
            _ => Err(ProtocolFault::BufferBeforeConfigure),
        }
    }
}

#[derive(Debug, Default)]
pub struct FocusState {
    focused_surface: Option<u64>,
}

impl FocusState {
    pub fn focus(&mut self, surface: u64) {
        self.focused_surface = Some(surface);
    }

    pub fn receives_input(&self, surface: u64) -> bool {
        self.focused_surface == Some(surface)
    }
}

pub fn demo_frame() -> Vec<u8> {
    let mut pixels = Vec::with_capacity((FRAME_STRIDE * FRAME_HEIGHT) as usize);
    for y in 0..FRAME_HEIGHT {
        for x in 0..FRAME_WIDTH {
            let checker = ((x / 8) + (y / 8)) % 2 == 0;
            let (red, green, blue) = if checker {
                (0x16, 0xc7, 0x84)
            } else {
                (0x21, 0x2a, 0x3a)
            };
            pixels.extend_from_slice(&[blue, green, red, 0xff]);
        }
    }
    pixels
}

pub fn frame_hash(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest
        .iter()
        .fold(String::with_capacity(64), |mut out, byte| {
            write!(out, "{byte:02x}").expect("writing to String cannot fail");
            out
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configure_ack_commit_is_ordered() {
        let mut lifecycle = SurfaceLifecycle::default();
        lifecycle.assign_toplevel_role().unwrap();
        assert_eq!(
            lifecycle.validate_buffer_commit(),
            Err(ProtocolFault::BufferBeforeConfigure)
        );
        lifecycle.configure(7);
        assert_eq!(
            lifecycle.ack_configure(8),
            Err(ProtocolFault::InvalidConfigureSerial)
        );
        lifecycle.ack_configure(7).unwrap();
        assert_eq!(lifecycle.validate_buffer_commit(), Ok(()));
    }

    #[test]
    fn a_surface_cannot_receive_two_roles() {
        let mut lifecycle = SurfaceLifecycle::default();
        lifecycle.assign_toplevel_role().unwrap();
        assert_eq!(
            lifecycle.assign_toplevel_role(),
            Err(ProtocolFault::RoleAlreadyAssigned)
        );
    }

    #[test]
    fn input_only_reaches_focus() {
        let mut focus = FocusState::default();
        focus.focus(11);
        assert!(focus.receives_input(11));
        assert!(!focus.receives_input(12));
    }

    #[test]
    fn demo_frame_hash_is_stable() {
        assert_eq!(
            frame_hash(&demo_frame()),
            "9b05ff34424f63e620a88baecc12950fe13e242ca1645ec9d8d57d939186f99d"
        );
    }
}
