//! Real DRM/KMS output for the Pixel 7 panel, via Smithay's dumb-buffer
//! path -- deliberately no GBM/renderer_* features (ADR-008 already
//! avoided pulling GPU/mesa cross-compile into scope), matching
//! drm-splash.c's own CPU-composited, no-GPU approach. Only compiled with
//! the `panther-hardware` cargo feature.
//!
//! S03 Change step 4: prove the pipeline end to end (open device via
//! libseat, modeset, blit a client buffer's pixels onto the real panel).
//! Not wired into native-init.c yet (Change step 6) -- run standalone,
//! after native-init.c's own supervision (Change step 3) has given up on
//! drm-splash so nothing else holds /dev/dri/card0 as DRM master.

use std::os::fd::OwnedFd;
use std::path::Path;

use smithay::backend::allocator::dumb::DumbAllocator;
use smithay::backend::allocator::{Allocator, Fourcc};
use smithay::backend::drm::dumb::{framebuffer_from_dumb_buffer, DumbFramebuffer};
use smithay::backend::drm::{
    DrmDevice, DrmDeviceFd, DrmDeviceNotifier, DrmSurface, PlaneConfig, PlaneState,
};
use smithay::backend::session::libseat::{LibSeatSession, LibSeatSessionNotifier};
use smithay::backend::session::Session;
use smithay::reexports::drm::control::{
    connector, dumbbuffer::DumbBuffer as RawDumbBuffer, Device as ControlDevice,
};
use smithay::reexports::rustix::fs::OFlags;
use smithay::utils::{DeviceFd, Rectangle, Size, Transform};

const CARD_PATH: &str = "/dev/dri/card0";

pub struct HardwareOutput {
    drm_fd: DrmDeviceFd,
    surface: DrmSurface,
    plane: smithay::reexports::drm::control::plane::Handle,
    framebuffer: DumbFramebuffer,
    raw_handle: RawDumbBuffer,
    // Kept alive only for its Drop impl (destroys the kernel dumb buffer);
    // never read after allocation, writes go through `raw_handle` instead.
    _dumb: smithay::backend::allocator::dumb::DumbBuffer,
    _session: LibSeatSession,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
}

pub fn init() -> Result<(HardwareOutput, DrmDeviceNotifier, LibSeatSessionNotifier), String> {
    let (mut session, session_notifier) =
        LibSeatSession::new().map_err(|e| format!("libseat session failed: {e}"))?;
    let raw_fd: OwnedFd = session
        .open(Path::new(CARD_PATH), OFlags::RDWR)
        .map_err(|e| format!("failed to open {CARD_PATH} via libseat: {e}"))?;
    let drm_fd = DrmDeviceFd::new(DeviceFd::from(raw_fd));

    let (mut device, notifier) =
        DrmDevice::new(drm_fd.clone(), false).map_err(|e| format!("DrmDevice::new failed: {e}"))?;

    let resources = drm_fd
        .resource_handles()
        .map_err(|e| format!("resource_handles failed: {e}"))?;

    let mut chosen = None;
    for &conn_handle in resources.connectors() {
        let info = drm_fd
            .get_connector(conn_handle, false)
            .map_err(|e| format!("get_connector failed: {e}"))?;
        if info.state() != connector::State::Connected {
            continue;
        }
        let Some(&mode) = info.modes().first() else {
            continue;
        };
        for &enc_handle in info.encoders() {
            let encoder = drm_fd
                .get_encoder(enc_handle)
                .map_err(|e| format!("get_encoder failed: {e}"))?;
            if let Some(crtc_handle) = encoder.crtc() {
                chosen = Some((conn_handle, crtc_handle, mode));
                break;
            }
        }
        if chosen.is_some() {
            break;
        }
    }
    let (connector_handle, crtc_handle, mode) = chosen
        .ok_or_else(|| "no connected connector with a usable encoder/crtc found".to_string())?;
    let (mode_w, mode_h) = mode.size();
    println!(
        "saai-displayd: hardware output {}x{}@{}",
        mode_w,
        mode_h,
        mode.vrefresh()
    );

    let surface = device
        .create_surface(crtc_handle, mode, &[connector_handle])
        .map_err(|e| format!("create_surface failed: {e}"))?;

    let plane = surface
        .planes()
        .primary
        .first()
        .ok_or_else(|| "no primary plane available".to_string())?
        .handle;

    let mut allocator = DumbAllocator::new(drm_fd.clone());
    let dumb_buffer = allocator
        .create_buffer(mode_w as u32, mode_h as u32, Fourcc::Xrgb8888, &[])
        .map_err(|e| format!("dumb buffer allocation failed: {e}"))?;
    let raw_handle = *dumb_buffer.handle();
    let framebuffer = framebuffer_from_dumb_buffer(&drm_fd, &dumb_buffer, true)
        .map_err(|e| format!("framebuffer_from_dumb_buffer failed: {e}"))?;

    let stride = {
        let mut handle_copy = raw_handle;
        let mapping = drm_fd
            .map_dumb_buffer(&mut handle_copy)
            .map_err(|e| format!("initial map_dumb_buffer failed: {e}"))?;
        mapping.as_ref().len() as u32 / mode_h as u32
    };

    let mut output = HardwareOutput {
        drm_fd,
        surface,
        plane,
        framebuffer,
        raw_handle,
        _dumb: dumb_buffer,
        _session: session,
        width: mode_w as u32,
        height: mode_h as u32,
        stride,
    };
    output.fill(0x00, 0x00, 0x00);
    output.present(true)?;

    Ok((output, notifier, session_notifier))
}

