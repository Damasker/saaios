use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

#[cfg(target_os = "linux")]
pub mod panther;

pub const FRAME_WIDTH: u32 = 64;
pub const FRAME_HEIGHT: u32 = 48;
pub const FRAME_STRIDE: u32 = FRAME_WIDTH * 4;
pub const PANTHER_WIDTH: u32 = 1080;
pub const PANTHER_HEIGHT: u32 = 2400;

#[derive(Debug, Eq, PartialEq)]
pub enum FrameTransformError {
    InvalidGeometry,
    SourceTooSmall,
    TargetTooSmall,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameGeometry {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
}

pub fn scale_argb_to_panther_bgrx(
    source: &[u8],
    source_geometry: FrameGeometry,
    target: &mut [u8],
    target_geometry: FrameGeometry,
) -> Result<(), FrameTransformError> {
    let FrameGeometry {
        width: source_width,
        height: source_height,
        stride: source_stride,
    } = source_geometry;
    let FrameGeometry {
        width: target_width,
        height: target_height,
        stride: target_stride,
    } = target_geometry;
    if source_width == 0
        || source_height == 0
        || source_stride < source_width.saturating_mul(4)
        || target_width == 0
        || target_height == 0
        || target_stride < target_width.saturating_mul(4)
    {
        return Err(FrameTransformError::InvalidGeometry);
    }
    let source_len = source_stride as usize * source_height as usize;
    let target_len = target_stride as usize * target_height as usize;
    if source.len() < source_len {
        return Err(FrameTransformError::SourceTooSmall);
    }
    if target.len() < target_len {
        return Err(FrameTransformError::TargetTooSmall);
    }

    for y in 0..target_height {
        let source_y = y as usize * source_height as usize / target_height as usize;
        for x in 0..target_width {
            let source_x = x as usize * source_width as usize / target_width as usize;
            let source_offset = source_y * source_stride as usize + source_x * 4;
            let target_offset = y as usize * target_stride as usize + x as usize * 4;
            let blue = source[source_offset];
            let green = source[source_offset + 1];
            let red = source[source_offset + 2];
            target[target_offset] = 0;
            target[target_offset + 1] = red;
            target[target_offset + 2] = green;
            target[target_offset + 3] = blue;
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TouchBounds {
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
}

impl TouchBounds {
    pub fn normalize(self, x: i32, y: i32, width: u32, height: u32) -> (f64, f64) {
        fn axis(value: i32, min: i32, max: i32, extent: u32) -> f64 {
            if max <= min || extent == 0 {
                return 0.0;
            }
            let value = value.clamp(min, max) - min;
            value as f64 * extent.saturating_sub(1) as f64 / (max - min) as f64
        }
        (
            axis(x, self.min_x, self.max_x, width),
            axis(y, self.min_y, self.max_y, height),
        )
    }
}

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

pub fn demo_frame_with_touch(x: u32, y: u32) -> Vec<u8> {
    let mut frame = demo_frame();
    let left = x.saturating_sub(3);
    let right = x.saturating_add(3).min(FRAME_WIDTH - 1);
    let top = y.saturating_sub(3);
    let bottom = y.saturating_add(3).min(FRAME_HEIGHT - 1);
    for marker_y in top..=bottom {
        for marker_x in left..=right {
            if marker_x == x || marker_y == y {
                let offset = (marker_y * FRAME_STRIDE + marker_x * 4) as usize;
                // wl_shm Argb8888 little-endian bytes are BGRA; white is all 0xff.
                frame[offset..offset + 4].copy_from_slice(&[0xff, 0xff, 0xff, 0xff]);
            }
        }
    }
    frame
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
    fn touch_marker_produces_a_second_distinct_frame() {
        let touched = demo_frame_with_touch(12, 8);
        assert_ne!(sha256_hex(&touched), expected_frame_hash());
        let offset = (8 * FRAME_STRIDE + 12 * 4) as usize;
        assert_eq!(&touched[offset..offset + 4], &[0xff, 0xff, 0xff, 0xff]);
    }

    #[test]
    fn panther_transform_scales_and_swizzles_argb() {
        let source = [
            0x33, 0x22, 0x11, 0xff, 0x66, 0x55, 0x44, 0xff, 0x99, 0x88, 0x77, 0xff, 0xcc, 0xbb,
            0xaa, 0xff,
        ];
        let mut target = vec![0xee; 4 * 4 * 4];
        scale_argb_to_panther_bgrx(
            &source,
            FrameGeometry {
                width: 2,
                height: 2,
                stride: 8,
            },
            &mut target,
            FrameGeometry {
                width: 4,
                height: 4,
                stride: 16,
            },
        )
        .unwrap();
        assert_eq!(&target[0..4], &[0, 0x11, 0x22, 0x33]);
        assert_eq!(&target[12..16], &[0, 0x44, 0x55, 0x66]);
        assert_eq!(&target[48..52], &[0, 0x77, 0x88, 0x99]);
        assert_eq!(&target[60..64], &[0, 0xaa, 0xbb, 0xcc]);
    }

    #[test]
    fn touch_normalization_clamps_to_surface() {
        let bounds = TouchBounds {
            min_x: 10,
            max_x: 1090,
            min_y: 20,
            max_y: 2420,
        };
        assert_eq!(bounds.normalize(-10, 20, 64, 48), (0.0, 0.0));
        assert_eq!(bounds.normalize(1090, 3000, 64, 48), (63.0, 47.0));
        assert_eq!(bounds.normalize(550, 1220, 64, 48), (31.5, 23.5));
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
