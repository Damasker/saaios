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
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, OwnedFd};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use nix::fcntl::{fcntl, FcntlArg, FdFlag};
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
use smithay::reexports::drm::{CLOEXEC, RDWR};
use smithay::utils::{DeviceFd, Rectangle, Size, Transform};

const CARD_PATH: &str = "/dev/dri/card0";
const GPU_COMPOSITOR_PATH: &str = "/data/saaios/system/saai-gpu-compositor";
const GPU_PROTOCOL_MAGIC: u32 = 0x5347_5055;
const GPU_OP_BEGIN: u32 = 1;
const GPU_OP_BLIT: u32 = 2;
const GPU_OP_END: u32 = 3;

/// Bionic Vulkan child used as an ABI firewall around Google's Mali UMD.
/// The DRM dma-buf descriptors are inherited once at spawn; every frame
/// after that is a small command stream plus the wl_shm bytes that must be
/// uploaded to the GPU anyway.
struct GpuCompositor {
    child: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
    _prime_fds: [OwnedFd; 2],
}

impl GpuCompositor {
    fn set_cloexec(fd: &OwnedFd, enabled: bool) -> Result<(), String> {
        let raw_flags = fcntl(fd, FcntlArg::F_GETFD).map_err(|e| format!("F_GETFD failed: {e}"))?;
        let mut flags = FdFlag::from_bits_truncate(raw_flags);
        flags.set(FdFlag::FD_CLOEXEC, enabled);
        fcntl(fd, FcntlArg::F_SETFD(flags)).map_err(|e| format!("F_SETFD failed: {e}"))?;
        Ok(())
    }

