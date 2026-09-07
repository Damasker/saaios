//! Real DRM/KMS output for the Pixel 7 panel, via Smithay's dumb-buffer
//! path -- deliberately no GBM/renderer_* features (ADR-008 already
//! avoided pulling GPU/mesa cross-compile into scope), matching
//! drm-splash.c's own CPU-composited, no-GPU approach. Only compiled with
//! the `panther-hardware` cargo feature.
//!
//! S03 Change step 4: prove the pipeline end to end (open device, modeset,
//! blit a client buffer's pixels onto the real panel). Not wired into
//! native-init.c yet (Change step 6) -- run standalone, after
//! native-init.c's own supervision (Change step 3) has given up on
//! drm-splash so nothing else holds /dev/dri/card0 as DRM master.
//!
//! /dev/dri/card0 is opened directly rather than through libseat
//! (ADR-010): this process is always root, is the only thing ever
//! allowed to hold the UI slot (ADR-009's single-owner invariant), and
//! never needs VT switching -- the three problems libseat exists to
//! solve. In testing, libseat's builtin backend failed to open the
//! device (EAGAIN), plausibly because this kernel's `console=ttynull`
//! leaves it no VT to manage.

use std::fs::OpenOptions;
use std::os::fd::OwnedFd;

use smithay::backend::allocator::dumb::DumbAllocator;
use smithay::backend::allocator::{Allocator, Fourcc};
use smithay::backend::drm::dumb::{framebuffer_from_dumb_buffer, DumbFramebuffer};
use smithay::backend::drm::{
    DrmDevice, DrmDeviceFd, DrmDeviceNotifier, DrmSurface, PlaneConfig, PlaneState,
};
use smithay::reexports::drm::control::{
    connector, dumbbuffer::DumbBuffer as RawDumbBuffer, Device as ControlDevice,
};
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
    pub width: u32,
    pub height: u32,
    pub stride: u32,
}

pub fn init() -> Result<(HardwareOutput, DrmDeviceNotifier), String> {
    let raw_fd: OwnedFd = OpenOptions::new()
        .read(true)
        .write(true)
        .open(CARD_PATH)
        .map_err(|e| format!("failed to open {CARD_PATH}: {e}"))?
        .into();
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
            // Prefer whatever crtc is already active on this encoder (e.g.
            // still set up by drm-splash), but that link only exists while
            // some process holds an active modeset -- after a hard kill
            // it's gone, so fall back to any crtc this encoder can legally
            // drive, exactly like a normal DRM client does on first setup.
            let crtc_handle = match encoder.crtc() {
                Some(crtc) => Some(crtc),
                None => resources
                    .filter_crtcs(encoder.possible_crtcs())
                    .first()
                    .copied(),
            };
            if let Some(crtc_handle) = crtc_handle {
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
    // An empty modifiers slice is *not* "no preference" here: Smithay's
    // own create_buffer() rejects it (Iterator::all() on an empty slice is
    // vacuously true, so the "is Linear/Invalid present" check always
    // fails -> EINVAL). Dumb buffers are inherently linear, so this is the
    // only modifier that could ever be valid anyway.
    let dumb_buffer = allocator
        .create_buffer(
            mode_w as u32,
            mode_h as u32,
            Fourcc::Xrgb8888,
            &[smithay::backend::allocator::Modifier::Linear],
        )
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
        width: mode_w as u32,
        height: mode_h as u32,
        stride,
    };
    output.fill(0x00, 0x00, 0x00);
    output.present(true)?;

    Ok((output, notifier))
}

impl HardwareOutput {
    /// Fills the whole framebuffer with one solid XRGB8888 color.
    pub fn fill(&mut self, r: u8, g: u8, b: u8) {
        let pixel = u32::from_be_bytes([0x00, r, g, b]);
        let mut handle_copy = self.raw_handle;
        let mut mapping = match self.drm_fd.map_dumb_buffer(&mut handle_copy) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("saai-displayd: fill: map_dumb_buffer failed: {e}");
                return;
            }
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
        let mut mapping = match self.drm_fd.map_dumb_buffer(&mut handle_copy) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("saai-displayd: blit: map_dumb_buffer failed: {e}");
                return;
            }
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
        eprintln!(
            "saai-displayd: blit wrote {copy_w}x{copy_h} px into fb (dst stride={}, src stride={src_stride})",
            self.stride
        );
    }

    /// Pushes the current framebuffer contents to the panel. `modeset`
    /// forces a full `commit()` (needed once, at startup); afterwards a
    /// `page_flip()` is enough since the mode never changes. Same fb handle
    /// every time (we mutate one dumb buffer in place rather than
    /// double-buffering) -- confirmed fine on this hardware: the exynos
    /// driver here doesn't implement dirty_framebuffer (ENOSYS) at all, so
    /// a flip to an unchanged fb id is the only signal it needs or supports.
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
