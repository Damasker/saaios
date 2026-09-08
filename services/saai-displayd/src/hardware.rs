//! Real DRM/KMS output for the Pixel 7 panel, via Smithay's dumb-buffer
//! path -- deliberately no GBM/renderer_* features (ADR-008 already
//! avoided pulling GPU/mesa cross-compile into scope), matching
//! drm-splash.c's own CPU-composited, no-GPU approach. Only compiled with
//! the `panther-hardware` cargo feature.
//!
//! S03 Change step 4 proved the pipeline end to end with a single,
//! in-place-mutated buffer. ADR-016 replaced that with real double
//! buffering: two dumb buffers, CPU always writes into whichever one
//! isn't the buffer most recently submitted to the DRM plane, so a
//! `blit()` never races the DPU's scanout of the buffer still on
//! screen. `present()` toggles which buffer is "current" the moment a
//! flip is *submitted*, not when it's confirmed complete -- safe only
//! because callers (`main.rs`) never submit a second flip before the
//! first one's completion event arrives (`flip_pending` gating), so by
//! the time a buffer's turn to be written into comes around again, its
//! *previous* flip is guaranteed long done.
//!
//! Not wired into native-init.c yet (Change step 6) -- run standalone,
//! after native-init.c's own supervision (Change step 3) has given up
//! on drm-splash so nothing else holds /dev/dri/card0 as DRM master.
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
use smithay::reexports::drm::buffer::Buffer as DrmBufferTrait;
use smithay::reexports::drm::control::{
    connector, dumbbuffer::DumbBuffer as RawDumbBuffer, Device as ControlDevice,
};
use smithay::utils::{DeviceFd, Rectangle, Size, Transform};

const CARD_PATH: &str = "/dev/dri/card0";

/// One of the two buffers in the double-buffer pair.
struct Slot {
    framebuffer: DumbFramebuffer,
    raw_handle: RawDumbBuffer,
    // Kept alive only for its Drop impl (destroys the kernel dumb buffer);
    // never read after allocation, writes go through `raw_handle` instead.
    _dumb: smithay::backend::allocator::dumb::DumbBuffer,
}

pub struct HardwareOutput {
    drm_fd: DrmDeviceFd,
    surface: DrmSurface,
    plane: smithay::reexports::drm::control::plane::Handle,
    slots: [Slot; 2],
    /// Index into `slots` that CPU writes (`fill`/`blit`) target right
    /// now. Toggled by `present()` the moment a flip is submitted --
    /// see the module doc comment for why that's safe under the
    /// caller's flip_pending gating.
    write_index: usize,
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
    let alloc_one = |allocator: &mut DumbAllocator| -> Result<(Slot, u32), String> {
        // An empty modifiers slice is *not* "no preference" here: Smithay's
        // own create_buffer() rejects it (Iterator::all() on an empty slice
        // is vacuously true, so the "is Linear/Invalid present" check
        // always fails -> EINVAL). Dumb buffers are inherently linear, so
        // this is the only modifier that could ever be valid anyway.
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
        // The kernel's own CREATE_DUMB response, not derived from anything
        // -- deriving it from `mmap`'s mapped length (as this used to) is
        // wrong: mmap rounds the mapping up to whole pages, so
        // `len() / height` only recovers the true per-row stride when
        // height happens to divide the padded size evenly. On the real
        // 1080x2400 panel it silently returned 4321 instead of 4320
        // (1080 * 4) -- a 1-byte-per-row drift that compounds down the
        // buffer and showed up as a diagonal shear/wash-out across the
        // whole frame.
        let stride = raw_handle.pitch();
        Ok((
            Slot {
                framebuffer,
                raw_handle,
                _dumb: dumb_buffer,
            },
            stride,
        ))
    };
    let (slot0, stride) = alloc_one(&mut allocator)?;
    let (slot1, stride1) = alloc_one(&mut allocator)?;
    debug_assert_eq!(stride, stride1, "both dumb buffers must share one stride");

