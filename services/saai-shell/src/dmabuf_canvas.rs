//! DRM dumb-buffer + `linux-dmabuf` backed CPU-writable canvas: an
//! alternative to smithay-client-toolkit's `wl_shm`-based `SlotPool`/
//! `Buffer` for surfaces that want to skip `saai-displayd`'s host-visible
//! staging copy on every commit (ADR-024's "GPU-native linux-dmabuf
//! client path"). Allocation mirrors the technique already proven on
//! device by `saai-displayd`'s own `examples/dmabuf_probe.rs`, but calls
//! the `drm` crate's `control::Device` trait directly instead of going
//! through `smithay::backend::allocator` -- the full `smithay` crate
//! pulls in an unconditional (non-optional, regardless of feature
//! flags) dependency on `xkbcommon`, which this crate's own `Cargo.toml`
//! has deliberately avoided from the start (no physical keyboard on this
//! device, same crash-risk/image-size reasoning as ADR-012/013 -- adding
//! `smithay` broke the real aarch64-musl static cross-build at link time
//! with a missing `libxkbcommon.a`, even though the host-target `cargo
//! check` never caught it). `drm` alone has no such baggage.
//!
//! Double-buffered, like `SlotPool` already is for every existing
//! `wl_shm` surface in this app: a dumb buffer just committed to the
//! compositor must not be overwritten with new pixels until the
//! compositor sends `wl_buffer`'s `release` event (it may still be mid
//! GPU-blit-read from it) -- unlike a fresh `SlotPool` buffer, nothing
//! here makes that mistake impossible by construction, so it is tracked
//! explicitly per slot.