    fn spawn(
        prime_fds: [OwnedFd; 2],
        width: u32,
        height: u32,
        stride: u32,
    ) -> Result<Self, String> {
        for fd in &prime_fds {
            Self::set_cloexec(fd, false)?;
        }
        let spawn_result = Command::new(GPU_COMPOSITOR_PATH)
            .args([
                width.to_string(),
                height.to_string(),
                stride.to_string(),
                prime_fds[0].as_raw_fd().to_string(),
                prime_fds[1].as_raw_fd().to_string(),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn();
        // The child already inherited these descriptors. Restore the
        // parent's normal close-on-exec policy before any later spawn.
        for fd in &prime_fds {
            let _ = Self::set_cloexec(fd, true);
        }
        let mut child =
            spawn_result.map_err(|e| format!("failed to start {GPU_COMPOSITOR_PATH}: {e}"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "GPU helper stdin unavailable".to_string())?;
        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| "GPU helper stdout unavailable".to_string())?;
        let mut ready = [0u8; 1];
        stdout
            .read_exact(&mut ready)
            .map_err(|e| format!("GPU helper readiness failed: {e}"))?;
        if ready[0] != b'R' {
            return Err(format!("GPU helper sent invalid readiness byte {ready:?}"));
        }
        eprintln!(
            "saai-displayd: Vulkan dma-buf compositor ready pid={}",
            child.id()
        );
        Ok(Self {
            child,
            stdin,
            stdout,
            _prime_fds: prime_fds,
        })
    }

    fn command(&mut self, words: [u32; 8], payload: Option<&[u8]>) -> Result<(), String> {
        let mut header = [0u8; 32];
        for (chunk, word) in header.chunks_exact_mut(4).zip(words) {
            chunk.copy_from_slice(&word.to_le_bytes());
        }
        self.stdin
            .write_all(&header)
            .map_err(|e| format!("GPU command write failed: {e}"))?;
        if let Some(bytes) = payload {
            self.stdin
                .write_all(bytes)
                .map_err(|e| format!("GPU payload write failed: {e}"))?;
        }
        self.stdin
            .flush()
            .map_err(|e| format!("GPU command flush failed: {e}"))?;
        let mut ack = [0u8; 1];
        self.stdout
            .read_exact(&mut ack)
            .map_err(|e| format!("GPU acknowledgement failed: {e}"))?;
        if ack[0] != b'K' {
            return Err(format!("GPU helper sent invalid acknowledgement {ack:?}"));
        }
        Ok(())
    }

    fn begin(&mut self, slot: usize, r: u8, g: u8, b: u8) -> Result<(), String> {
        let color = ((r as u32) << 16) | ((g as u32) << 8) | b as u32;
        self.command(
            [
                GPU_PROTOCOL_MAGIC,
                GPU_OP_BEGIN,
                slot as u32,
                0,
                0,
                0,
                0,
                color,
            ],
            None,
        )
    }

    fn blit(&mut self, src: &[u8], width: u32, height: u32, stride: u32) -> Result<(), String> {
        let length = (stride as usize)
            .checked_mul(height as usize)
            .ok_or_else(|| "GPU blit size overflow".to_string())?;
        let bytes = src
            .get(..length)
            .ok_or_else(|| format!("GPU blit source is short: {} < {length}", src.len()))?;
        self.command(
            [
                GPU_PROTOCOL_MAGIC,
                GPU_OP_BLIT,
                0,
                width,
                height,
                stride,
                length as u32,
                0,
            ],
            Some(bytes),
        )
    }

    fn end(&mut self, slot: usize) -> Result<(), String> {
        self.command(
            [GPU_PROTOCOL_MAGIC, GPU_OP_END, slot as u32, 0, 0, 0, 0, 0],
            None,
        )
    }
}

impl Drop for GpuCompositor {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

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
    gpu: Option<GpuCompositor>,
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

    let gpu = match (
        drm_fd.buffer_to_prime_fd(slot0.raw_handle.handle(), CLOEXEC | RDWR),
        drm_fd.buffer_to_prime_fd(slot1.raw_handle.handle(), CLOEXEC | RDWR),
    ) {
        (Ok(prime0), Ok(prime1)) => {
            match GpuCompositor::spawn([prime0, prime1], mode_w as u32, mode_h as u32, stride) {
                Ok(gpu) => Some(gpu),
                Err(error) => {
                    eprintln!("saai-displayd: GPU compositor unavailable: {error}; using CPU");
                    None
                }
            }
        }
        (first, second) => {
            eprintln!(
                "saai-displayd: DRM PRIME export failed ({:?}, {:?}); using CPU",
                first.err(),
                second.err()
            );
            None
        }
    };

    let mut output = HardwareOutput {
        drm_fd,
        surface,
        plane,
        slots: [slot0, slot1],
        gpu,
        write_index: 0,
        width: mode_w as u32,
        height: mode_h as u32,
        stride,
    };
    output.fill(0x00, 0x00, 0x00)?;
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
    pub fn fill(&mut self, r: u8, g: u8, b: u8) -> Result<(), String> {
        if let Some(gpu) = self.gpu.as_mut() {
            return gpu.begin(self.write_index, r, g, b);
        }
        let pixel = u32::from_be_bytes([0x00, r, g, b]);
        let Some(mut mapping) = self.map_write_slot() else {
            return Err("CPU scanout mapping failed".to_string());
        };
        let bytes = mapping.as_mut();
        for chunk in bytes.chunks_exact_mut(4) {
            chunk.copy_from_slice(&pixel.to_ne_bytes());
        }
        Ok(())
    }

    /// Blits a client's XRGB8888/ARGB8888 buffer at (0, 0) into the write
    /// buffer, clipped to the panel size -- no scaling, matching the
    /// smallest-verifiable-step cut of this milestone. Callers composite a
    /// full scene by calling this once per visible layer, background to
    /// foreground, before `present()`; this alone never clears the rest of
    /// the buffer (that's `fill()`'s job, called first).
    pub fn blit(
        &mut self,
        src: &[u8],
        src_width: u32,
        src_height: u32,
        src_stride: u32,
    ) -> Result<(), String> {
        let width = self.width;
        let height = self.height;
        let stride = self.stride;
        let copy_w = src_width.min(width);
        let copy_h = src_height.min(height);
        if let Some(gpu) = self.gpu.as_mut() {
            gpu.blit(src, copy_w, copy_h, src_stride)?;
            eprintln!(
                "saai-displayd: GPU blit submitted {copy_w}x{copy_h} px (src stride={src_stride})"
            );
            return Ok(());
        }
        let Some(mut mapping) = self.map_write_slot() else {
            return Err("CPU scanout mapping failed".to_string());
        };
        let dst = mapping.as_mut();
        let copy_w = copy_w as usize;
        let copy_h = copy_h as usize;
        for row in 0..copy_h {
            let src_off = row * src_stride as usize;
            let dst_off = row * stride as usize;
            let n = copy_w * 4;
            dst[dst_off..dst_off + n].copy_from_slice(&src[src_off..src_off + n]);
        }
        eprintln!(
            "saai-displayd: blit wrote {copy_w}x{copy_h} px into fb (dst stride={stride}, src stride={src_stride})"
        );
        Ok(())
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
        if let Some(gpu) = self.gpu.as_mut() {
            gpu.end(self.write_index)?;
        }
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