    let mut output = HardwareOutput {
        drm_fd,
        surface,
        plane,
        slots: [slot0, slot1],
        write_index: 0,
        width: mode_w as u32,
        height: mode_h as u32,
        stride,
    };
    output.fill(0x00, 0x00, 0x00);
    output.present(true)?;

    Ok((output, notifier))
}

impl HardwareOutput {
    fn map_write_slot(
        &mut self,
    ) -> Option<smithay::reexports::drm::control::dumbbuffer::DumbMapping<'_>> {
        let handle = &mut self.slots[self.write_index].raw_handle;
        match self.drm_fd.map_dumb_buffer(handle) {
            Ok(m) => Some(m),
            Err(e) => {
                eprintln!("saai-displayd: map_dumb_buffer failed: {e}");
                None
            }
        }
    }

    /// Fills the whole write buffer with one solid XRGB8888 color. Used as
    /// the scene's background layer -- see `main.rs`'s `recomposite()`.
    pub fn fill(&mut self, r: u8, g: u8, b: u8) {
        let pixel = u32::from_be_bytes([0x00, r, g, b]);
        let Some(mut mapping) = self.map_write_slot() else {
            return;
        };
        let bytes = mapping.as_mut();
        for chunk in bytes.chunks_exact_mut(4) {
            chunk.copy_from_slice(&pixel.to_ne_bytes());
        }
    }

    /// Blits a client's XRGB8888/ARGB8888 buffer at (0, 0) into the write
    /// buffer, clipped to the panel size -- no scaling, matching the
    /// smallest-verifiable-step cut of this milestone. Callers composite a
    /// full scene by calling this once per visible layer, background to
    /// foreground, before `present()`; this alone never clears the rest of
    /// the buffer (that's `fill()`'s job, called first).
    pub fn blit(&mut self, src: &[u8], src_width: u32, src_height: u32, src_stride: u32) {
        let width = self.width;
        let height = self.height;
        let stride = self.stride;
        let Some(mut mapping) = self.map_write_slot() else {
            return;
        };
        let dst = mapping.as_mut();
        let copy_w = src_width.min(width) as usize;
        let copy_h = src_height.min(height) as usize;
        for row in 0..copy_h {
            let src_off = row * src_stride as usize;
            let dst_off = row * stride as usize;
            let n = copy_w * 4;
            dst[dst_off..dst_off + n].copy_from_slice(&src[src_off..src_off + n]);
        }
        eprintln!(
            "saai-displayd: blit wrote {copy_w}x{copy_h} px into fb (dst stride={stride}, src stride={src_stride})"
        );
    }

    /// Submits the write buffer to the panel and toggles which buffer CPU
    /// writes target next. `modeset` forces a full `commit()` (needed once,
    /// at startup); afterwards a `page_flip()` is enough since the mode
    /// never changes. Always requests a completion event (`event: true`) --
    /// callers (`main.rs`) must not call this again until that event
    /// arrives (`DrmEvent::VBlank` via the `DrmDeviceNotifier`), which is
    /// also what makes toggling `write_index` here, at submission time
    /// rather than at confirmed completion, safe: see the module doc
    /// comment.
    pub fn present(&mut self, modeset: bool) -> Result<(), String> {
        let config = PlaneConfig {
            src: Rectangle::from_size(Size::from((self.width as f64, self.height as f64))),
            dst: Rectangle::from_size(Size::from((self.width as i32, self.height as i32))),
            transform: Transform::Normal,
            alpha: 1.0,
            damage_clips: None,
            fb: *self.slots[self.write_index].framebuffer.as_ref(),
            fence: None,
        };
        let planes = [PlaneState {
            handle: self.plane,
            config: Some(config),
        }];
        let result = if modeset {
            self.surface.commit(planes, true)
        } else {
            self.surface.page_flip(planes, true)
        };
        result.map_err(|e| format!("present failed: {e}"))?;
        self.write_index = 1 - self.write_index;
        Ok(())
    }
}