use std::fs::{File, OpenOptions};
use std::os::fd::{AsFd, BorrowedFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use drm::buffer::{Buffer as DrmBufferTrait, DrmFourcc};
use drm::control::{dumbbuffer::DumbBuffer as RawDumbBuffer, Device as ControlDevice};
use drm::Device as BasicDevice;
use drm::{CLOEXEC, RDWR};
use smithay_client_toolkit::reexports::client::protocol::wl_buffer;
use smithay_client_toolkit::reexports::client::{Dispatch, QueueHandle};
use wayland_protocols::wp::linux_dmabuf::zv1::client::{zwp_linux_buffer_params_v1, zwp_linux_dmabuf_v1};

const DRM_DEVICE: &str = "/dev/dri/card0";
/// XRGB8888 is always 32 bits per pixel -- `create_dumb_buffer` needs
/// this explicitly (unlike the higher-level `smithay::backend::
/// allocator::Allocator` trait this module intentionally does not use,
/// which derives it from the fourcc internally).
const XRGB8888_BPP: u32 = 32;
/// Close-on-exec + read/write flags for the exported PRIME fd, same
/// values `dmabuf_probe.rs` uses -- `drm::{CLOEXEC, RDWR}` re-exports
/// these directly, no need to hand-roll the raw `O_*` bits.
const PRIME_FD_FLAGS: u32 = CLOEXEC | RDWR;

/// Minimal DRM device handle: this app never becomes DRM master (that
/// stays `saai-displayd`'s alone, ADR-024) -- allocating dumb buffers
/// and exporting them as PRIME fds needs no master access, exactly as
/// `dmabuf_probe.rs` already demonstrated. `drm::Device`/`drm::control::
/// Device` are pure extension-method traits over `AsFd`; no extra state
/// needed beyond the open file.
struct DrmCard(File);

impl AsFd for DrmCard {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}
impl BasicDevice for DrmCard {}
impl ControlDevice for DrmCard {}

/// Per-buffer "may the client safely overwrite this yet" flag, shared
/// with the `wl_buffer`'s own Wayland user-data. `wayland-client`'s
/// `Dispatch` trait must be implemented on the app's actual state type
/// (`Shell`, in `main.rs`), not on this module's `DmabufCanvas` -- its
/// impl there should just call [`handle_release`] with the event and
/// the `Busy` user-data it receives.
pub type Busy = Arc<AtomicBool>;

/// Call from `Dispatch<wl_buffer::WlBuffer, Busy>::event` in `main.rs`
/// for every event on a `wl_buffer` created by this module.
pub fn handle_release(data: &Busy, event: &wl_buffer::Event) {
    if matches!(event, wl_buffer::Event::Release) {
        data.store(false, Ordering::Release);
    }
}

struct Slot {
    raw: RawDumbBuffer,
    wl_buffer: wl_buffer::WlBuffer,
    busy: Busy,
}

/// A double-buffered `linux-dmabuf` canvas for one surface. Geometry is
/// fixed at construction; call `ensure_size` (idempotent -- only
/// reallocates if the size actually changed) before every `paint`.
pub struct DmabufCanvas {
    drm: DrmCard,
    width: u32,
    height: u32,
    pitch: usize,
    slots: Vec<Slot>,
    next: usize,
}

impl DmabufCanvas {
    pub fn new() -> Result<Self, String> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(DRM_DEVICE)
            .map_err(|error| format!("open {DRM_DEVICE}: {error}"))?;
        Ok(Self {
            drm: DrmCard(file),
            width: 0,
            height: 0,
            pitch: 0,
            slots: Vec::new(),
            next: 0,
        })
    }

    pub fn pitch(&self) -> usize {
        self.pitch
    }

    fn free_slots(&mut self) {
        for slot in self.slots.drain(..) {
            // Best-effort: an already-invalid handle (e.g. the DRM
            // device somehow went away) isn't worth failing over here,
            // there is nothing left to recover.
            let _ = self.drm.destroy_dumb_buffer(slot.raw);
        }
    }

    /// Idempotent: a no-op if already allocated at this exact size (the
    /// common case -- call this unconditionally at the top of every
    /// redraw, exactly like `SlotPool::create_buffer` is only actually
    /// invoked when `self.layer_buffer.is_none()`). Only when the size
    /// really changes does it destroy the old slots' real kernel dumb
    /// buffers (the compositor might still hold `wl_buffer` references
    /// to them and keep working until it releases/destroys those --
    /// this only stops issuing new frames from the old ones, it does
    /// not force-revoke what the compositor already has) and allocate
    /// two fresh dumb buffers + `wl_buffer`s at the new size.
    pub fn ensure_size<D>(
        &mut self,
        width: u32,
        height: u32,
        dmabuf: &zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1,
        qh: &QueueHandle<D>,
    ) -> Result<(), String>
    where
        D: Dispatch<wl_buffer::WlBuffer, Busy>
            + Dispatch<zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1, ()>
            + 'static,
    {
        let (width, height) = (width.max(1), height.max(1));
        if !self.slots.is_empty() && self.width == width && self.height == height {
            return Ok(());
        }
        self.free_slots();
        self.width = width;
        self.height = height;
        self.next = 0;
        let first = self.alloc_slot(dmabuf, qh)?;
        let second = self.alloc_slot(dmabuf, qh)?;
        self.slots.push(first);
        self.slots.push(second);
        Ok(())
    }

    fn alloc_slot<D>(
        &mut self,
        dmabuf: &zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1,
        qh: &QueueHandle<D>,
    ) -> Result<Slot, String>
    where
        D: Dispatch<wl_buffer::WlBuffer, Busy>
            + Dispatch<zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1, ()>
            + 'static,
    {
        let raw = self
            .drm
            .create_dumb_buffer((self.width, self.height), DrmFourcc::Xrgb8888, XRGB8888_BPP)
            .map_err(|error| format!("DRM dumb buffer alloc: {error}"))?;
        self.pitch = raw.pitch() as usize;
        let prime_fd = self
            .drm
            .buffer_to_prime_fd(raw.handle(), PRIME_FD_FLAGS)
            .map_err(|error| format!("PRIME export: {error}"))?;
        let params = dmabuf.create_params(qh, ());
        params.add(prime_fd.as_fd(), 0, 0, self.pitch as u32, 0, 0);
        let busy: Busy = Arc::new(AtomicBool::new(false));
        let wl_buffer = params.create_immed(
            self.width as i32,
            self.height as i32,
            DrmFourcc::Xrgb8888 as u32,
            zwp_linux_buffer_params_v1::Flags::empty(),
            qh,
            busy.clone(),
        );
        Ok(Slot {
            raw,
            wl_buffer,
            busy,
        })
    }

    /// Picks the next slot in ping-pong order, refusing to hand back one
    /// still marked busy (the compositor has not released it yet -- for
    /// a two-slot rotation this should only ever happen if a surface is
    /// redrawn faster than the compositor can flip, which none of this
    /// app's current call sites do).
    fn acquire(&mut self) -> Result<usize, String> {
        for _ in 0..self.slots.len() {
            let index = self.next;
            self.next = (self.next + 1) % self.slots.len();
            if !self.slots[index].busy.load(Ordering::Acquire) {
                return Ok(index);
            }
        }
        Err("both dma-buf slots still busy (compositor has not released either yet)".into())
    }

    /// Maps the next available slot, lets `paint` write pixels into it
    /// (`stride` = `self.pitch()`, matching what `render::draw_status_bar`
    /// already expects from a `SlotPool` canvas), marks it busy, and
    /// returns the `wl_buffer` to attach/damage/commit exactly like a
    /// `SlotPool::create_buffer`'s output.
    pub fn paint(
        &mut self,
        paint: impl FnOnce(&mut [u8]),
    ) -> Result<&wl_buffer::WlBuffer, String> {
        let index = self.acquire()?;
        let logical_len = self.pitch * self.height as usize;
        let slot = &mut self.slots[index];
        {
            let mut mapping = self
                .drm
                .map_dumb_buffer(&mut slot.raw)
                .map_err(|error| format!("map dumb buffer: {error}"))?;
            // The kernel's GEM mmap rounds the mapped length up to a whole
            // number of pages (physically confirmed on device: a
            // 1080x120 XRGB8888 buffer mmaps as 520192 bytes -- 127 pages
            // -- not the logical 518400 = pitch*height), but
            // `render::Canvas::new` asserts its slice is exactly
            // width*height*4. Truncate to the buffer's real logical size
            // before handing it to the caller -- `pitch == width*4` on
            // this device (confirmed: no row padding), so this slice is
            // exactly what Canvas expects, tightly packed.
            let mapped = mapping.as_mut();
            let logical_len = logical_len.min(mapped.len());
            paint(&mut mapped[..logical_len]);
        }
        slot.busy.store(true, Ordering::Release);
        Ok(&slot.wl_buffer)
    }
}

impl Drop for DmabufCanvas {
    fn drop(&mut self) {
        self.free_slots();
    }
}
