//! Deterministic software-rendered shell frame used by the host Wayland slice.

use sha2::{Digest, Sha256};
use std::fmt::Write;

pub const FRAME_WIDTH: u32 = 64;
pub const FRAME_HEIGHT: u32 = 48;
pub const FRAME_STRIDE: u32 = FRAME_WIDTH * 4;

pub fn shell_frame() -> Vec<u8> {
    let mut pixels = Vec::with_capacity((FRAME_STRIDE * FRAME_HEIGHT) as usize);
    for y in 0..FRAME_HEIGHT {
        for x in 0..FRAME_WIDTH {
            let header = y < 9;
            let navigation = y >= FRAME_HEIGHT - 8;
            let active_card = (8..56).contains(&x) && (15..35).contains(&y);
            let accent = (x + y) % 11 == 0;
            let (red, green, blue) = if header {
                (0x0c, 0x13, 0x20)
            } else if navigation {
                (0x12, 0x1c, 0x2b)
            } else if active_card && accent {
                (0x16, 0xc7, 0x84)
            } else if active_card {
                (0x23, 0x31, 0x46)
            } else {
                (0x08, 0x0d, 0x16)
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
    fn shell_frame_is_xrgb8888_and_deterministic() {
        let frame = shell_frame();
        assert_eq!(frame.len(), (FRAME_STRIDE * FRAME_HEIGHT) as usize);
        assert_eq!(
            frame_hash(&frame),
            "2bba4517e6540d3af0ec30fbcaf5818733d7e685807608f4189381069576155a"
        );
        assert!(frame.chunks_exact(4).all(|pixel| pixel[3] == 0xff));
    }
}
