use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

pub const FRAME_WIDTH: u32 = 64;
pub const FRAME_HEIGHT: u32 = 48;
pub const FRAME_STRIDE: u32 = FRAME_WIDTH * 4;

pub fn demo_frame() -> Vec<u8> {
    let mut pixels = Vec::with_capacity((FRAME_STRIDE * FRAME_HEIGHT) as usize);
    for y in 0..FRAME_HEIGHT {
        for x in 0..FRAME_WIDTH {
            let (red, green, blue) = match (x < FRAME_WIDTH / 2, y < FRAME_HEIGHT / 2) {
                (true, true) => (0xe8, 0x32, 0x5a),
                (false, true) => (0x19, 0xc3, 0x7d),
                (true, false) => (0x36, 0x71, 0xe8),
                (false, false) => (0xf4, 0xc4, 0x30),
            };
            pixels.extend_from_slice(&[blue, green, red, 0xff]);
        }
    }
    pixels
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn expected_frame_hash() -> String {
    sha256_hex(&demo_frame())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfacePhase {
    New,
    ConfigureSent(u32),
    Configured,
    Presented,
    Closed,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ProtocolViolation {
    UnknownConfigure,
    BufferBeforeConfigure,
    CommitAfterClose,
    RoleAlreadyAssigned,
}

#[derive(Debug)]
pub struct SurfaceLifecycle {
    phase: SurfacePhase,
}

impl Default for SurfaceLifecycle {
    fn default() -> Self {
        Self {
            phase: SurfacePhase::New,
        }
    }
}

impl SurfaceLifecycle {
    pub fn phase(&self) -> SurfacePhase {
        self.phase
    }

    pub fn configure(&mut self, serial: u32) {
        self.phase = SurfacePhase::ConfigureSent(serial);
    }

    pub fn ack_configure(&mut self, serial: u32) -> Result<(), ProtocolViolation> {
        match self.phase {
            SurfacePhase::ConfigureSent(expected) if expected == serial => {
                self.phase = SurfacePhase::Configured;
                Ok(())
            }
            _ => Err(ProtocolViolation::UnknownConfigure),
        }
    }

    pub fn attach_buffer(&mut self) -> Result<(), ProtocolViolation> {
        match self.phase {
            SurfacePhase::Configured | SurfacePhase::Presented => {
                self.phase = SurfacePhase::Presented;
                Ok(())
            }
            SurfacePhase::Closed => Err(ProtocolViolation::CommitAfterClose),
            _ => Err(ProtocolViolation::BufferBeforeConfigure),
        }
    }

    pub fn close(&mut self) {
        self.phase = SurfacePhase::Closed;
    }
}

#[derive(Debug, Default)]
pub struct FocusRouter {
    focused_client: Option<u64>,
}

#[derive(Debug, Default)]
pub struct RoleRegistry {
    assigned: HashSet<u64>,
}

impl RoleRegistry {
    pub fn assign_xdg_surface(&mut self, surface_id: u64) -> Result<(), ProtocolViolation> {
        if self.assigned.insert(surface_id) {
            Ok(())
        } else {
            Err(ProtocolViolation::RoleAlreadyAssigned)
        }
    }

    pub fn release_xdg_surface(&mut self, surface_id: u64) {
        self.assigned.remove(&surface_id);
    }
}

impl FocusRouter {
    pub fn focus(&mut self, client_id: u64) {
        self.focused_client = Some(client_id);
    }

    pub fn recipients(&self, clients: &[u64]) -> Vec<u64> {
        clients
            .iter()
            .copied()
            .filter(|client| Some(*client) == self.focused_client)
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct HeadlessReport {
    pub schema: u32,
    pub configured: bool,
    pub frame_hash: String,
    pub input_events_sent: u32,
    pub close_sent: bool,
    pub disconnected_clients: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_hash_is_stable() {
        assert_eq!(
            expected_frame_hash(),
            "b71d8fd372b5947dd4bb9d23dde37f777ccc57547fa0fcd59a7e924ef69efc61",
        );
    }

    #[test]
    fn buffer_requires_matching_configure_ack() {
        let mut lifecycle = SurfaceLifecycle::default();
        assert_eq!(
            lifecycle.attach_buffer(),
            Err(ProtocolViolation::BufferBeforeConfigure)
        );

        lifecycle.configure(41);
        assert_eq!(
            lifecycle.ack_configure(40),
            Err(ProtocolViolation::UnknownConfigure)
        );
        lifecycle.ack_configure(41).unwrap();
        lifecycle.attach_buffer().unwrap();
        assert_eq!(lifecycle.phase(), SurfacePhase::Presented);
    }

    #[test]
    fn input_routes_only_to_focused_client() {
        let mut router = FocusRouter::default();
        router.focus(8);
        assert_eq!(router.recipients(&[7, 8, 9]), vec![8]);
    }

    #[test]
    fn surface_cannot_receive_xdg_role_twice() {
        let mut roles = RoleRegistry::default();
        roles.assign_xdg_surface(12).unwrap();
        assert_eq!(
            roles.assign_xdg_surface(12),
            Err(ProtocolViolation::RoleAlreadyAssigned)
        );
        roles.release_xdg_surface(12);
        roles.assign_xdg_surface(12).unwrap();
    }
}