impl HardwareOutput {
    /// Fills the whole framebuffer with one solid XRGB8888 color.
    pub fn fill(&mut self, r: u8, g: u8, b: u8) {
        let pixel = u32::from_be_bytes([0x00, r, g, b]);
        let mut handle_copy = self.raw_handle;
        let Ok(mut mapping) = self.drm_fd.map_dumb_buffer(&mut handle_copy) else {
            return;
        };
        let bytes = mapping.as_mut();
        for chunk in bytes.chunks_exact_mut(4) {
            chunk.copy_from_slice(&pixel.to_ne_bytes());
        }
    }

    /// Blits a client's XRGB8888/ARGB8888 buffer at (0, 0), clipped to the
    /// panel size -- no scaling, matching the smallest-verifiable-step cut
    /// of this milestone. Rest of the screen keeps whatever `fill` set.
    pub fn blit(&mut self, src: &[u8], src_width: u32, src_height: u32, src_stride: u32) {
        let mut handle_copy = self.raw_handle;
        let Ok(mut mapping) = self.drm_fd.map_dumb_buffer(&mut handle_copy) else {
            return;
        };
        let dst = mapping.as_mut();
        let copy_w = src_width.min(self.width) as usize;
        let copy_h = src_height.min(self.height) as usize;
        for row in 0..copy_h {
            let src_off = row * src_stride as usize;
            let dst_off = row * self.stride as usize;
            let n = copy_w * 4;
            dst[dst_off..dst_off + n].copy_from_slice(&src[src_off..src_off + n]);
        }
    }

    /// Pushes the current framebuffer contents to the panel. `modeset`
    /// forces a full `commit()` (needed once, at startup); afterwards a
    /// `page_flip()` is enough since the mode never changes.
    pub fn present(&self, modeset: bool) -> Result<(), String> {
        let config = PlaneConfig {
            src: Rectangle::from_size(Size::from((self.width as f64, self.height as f64))),
            dst: Rectangle::from_size(Size::from((self.width as i32, self.height as i32))),
            transform: Transform::Normal,
            alpha: 1.0,
            damage_clips: None,
            fb: *self.framebuffer.as_ref(),
            fence: None,
        };
        let planes = [PlaneState {
            handle: self.plane,
            config: Some(config),
        }];
        let result = if modeset {
            self.surface.commit(planes, false)
        } else {
            self.surface.page_flip(planes, false)
        };
        result.map_err(|e| format!("present failed: {e}"))
    }
}
