use std::cell::RefCell;
use std::collections::HashMap;
#[cfg(any(test, feature = "panther-hardware"))]
use std::collections::VecDeque;
#[cfg(not(feature = "panther-hardware"))]
use std::io::BufRead;
#[cfg(feature = "panther-hardware")]
use std::process::{Child, Command};
use std::rc::Rc;
#[cfg(feature = "panther-hardware")]
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
#[cfg(any(test, feature = "panther-hardware"))]
use std::time::{Duration, Instant};

#[cfg(feature = "panther-hardware")]
use calloop::signals::{Signal, Signals};
#[cfg(feature = "panther-hardware")]
use nix::sys::socket::{getsockopt, sockopt::PeerCredentials};

mod data_device;
#[cfg(feature = "panther-hardware")]
mod hardware;
mod hid;
mod layer_geom;
mod text_ime;
#[cfg(feature = "panther-hardware")]
mod touch;

use sha2::{Digest, Sha256};
#[cfg(feature = "panther-hardware")]
use smithay::desktop::utils::send_frames_surface_tree;
#[cfg(not(feature = "panther-hardware"))]
use smithay::input::keyboard::Keycode;
#[cfg(not(feature = "panther-hardware"))]
use smithay::input::keyboard::{FilterResult, XkbConfig};
use smithay::{
    backend::allocator::{dmabuf::Dmabuf, Buffer as AllocatorBuffer, Format, Fourcc, Modifier},
    delegate_compositor, delegate_dmabuf, delegate_fractional_scale, delegate_layer_shell,
    delegate_output, delegate_seat, delegate_session_lock, delegate_shm, delegate_viewporter,
    delegate_xdg_shell,
    input::{Seat, SeatHandler, SeatState},
    output::{Mode as OutputMode, Output, PhysicalProperties, Scale, Subpixel},
    reexports::{
        calloop::{generic::Generic, EventLoop, Interest, Mode, PostAction},
        wayland_server::{
            backend::ClientData,
            protocol::{
                wl_buffer::WlBuffer, wl_output::WlOutput, wl_seat::WlSeat, wl_surface::WlSurface,
            },
            Client, Display, DisplayHandle, Resource,
        },
    },
    utils::Serial,
    utils::Transform,
    utils::SERIAL_COUNTER,
    wayland::{
        buffer::BufferHandler,
        compositor::{
            with_states, BufferAssignment, CompositorClientState, CompositorHandler,
            CompositorState, SurfaceAttributes,
        },
        dmabuf::{get_dmabuf, DmabufGlobal, DmabufHandler, DmabufState, ImportNotifier},
        fractional_scale::{
            with_fractional_scale, FractionalScaleHandler, FractionalScaleManagerState,
        },
        output::OutputHandler,
        session_lock::{LockSurface, SessionLockHandler, SessionLockManagerState, SessionLocker},
        shell::{
            wlr_layer::{
                Anchor, LayerSurface, LayerSurfaceCachedState, WlrLayerShellHandler,
                WlrLayerShellState,
            },
            xdg::{PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState},
        },
        shm::{with_buffer_contents, ShmHandler, ShmState},
        socket::ListeningSocketSource,
        viewporter::ViewporterState,
    },
};

/// Matches `Scale::Integer(1)` on the advertised output. GDK initializes
/// its shm height from `wp_fractional_scale_v1.preferred_scale` in 120ths
/// (1.0 → 120). Without that event the pointer stays uninitialized
/// (ADR-025 height=1776831).
const PREFERRED_FRACTIONAL_SCALE: f64 = 1.0;

/// Laptop surface (PCE-25). Smaller than the host output; not a phone panel.
const WINDOWED_WIDTH: i32 = 1280;
const WINDOWED_HEIGHT: i32 = 800;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ToplevelGeometry {
    width: i32,
    height: i32,
    fullscreen: bool,
}

fn output_model_for(phone_gate: bool) -> &'static str {
    if phone_gate {
        "panther"
    } else {
        "x86"
    }
}

fn toplevel_geometry_for(
    phone_gate: bool,
    output_width: i32,
    output_height: i32,
) -> ToplevelGeometry {
    if phone_gate {
        ToplevelGeometry {
            width: output_width,
            height: output_height,
            fullscreen: true,
        }
    } else {
        ToplevelGeometry {
            width: WINDOWED_WIDTH,
            height: WINDOWED_HEIGHT,
            fullscreen: false,
        }
    }
}

#[derive(Default)]
struct SaaiClientState {
    compositor_state: CompositorClientState,
    #[cfg(feature = "panther-hardware")]
    peer_pid: u32,
}
impl ClientData for SaaiClientState {}

/// One surface's last committed frame, cached for `State::recomposite()`
/// (ADR-016). Plain owned pixels, not a reference into the Wayland
/// buffer -- the buffer itself may be reused/destroyed by the client
/// well before this compositor needs to redraw the scene again (e.g.
/// on unlock, with no new commit from anyone involved).
#[cfg(feature = "panther-hardware")]
enum FrameBacking {
    Pixels(Vec<u8>),
    Dmabuf(DmabufFrame),
}

/// Holding the wl_buffer until this cached frame is replaced is required by
/// the Wayland lifetime contract: a release event tells the client it may
/// overwrite/reuse the allocation. The Dmabuf clone keeps the fd valid for
/// recomposition; this guard emits release when the frame leaves the scene
/// cache.
#[cfg(feature = "panther-hardware")]
struct DmabufFrame {
    buffer: WlBuffer,
    dmabuf: Dmabuf,
}

#[cfg(feature = "panther-hardware")]
impl Drop for DmabufFrame {
    fn drop(&mut self) {
        self.buffer.release();
    }
}

#[cfg(feature = "panther-hardware")]
struct SurfaceFrame {
    backing: FrameBacking,
    width: u32,
    height: u32,
    stride: u32,
    /// SHA-256 of `pixels`, already computed by `commit()` for every
    /// buffer anyway -- lets a repeat commit with byte-identical content
    /// (real and reproducible under rapid input; see the OOM
    /// investigation this field was added for) skip the ~10MB copy this
    /// cache would otherwise take on every single raw commit, not just
    /// ones that actually change the picture.
    // DMA-BUF contents may change between commits while retaining the same
    // fd and geometry, so only copied wl_shm frames can be deduplicated by
    // content hash.
    digest: Option<[u8; 32]>,
    /// ADR/S-follow-up (found necessary once "Я"'s drag-to-scroll made
    /// `recomposite()` run far more often than any tap-driven redraw
    /// ever did): a monotonic counter bumped every time `commit()`
    /// stores a genuinely new `SurfaceFrame` for a surface, Pixels or
    /// Dmabuf alike. Unlike `digest`, this works for dma-buf frames
    /// too -- `recomposite()` uses it to skip re-blitting a surface
    /// into a scanout buffer slot that already has this exact
    /// generation's content in it, at the cost of trusting a
    /// well-behaved client not to rewrite a dma-buf's pixels without a
    /// matching new commit (this compositor's own first-party clients
    /// never do; one that did would show a stale frame for at most one
    /// more flip, not corrupt anything).
    generation: u64,
}

#[cfg(feature = "panther-hardware")]
fn blit_surface_frame(
    hardware: &mut hardware::HardwareOutput,
    frame: &SurfaceFrame,
    dst_x: i32,
    dst_y: i32,
) -> Result<(), String> {
    match &frame.backing {
        FrameBacking::Pixels(pixels) => hardware.blit(
            pixels,
            frame.width,
            frame.height,
            frame.stride,
            dst_x,
            dst_y,
        ),
        FrameBacking::Dmabuf(frame) => hardware.blit_dmabuf(&frame.dmabuf, dst_x, dst_y),
    }
}

/// Skips re-blitting `surface`'s current frame into the physical
/// scanout slot `write_index` is about to write into, if that exact
/// slot already has this frame's generation blit into it from an
/// earlier `recomposite()` -- see `SurfaceFrame::generation`'s own doc
/// comment for why a generation counter (not `digest`) is what makes
/// this safe for dma-buf frames too, and `State::slot_generations`'s
/// for why per-slot (not just "last blit anywhere") tracking matters
/// with two alternating physical buffers. Takes its pieces of `State`
/// as separate arguments rather than `&mut State` itself, since
/// `recomposite()`'s own `hw` is a `self.hardware`-derived borrow that
/// has to stay alive across several calls -- passing `&mut self` here
/// too would conflict with that.
#[cfg(feature = "panther-hardware")]
fn blit_if_changed(
    hw: &mut hardware::HardwareOutput,
    surface_frames: &HashMap<WlSurface, SurfaceFrame>,
    slot_generations: &mut [HashMap<WlSurface, u64>; 2],
    write_index: usize,
    surface: &WlSurface,
    dst_x: i32,
    dst_y: i32,
) -> Result<(), String> {
    let Some(frame) = surface_frames.get(surface) else {
        return Ok(());
    };
    if slot_generations[write_index].get(surface) == Some(&frame.generation) {
        return Ok(());
    }
    blit_surface_frame(hw, frame, dst_x, dst_y)?;
    slot_generations[write_index].insert(surface.clone(), frame.generation);
    Ok(())
}

/// Whether a commit with `new_digest` actually changes what's on screen
/// for a surface whose previously cached frame hashed to `previous`.
/// `None` (nothing cached yet, e.g. a surface's first commit) always
/// counts as new content.
#[cfg(feature = "panther-hardware")]
fn is_new_content(previous: Option<[u8; 32]>, new_digest: [u8; 32]) -> bool {
    Some(new_digest) != previous
}

#[cfg(all(test, feature = "panther-hardware"))]
mod surface_frame_dedup_tests {
    use super::is_new_content;

    #[test]
    fn first_commit_with_nothing_cached_is_new() {
        assert!(is_new_content(None, [1u8; 32]));
    }

    #[test]
    fn matching_digest_is_not_new() {
        assert!(!is_new_content(Some([7u8; 32]), [7u8; 32]));
    }

    #[test]
    fn differing_digest_is_new() {
        assert!(is_new_content(Some([7u8; 32]), [8u8; 32]));
    }
}

struct State {
    compositor_state: CompositorState,
    shm_state: ShmState,
    dmabuf_state: DmabufState,
    // Kept so future renderer-specific feedback can update or disable the
    // same global without rediscovering it. The protocol objects own their
    // own dispatch state after creation.
    _dmabuf_global: DmabufGlobal,
    xdg_shell_state: XdgShellState,
    seat_state: SeatState<State>,
    seat: Seat<State>,
    /// ADR-021: GTK4's `_gdk_wayland_display_open()` requires
    /// `wl_data_device_manager`. ADR-294 owns the global so smithay
    /// cannot broker native copy/paste. Portal clipboard stays the
    /// capability-gated channel (AUTH-08).
    _data_device_manager: data_device::SaaiDataDeviceManager,
    /// ADR-267 (APP-03): our text-input-v3 + input-method-v2 pair, not
    /// smithay's `InputMethodManagerState` (that path unwraps a keyboard,
    /// ADR-022). Keep-alive for the globals, same as `_dmabuf_global`.
    _text_input_manager_state: text_ime::SaaiTextInputManager,
    _input_method_manager_state: text_ime::SaaiInputMethodManager,
    /// ADR-266 (APP-02): `wp_fractional_scale_manager_v1` so GTK4/GDK
    /// receives `preferred_scale` instead of an uninitialized `double *`
    /// (ADR-025). Same keep-alive pattern as `_text_input_manager_state`.
    _fractional_scale_manager_state: FractionalScaleManagerState,
    /// Pair protocol GDK expects alongside fractional-scale. Viewport
    /// dest size is not applied to the DRM blit path in this slice;
    /// advertising the global is enough for the client to bind.
    _viewporter_state: ViewporterState,
    // No keyboard capability on the real Pixel 7 build (ADR-012): this
    // device has no physical keyboard, and drm-splash.c's own on-screen
    // keyboard proves this architecture never needed wl_keyboard/xkbcommon
    // keysym translation to begin with. Kept for the headless S02 build,
    // where the keyboard-focus/synthetic-inject acceptance tests still use
    // a real KeyboardHandle and a normal host libxkbcommon works fine.
    #[cfg(not(feature = "panther-hardware"))]
    keyboard: smithay::input::keyboard::KeyboardHandle<State>,
    #[cfg(not(feature = "panther-hardware"))]
    pointer: smithay::input::pointer::PointerHandle<State>,
    /// Active fullscreen toplevel. A newly mapped toplevel becomes active;
    /// destroying it restores the previous live toplevel (normally the
    /// persistent system shell).
    focused_surface: Option<WlSurface>,
    /// Mapping order doubles as a minimal focus stack for the S05
    /// single-fullscreen-app model. Existing surfaces cannot steal focus by
    /// merely committing another frame.
    focus_history: Vec<WlSurface>,
    /// wl_surface id -> its ToplevelSurface handle, so commit() can call
    /// ensure_configured() (S02 protocol-negative test: reject a buffer
    /// attached before the surface's first configure was acked).
    toplevels: HashMap<WlSurface, ToplevelSurface>,
    #[cfg(feature = "panther-hardware")]
    hardware: Option<hardware::HardwareOutput>,
    /// Every known surface's last committed frame (ADR-016) -- lets
    /// `recomposite()` rebuild the whole visible scene (background ->
    /// app -> system layers -> lock) on demand, instead of the old
    /// approach of blitting whatever surface happened to commit most
    /// recently and hoping nothing else needed redrawing. Not pruned
    /// aggressively: entries for destroyed surfaces are removed
    /// opportunistically (toplevel/layer destroy handlers), a leaked
    /// entry just wastes a little memory, it can't cause a stale
    /// surface to render (recomposite() only ever looks up frames for
    /// surfaces still referenced by focused_surface/layer_surfaces/
    /// lock_surface).
    #[cfg(feature = "panther-hardware")]
    surface_frames: HashMap<WlSurface, SurfaceFrame>,
    /// Bumped and assigned to a `SurfaceFrame::generation` every time
    /// `commit()` stores a genuinely new one -- see that field's own
    /// doc comment.
    #[cfg(feature = "panther-hardware")]
    next_frame_generation: u64,
    /// Which `SurfaceFrame::generation` is currently blit into each of
    /// the two physical scanout buffer slots (`HardwareOutput` always
    /// double-buffers exactly two, indexed by its own `write_index`) --
    /// `recomposite()` skips re-blitting a surface whose current
    /// generation already matches what's recorded here for the slot
    /// it's about to write into. Same small, bounded, never-explicitly-
    /// cleaned-up leak character as `surface_frames` itself for a
    /// destroyed surface's stale entry -- see that field's own doc
    /// comment for why that's an accepted tradeoff here, not a bug.
    #[cfg(feature = "panther-hardware")]
    slot_generations: [HashMap<WlSurface, u64>; 2],
    /// Set the moment a page-flip is submitted (`HardwareOutput::present`),
    /// cleared when its completion event arrives (`DrmEvent::VBlank`,
    /// see main()). While set, `recomposite()` must not run again --
    /// `HardwareOutput` only double-buffers two frames deep, and
    /// submitting a second flip before the first completes is exactly
    /// the EBUSY bug ADR-016 was written to fix.
    #[cfg(feature = "panther-hardware")]
    flip_pending: bool,
    /// Set when something wanted to recomposite while `flip_pending`
    /// was already true -- the VBlank handler checks this and runs
    /// exactly one more recomposite() once the in-flight flip
    /// completes, coalescing any number of scene changes that arrived
    /// mid-flip into a single follow-up repaint instead of queuing one
    /// per change.
    #[cfg(feature = "panther-hardware")]
    repaint_needed: bool,
    /// Surfaces included in the in-flight DRM frame. Their Wayland frame
    /// callbacks are completed only after the kernel reports VBlank.
    #[cfg(feature = "panther-hardware")]
    pending_frame_surfaces: Vec<WlSurface>,
    /// ADR-024 continued (full async submit pipeline): a surface's
    /// previous `Dmabuf`-backed frame, replaced by a newer commit while
    /// a flip that may still be reading it (via GPU-compositor's own
    /// async fence wait) is in flight. `present()` no longer blocks
    /// until the GPU actually finishes with a frame's dma-bufs before
    /// returning, so a `DmabufFrame` still in the in-flight flip's
    /// scene cannot be allowed to `Drop` (and so `wl_buffer.release()`)
    /// the moment a client happens to commit again for that surface --
    /// only once `DrmEvent::VBlank` confirms the flip (and therefore
    /// the fence the kernel waited on for it) is actually done is it
    /// safe. Conservative on purpose: anything replaced while
    /// `flip_pending` is true lands here regardless of whether it was
    /// actually part of that specific flip's scene, in exchange for
    /// not needing to track that precisely.
    #[cfg(feature = "panther-hardware")]
    pending_frame_trash: Vec<FrameBacking>,
    /// ADR-014: the compositor owns the system shell process. Keeping the
    /// Child handle here makes that ownership explicit and gives the next
    /// supervision step a single place to observe/restart it.
    #[cfg(feature = "panther-hardware")]
    shell_child: Option<Child>,
    #[cfg(feature = "panther-hardware")]
    shell_socket_name: String,
    #[cfg(feature = "panther-hardware")]
    shell_restart_budget: RestartBudget,
    #[cfg(feature = "panther-hardware")]
    privileged_shell_pid: Arc<AtomicU32>,
    #[cfg(feature = "panther-hardware")]
    presentation_started: Instant,
    #[cfg(feature = "panther-hardware")]
    touch: smithay::input::touch::TouchHandle<State>,
    /// Kept alive for the lifetime of the process -- not because the
    /// wl_output global depends on it (smithay's global dispatch data
    /// owns what's needed to answer protocol requests independently),
    /// but so a future mode change is a method call away rather than a
    /// rediscovery.
    _wl_output: Output,
    /// Real dimensions the client sees via wl_output/lock surfaces --
    /// pixel7's real panel size, or a generic placeholder for the
    /// headless S02 build.
    output_width: i32,
    output_height: i32,
    session_lock_state: SessionLockManagerState,
    /// ADR-015's core security invariant lives here, not in the protocol
    /// alone: ext-session-lock-v1 only gives us the lock/unlock lifecycle,
    /// *we* still have to make sure input actually stops reaching
    /// `focused_surface` while locked. See the touch routing in main().
    locked: bool,
    lock_surface: Option<LockSurface>,
    layer_shell_state: WlrLayerShellState,
    /// Known layer surfaces (status bar / OSK). Placement follows the
    /// client's size and anchor (ADR-271). Touch hits the topmost layer
    /// under the contact, then `focused_surface`. Locked still wins.
    layer_surfaces: Vec<LayerSurface>,
}

/// S32's follow-up (docs/os/ideas.md): moved off the fixed 8MB
/// init_boot ramdisk onto the same persistent /data volume
/// saai-appd/saai-entityd/saaios-runtime already use -- the same
/// "independent of the fixed-size image" reasoning, just applied to
/// the two remaining occupants (this one and saai-displayd itself,
/// native-init.c's own DISPLAYD_PATH). If this path is ever missing
/// (a fresh /data with no bootstrap copy yet), spawn_shell()'s
/// std::process::Command::spawn() fails cleanly -- launch_shell()'s
/// own RestartBudget/SHELL_RESTART_LIMIT already handles that by
/// exiting this process, letting PID 1's own UI-slot restart budget
/// fall back to drm-splash (ADR-009), same safety net that already
/// covered a crashing saai-shell before this move.
#[cfg(feature = "panther-hardware")]
const SAAI_SHELL_PATH: &str = "/data/saaios/system/saai-shell";

#[cfg(any(test, feature = "panther-hardware"))]
const SHELL_RESTART_LIMIT: usize = 3;
#[cfg(any(test, feature = "panther-hardware"))]
const SHELL_RESTART_WINDOW: Duration = Duration::from_secs(60);
#[cfg(feature = "panther-hardware")]
const SHELL_FAILURE_EXIT_CODE: i32 = 71;

#[cfg(any(test, feature = "panther-hardware"))]
#[derive(Default)]
struct RestartBudget {
    failures: VecDeque<Instant>,
}

#[cfg(any(test, feature = "panther-hardware"))]
impl RestartBudget {
    fn record_failure(&mut self, now: Instant) -> usize {
        while self
            .failures
            .front()
            .is_some_and(|failure| now.duration_since(*failure) >= SHELL_RESTART_WINDOW)
        {
            self.failures.pop_front();
        }
        self.failures.push_back(now);
        self.failures.len()
    }
}

#[cfg(feature = "panther-hardware")]
fn spawn_shell(socket_name: &str) -> Result<Child, String> {
    Command::new(SAAI_SHELL_PATH)
        .env("WAYLAND_DISPLAY", socket_name)
        .spawn()
        .map_err(|error| format!("failed to start {SAAI_SHELL_PATH}: {error}"))
}

#[cfg(feature = "panther-hardware")]
fn is_privileged_shell(client: &Client, shell_pid: &AtomicU32) -> bool {
    let expected = shell_pid.load(Ordering::Acquire);
    expected != 0
        && client
            .get_data::<SaaiClientState>()
            .is_some_and(|data| data.peer_pid == expected)
}

impl State {
    fn activate_toplevel(&mut self, surface: Option<WlSurface>) {
        if self.focused_surface == surface {
            return;
        }
        self.focused_surface = surface.clone();
        // ADR-267: text-input focus is "which client's field is live", not
        // "which client gets key events". Same seat, no keyboard required.
        text_ime::on_focus(&self.seat, surface.clone());
        #[cfg(not(feature = "panther-hardware"))]
        {
            let serial = SERIAL_COUNTER.next_serial();
            let keyboard = self.keyboard.clone();
            keyboard.set_focus(self, surface.clone(), serial);
            println!(
                "saai-displayd: keyboard focus set to {:?}",
                surface.as_ref().map(Resource::id)
            );
        }
        #[cfg(feature = "panther-hardware")]
        {
            println!(
                "saai-displayd: focus set to {:?}",
                surface.as_ref().map(Resource::id)
            );
            self.request_recomposite();
        }
    }

    fn layer_client_size_anchor(
        surface: &LayerSurface,
        pending: bool,
    ) -> (i32, i32, bool, bool, bool, bool) {
        with_states(surface.wl_surface(), |states| {
            let mut guard = states.cached_state.get::<LayerSurfaceCachedState>();
            let cached = if pending {
                *guard.pending()
            } else {
                *guard.current()
            };
            (
                cached.size.w,
                cached.size.h,
                cached.anchor.contains(Anchor::TOP),
                cached.anchor.contains(Anchor::BOTTOM),
                cached.anchor.contains(Anchor::LEFT),
                cached.anchor.contains(Anchor::RIGHT),
            )
        })
    }

    #[cfg_attr(not(feature = "panther-hardware"), allow(dead_code))]
    fn layer_geom(&self, surface: &LayerSurface) -> layer_geom::LayerGeom {
        let (requested_w, requested_h, top, bottom, left, right) =
            Self::layer_client_size_anchor(surface, false);
        let (width, height) = layer_geom::configure_size(
            self.output_width,
            self.output_height,
            requested_w,
            requested_h,
        );
        layer_geom::destination(
            self.output_width,
            self.output_height,
            width,
            height,
            top,
            bottom,
            left,
            right,
        )
    }

    #[cfg_attr(not(feature = "panther-hardware"), allow(dead_code))]
    fn touch_focus_at(
        &self,
        x: f64,
        y: f64,
    ) -> Option<(
        WlSurface,
        smithay::utils::Point<f64, smithay::utils::Logical>,
    )> {
        use smithay::utils::Point;
        if self.locked {
            return self
                .lock_surface
                .as_ref()
                .map(|ls| (ls.wl_surface().clone(), Point::from((0.0, 0.0))));
        }
        for layer in self.layer_surfaces.iter().rev() {
            let geom = self.layer_geom(layer);
            if geom.contains(x, y) {
                return Some((
                    layer.wl_surface().clone(),
                    Point::from((f64::from(geom.x), f64::from(geom.y))),
                ));
            }
        }
        self.focused_surface
            .clone()
            .map(|s| (s, Point::from((0.0, 0.0))))
    }
}

#[cfg(feature = "panther-hardware")]
impl State {
    fn launch_shell(&mut self) -> bool {
        loop {
            match spawn_shell(&self.shell_socket_name) {
                Ok(child) => {
                    self.privileged_shell_pid
                        .store(child.id(), Ordering::Release);
                    println!("saai-displayd: started saai-shell pid={}", child.id());
                    self.shell_child = Some(child);
                    return true;
                }
                Err(error) => {
                    let failures = self.shell_restart_budget.record_failure(Instant::now());
                    eprintln!(
                        "saai-displayd: {error}; shell failure {failures}/{SHELL_RESTART_LIMIT}"
                    );
                    if failures >= SHELL_RESTART_LIMIT {
                        return false;
                    }
                }
            }
        }
    }

    fn handle_shell_sigchld(&mut self) -> bool {
        let status = match self.shell_child.as_mut() {
            Some(child) => match child.try_wait() {
                Ok(status) => status,
                Err(error) => {
                    eprintln!("saai-displayd: failed to inspect saai-shell: {error}");
                    return true;
                }
            },
            None => return true,
        };
        let Some(status) = status else {
            return true;
        };
        self.shell_child = None;
        self.privileged_shell_pid.store(0, Ordering::Release);
        let failures = self.shell_restart_budget.record_failure(Instant::now());
        eprintln!(
            "saai-displayd: saai-shell exited ({status}); shell failure {failures}/{SHELL_RESTART_LIMIT}"
        );
        if failures >= SHELL_RESTART_LIMIT {
            return false;
        }
        self.launch_shell()
    }
}

#[cfg(feature = "panther-hardware")]
impl State {
    /// Ask for a scene recomposite, respecting the one-flip-in-flight
    /// rule (ADR-016): runs immediately if nothing is already pending,
    /// otherwise just remembers to run exactly once more when the
    /// in-flight flip's completion event arrives (see the DRM notifier
    /// handler in main()).
    fn request_recomposite(&mut self) {
        if self.flip_pending {
            self.repaint_needed = true;
        } else {
            self.recomposite();
        }
    }

    /// Rebuilds the visible scene from cached per-surface frames and
    /// presents it, in a fixed background -> app -> system layers ->
    /// lock order. Replaces the old approach of blitting whatever
    /// surface happened to commit most recently: that approach could
    /// never correctly redraw after unlock (nothing remembered the
    /// toplevel's or layer surfaces' content once the lock surface had
    /// painted over them), and offered no path to composite more than
    /// one non-lock surface at a time -- both needed for Change 6's
    /// four root sections.
    ///
    /// Must only be called when `!self.flip_pending` -- callers go
    /// through `request_recomposite()`, not this directly, except the
    /// DRM notifier's VBlank handler which already checked.
    /// ADR-024 continued: called wherever a surface's cached
    /// `SurfaceFrame` is about to be replaced (or removed outright).
    /// A `Pixels`-backed frame's bytes were already copied out of its
    /// `wl_buffer` at commit time (see the comment above
    /// `with_buffer_contents` in `commit()`), so it is always safe to
    /// drop immediately regardless of any in-flight flip. A
    /// `Dmabuf`-backed frame is not: it stays a live reference into the
    /// client's own buffer, which `present()` (this file's async
    /// `HardwareOutput::present()`, ADR-024) may still have an
    /// in-flight GPU read against even after returning. If a flip is
    /// currently pending, defer it to `pending_frame_trash` instead of
    /// letting it `Drop` (and so `wl_buffer.release()`) here --
    /// `DrmEvent::VBlank` clears that trash once the flip (and the
    /// fence the kernel waited on for it) is confirmed done.
    fn stash_or_drop_old_frame(&mut self, old: Option<SurfaceFrame>) {
        let Some(old) = old else { return };
        if self.flip_pending && matches!(old.backing, FrameBacking::Dmabuf(_)) {
            self.pending_frame_trash.push(old.backing);
        }
        // else: `old` (and its `backing`) drops here normally -- always
        // safe for `Pixels`, and safe for `Dmabuf` when no flip is
        // pending.
    }

    fn recomposite(&mut self) {
        let layer_blits: Vec<(WlSurface, i32, i32)> = self
            .layer_surfaces
            .iter()
            .map(|layer| {
                let geom = self.layer_geom(layer);
                (layer.wl_surface().clone(), geom.x, geom.y)
            })
            .collect();
        let Some(hw) = self.hardware.as_mut() else {
            return;
        };
        // `fill()` clears the complete target slot, so none of the layer
        // generations previously recorded for that slot remain present.
        // Keeping those entries made `blit_if_changed` skip an unchanged
        // status bar after the clear: the next main-surface frame covered
        // the screen and the bar appeared to vanish while scrolling.
        // Invalidate the slot immediately; every visible layer must be
        // composited again after a full clear.
        let write_index = hw.write_index();
        if let Err(error) = hw.fill(0x00, 0x00, 0x00) {
            eprintln!("saai-displayd: hardware frame begin failed: {error}");
            std::process::exit(72);
        }
        self.slot_generations[write_index].clear();
        let mut shown: Vec<WlSurface> = Vec::new();
        if self.locked {
            if let Some(s) = self.lock_surface.as_ref().map(|ls| ls.wl_surface().clone()) {
                if let Err(error) = blit_if_changed(
                    hw,
                    &self.surface_frames,
                    &mut self.slot_generations,
                    write_index,
                    &s,
                    0,
                    0,
                ) {
                    eprintln!("saai-displayd: hardware lock blit failed: {error}");
                    std::process::exit(72);
                }
                if self.surface_frames.contains_key(&s) {
                    shown.push(s);
                }
            }
        } else {
            if let Some(s) = self.focused_surface.clone() {
                if let Err(error) = blit_if_changed(
                    hw,
                    &self.surface_frames,
                    &mut self.slot_generations,
                    write_index,
                    &s,
                    0,
                    0,
                ) {
                    eprintln!("saai-displayd: hardware toplevel blit failed: {error}");
                    std::process::exit(72);
                }
                if self.surface_frames.contains_key(&s) {
                    shown.push(s);
                }
            }
            for (s, dst_x, dst_y) in &layer_blits {
                if let Err(error) = blit_if_changed(
                    hw,
                    &self.surface_frames,
                    &mut self.slot_generations,
                    write_index,
                    s,
                    *dst_x,
                    *dst_y,
                ) {
                    eprintln!("saai-displayd: hardware layer blit failed: {error}");
                    std::process::exit(72);
                }
                if self.surface_frames.contains_key(s) {
                    shown.push(s.clone());
                }
            }
        }
        match hw.present(false) {
            Ok(()) => {
                self.flip_pending = true;
                self.pending_frame_surfaces = shown;
            }
            Err(err) => eprintln!("saai-displayd: hardware present failed: {err}"),
        }
    }
}

impl CompositorHandler for State {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client
            .get_data::<SaaiClientState>()
            .unwrap()
            .compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        if let Some(layer) = self
            .layer_surfaces
            .iter()
            .find(|layer| layer.wl_surface() == surface)
            .cloned()
        {
            let (requested_w, requested_h, _, _, _, _) =
                Self::layer_client_size_anchor(&layer, false);
            let (width, height) = layer_geom::configure_size(
                self.output_width,
                self.output_height,
                requested_w,
                requested_h,
            );
            layer.with_pending_state(|state| {
                state.size = Some((width, height).into());
            });
            layer.send_pending_configure();
        }
        // Frame callbacks (`wl_surface.frame`) are acknowledged after
        // the DRM VBlank for the scene submitted by recomposite(), not
        // unconditionally here on every commit
        // -- an earlier version of this fix did ack them here, and
        // physically exposed a real bug it wasn't looking for:
        // saai-shell's CompositorHandler::frame() redraws and requests
        // another callback on every ack, a normal "stay in sync with
        // the compositor" pattern that had silently never fired before
        // (frame callbacks didn't work at all until that first fix).
        // Acking synchronously on every raw commit meant the toplevel's
        // static placeholder redrew as fast as commits round-tripped.
        // Delivery after VBlank gives animated clients correct pacing;
        // saai-shell itself is event-driven and does not request another
        // frame until its content actually changes.
        let buffer = with_states(surface, |states| {
            let mut guard = states.cached_state.get::<SurfaceAttributes>();
            match &guard.current().buffer {
                Some(BufferAssignment::NewBuffer(buffer)) => Some(buffer.clone()),
                _ => None,
            }
        });

        let Some(buffer) = buffer else {
            println!(
                "saai-displayd: commit on surface {:?} (no buffer)",
                surface.id()
            );
            return;
        };

        // S02 protocol-negative test: xdg-shell requires a surface to have
        // its initial configure acked before any buffer-carrying commit --
        // only relevant once we know this commit actually carries one.
        // ensure_configured() checks this and, on violation, posts
        // xdg_surface::Error::NotConstructed itself -- the offending client
        // gets disconnected with a protocol error, everyone else is
        // unaffected.
        if let Some(toplevel) = self.toplevels.get(surface) {
            if !toplevel.ensure_configured() {
                println!(
                    "saai-displayd: rejected commit on surface {:?} -- buffer attached before initial configure was acked",
                    surface.id()
                );
                return;
            }
        }

        if let Ok(dmabuf) = get_dmabuf(&buffer).cloned() {
            // Mapping, not every later commit, changes focus. Keep this
            // identical to the wl_shm path below.
            if self.toplevels.contains_key(surface) && !self.focus_history.contains(surface) {
                self.focus_history.push(surface.clone());
                self.activate_toplevel(Some(surface.clone()));
            }

            #[cfg(feature = "panther-hardware")]
            {
                let size = dmabuf.size();
                let stride = dmabuf
                    .strides()
                    .next()
                    .expect("validated dma-buf always has one stride");
                let format = dmabuf.format();
                println!(
                    "saai-displayd: commit on surface {:?}, linux-dmabuf {}x{} stride={} format={:?} modifier={:?}",
                    surface.id(),
                    size.w,
                    size.h,
                    stride,
                    format.code,
                    format.modifier
                );
                let generation = self.next_frame_generation;
                self.next_frame_generation += 1;
                let old = self.surface_frames.insert(
                    surface.clone(),
                    SurfaceFrame {
                        backing: FrameBacking::Dmabuf(DmabufFrame { buffer, dmabuf }),
                        width: size.w as u32,
                        height: size.h as u32,
                        stride,
                        digest: None,
                        generation,
                    },
                );
                self.stash_or_drop_old_frame(old);
                let affects_scene = if self.locked {
                    self.lock_surface.as_ref().map(|ls| ls.wl_surface()) == Some(surface)
                } else {
                    self.focused_surface.as_ref() == Some(surface)
                        || self
                            .layer_surfaces
                            .iter()
                            .any(|ls| ls.wl_surface() == surface)
                };
                if affects_scene {
                    self.request_recomposite();
                }
            }
            #[cfg(not(feature = "panther-hardware"))]
            {
                drop(dmabuf);
                buffer.release();
            }
            return;
        }

        // Real, reproducible under rapid input (ADR-049): Wayland can
        // legitimately deliver more than one raw commit in a row for the
        // same surface with byte-identical content --
        // request_recomposite()'s flip_pending gating already coalesces
        // the *redraw* side of that into one repaint, but nothing skipped
        // the ~10MB copy+cache-replace below, so each redundant commit
        // still paid for one anyway. The digest is already computed here
        // for every commit regardless -- comparing it against what's
        // already cached for this surface costs nothing extra and lets a
        // matching commit skip the copy entirely.
        //
        // Beyond that: even a commit that *does* change the picture used
        // to pay for a brand new `Vec<u8>` every time (`bytes.to_vec()`),
        // immediately dropping whatever was cached before -- alloc/free
        // churn of a full framebuffer-sized block on every single
        // genuinely new frame, still enough on its own to grow RSS under
        // sustained rapid redraw (ADR-049's own physical test still
        // reached 670MB from the ~20% of commits that *were* real content
        // changes, even with duplicates already skipped). Taking the
        // previous frame's `Vec` out of the map here and writing the new
        // bytes into it via `clear()` + `extend_from_slice()` reuses its
        // existing allocation whenever the new content fits the same
        // capacity (the overwhelmingly common case -- a surface's frame
        // size does not usually change commit to commit) instead of
        // allocating and freeing a new one every time. A brand new
        // surface (nothing to take) still allocates once, unavoidably.
        #[cfg(feature = "panther-hardware")]
        let previous_frame = self.surface_frames.remove(surface);
        #[cfg(feature = "panther-hardware")]
        let previous_digest = previous_frame.as_ref().and_then(|frame| frame.digest);
        #[cfg(feature = "panther-hardware")]
        let previous_generation = previous_frame.as_ref().map(|frame| frame.generation);
        #[cfg(feature = "panther-hardware")]
        let mut reused_pixels = match previous_frame {
            Some(SurfaceFrame {
                backing: FrameBacking::Pixels(pixels),
                ..
            }) => pixels,
            Some(frame) => {
                // Old backing was `Dmabuf` -- this surface just fell
                // back from GPU-native dma-buf to wl_shm (or similar).
                // Same deferred-release hazard `stash_or_drop_old_
                // frame`'s own doc comment describes for the dma-buf
                // commit branch above; route through the same helper
                // rather than dropping `frame` here unconditionally.
                self.stash_or_drop_old_frame(Some(frame));
                Vec::new()
            }
            None => Vec::new(),
        };

        let result = with_buffer_contents(&buffer, move |ptr, len, data| {
            // `with_buffer_contents` hands back the *pool's* pointer and
            // length (its own doc comment says so explicitly), not this
            // buffer's -- a pool backs every buffer a client has ever
            // created on it and only grows, so using `len` directly here
            // hashed and copied the whole, ever-growing pool on every
            // single commit instead of just this frame's own pixels
            // (ADR-052: measured 4x-8x the real ~10MB frame size and
            // climbing over a session, accounting for the multi-hundred-
            // millisecond per-tap lag this caused). This buffer's own
            // window into that pool is `data.offset..+data.stride*
            // data.height`; falling back to an empty slice on a bounds
            // mismatch is defensive only -- a well-behaved client (the
            // only kind this compositor talks to) never triggers it.
            let buffer_len = data.stride as usize * data.height as usize;
            let end = (data.offset as usize).saturating_add(buffer_len);
            let bytes: &[u8] = if end <= len {
                unsafe { std::slice::from_raw_parts(ptr.add(data.offset as usize), buffer_len) }
            } else {
                eprintln!(
                    "saai-displayd: commit buffer geometry (offset={} len={buffer_len}) exceeds pool length {len}, skipping",
                    data.offset
                );
                &[]
            };
            let mut hasher = Sha256::new();
            hasher.update(bytes);
            let digest = hasher.finalize();
            #[cfg(feature = "panther-hardware")]
            {
                let digest_bytes: [u8; 32] = digest
                    .as_slice()
                    .try_into()
                    .expect("sha256 digest is always 32 bytes");
                if is_new_content(previous_digest, digest_bytes) {
                    reused_pixels.clear();
                    reused_pixels.extend_from_slice(bytes);
                }
            }
            #[cfg(feature = "panther-hardware")]
            let pixels = reused_pixels;
            #[cfg(not(feature = "panther-hardware"))]
            let pixels = ();
            (
                digest,
                pixels,
                data.width as u32,
                data.height as u32,
                data.stride as u32,
            )
        });

        // The compositor owns a complete copy after with_buffer_contents()
        // returns. Release the client buffer immediately so SlotPool can
        // safely reuse it for the next page. Without this event saai-shell
        // kept every full-screen slot permanently busy; on hardware that
        // produced commits whose pixels occasionally still represented the
        // previous tab even though current_page had already changed.
        buffer.release();

        // Mapping, not every commit, changes focus. This makes a newly
        // launched S05 application visible while preventing a background
        // client animation from stealing focus later.
        if self.toplevels.contains_key(surface) && !self.focus_history.contains(surface) {
            self.focus_history.push(surface.clone());
            self.activate_toplevel(Some(surface.clone()));
        }

        match result {
            Ok((digest, _pixels, _width, _height, _stride)) => {
                println!(
                    "saai-displayd: commit on surface {:?}, frame sha256={:x}",
                    surface.id(),
                    digest
                );
                #[cfg(feature = "panther-hardware")]
                {
                    // Every known surface's frame is cached regardless
                    // of `self.locked` -- this client requests its
                    // initial lock immediately at startup, in the same
                    // burst of requests as creating the toplevel, so by
                    // the time the toplevel's *real* first commit (with
                    // an actual buffer) reaches the server, locked is
                    // often already true; gating the cache on
                    // `!self.locked` (an earlier version of this code)
                    // meant the very first boot's toplevel frame was
                    // never cached at all, which is exactly the case
                    // that matters most for redraw-after-unlock.
                    //
                    // `_pixels` is the same `Vec` that was already cached
                    // for this surface (or a fresh, empty one for a
                    // surface committing for the first time), reused
                    // in-place by the closure above -- reinserting it
                    // here is a cheap struct move, not a copy; the
                    // expensive byte copy already happened (or was
                    // skipped) inside `with_buffer_contents`.
                    let digest_bytes: [u8; 32] = digest
                        .as_slice()
                        .try_into()
                        .expect("sha256 digest is always 32 bytes");
                    let generation = if is_new_content(previous_digest, digest_bytes) {
                        let generation = self.next_frame_generation;
                        self.next_frame_generation += 1;
                        generation
                    } else {
                        // Byte-identical repeat commit -- keep the same
                        // generation so recomposite() recognizes this
                        // surface as unchanged and skips re-blitting it,
                        // instead of a fresh generation making it look
                        // like new content on every duplicate commit.
                        previous_generation.unwrap_or(0)
                    };
                    self.surface_frames.insert(
                        surface.clone(),
                        SurfaceFrame {
                            backing: FrameBacking::Pixels(_pixels),
                            width: _width,
                            height: _height,
                            stride: _stride,
                            digest: Some(digest_bytes),
                            generation,
                        },
                    );
                    // While locked, only the lock surface affects the
                    // visible scene -- otherwise the toplevel
                    // underneath could keep animating on screen even
                    // though it can no longer receive input, which would
                    // be a visible break of the "locked means locked"
                    // expectation even though the security property
                    // (touch routing) is already correctly enforced
                    // elsewhere.
                    let affects_scene = if self.locked {
                        self.lock_surface.as_ref().map(|ls| ls.wl_surface()) == Some(surface)
                    } else {
                        self.focused_surface.as_ref() == Some(surface)
                            || self
                                .layer_surfaces
                                .iter()
                                .any(|ls| ls.wl_surface() == surface)
                    };
                    if affects_scene {
                        self.request_recomposite();
                    }
                }
            }
            Err(err) => eprintln!("saai-displayd: failed to hash committed buffer: {err}"),
        }
    }
}
delegate_compositor!(State);

impl BufferHandler for State {
    fn buffer_destroyed(&mut self, _buffer: &WlBuffer) {}
}

impl ShmHandler for State {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}
delegate_shm!(State);

/// The first linux-dmabuf contract is intentionally narrow: one linear
/// XRGB/ARGB plane with no offset. It exactly matches both the Pixel panel's
/// current scanout layout and the Vulkan buffer-copy path. Advertising only
/// formats we can consume prevents clients from selecting tiled/compressed
/// allocations that would need format-modifier-aware image sampling.
fn validate_dmabuf(dmabuf: &Dmabuf) -> Result<(), &'static str> {
    let size = dmabuf.size();
    if size.w <= 0 || size.h <= 0 {
        return Err("non-positive dimensions");
    }
    if dmabuf.num_planes() != 1 {
        return Err("only single-plane buffers are supported");
    }
    let format = dmabuf.format();
    if !matches!(format.code, Fourcc::Xrgb8888 | Fourcc::Argb8888) {
        return Err("unsupported pixel format");
    }
    if !matches!(format.modifier, Modifier::Linear | Modifier::Invalid) {
        return Err("only linear buffers are supported");
    }
    if dmabuf.offsets().next() != Some(0) {
        return Err("non-zero plane offsets are not supported");
    }
    let Some(stride) = dmabuf.strides().next() else {
        return Err("missing plane stride");
    };
    if stride < size.w as u32 * 4 {
        return Err("plane stride is shorter than one pixel row");
    }
    if dmabuf.y_inverted() {
        return Err("Y-inverted buffers are not supported yet");
    }
    Ok(())
}

impl DmabufHandler for State {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.dmabuf_state
    }

    fn dmabuf_imported(
        &mut self,
        _global: &DmabufGlobal,
        dmabuf: Dmabuf,
        notifier: ImportNotifier,
    ) {
        match validate_dmabuf(&dmabuf) {
            Ok(()) => {
                if let Err(error) = notifier.successful::<State>() {
                    eprintln!("saai-displayd: failed to create linux-dmabuf wl_buffer: {error}");
                }
            }
            Err(reason) => {
                eprintln!("saai-displayd: rejected linux-dmabuf import: {reason}");
                notifier.failed();
            }
        }
    }
}
delegate_dmabuf!(State);

impl SeatHandler for State {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<State> {
        &mut self.seat_state
    }
}
delegate_seat!(State);

// ADR-294: native Wayland clipboard is deny-by-default. The global still
// exists so GTK4 can open a display (ADR-021). Smithay's data-device
// path is not used — it cannot refuse SetSelection or Receive (ADR-023).
delegate_saai_data_device!(State);

// ADR-267 (APP-03): text-input-v3 + input-method-v2 without keymap.
// Focus is still driven from `activate_toplevel()`.
delegate_saai_text_ime!(State);

// ADR-266 (APP-02): send preferred_scale=1.0 as soon as a client binds
// wp_fractional_scale_v1 on a surface. Output is already Scale::Integer(1);
// this is the client-facing event GDK actually waits for.
impl FractionalScaleHandler for State {
    fn new_fractional_scale(&mut self, surface: WlSurface) {
        with_states(&surface, |states| {
            with_fractional_scale(states, |fractional_scale| {
                fractional_scale.set_preferred_scale(PREFERRED_FRACTIONAL_SCALE);
            });
        });
    }
}
delegate_fractional_scale!(State);
delegate_viewporter!(State);

impl XdgShellHandler for State {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let geo = toplevel_geometry_for(
            cfg!(feature = "panther-hardware"),
            self.output_width,
            self.output_height,
        );
        println!(
            "saai-displayd: new xdg_toplevel {}x{} fullscreen={}",
            geo.width, geo.height, geo.fullscreen
        );
        // Panther stays the phone panel. x86 is a window inside the host
        // output, not a second panther (ADR-250).
        // ADR-311: smithay serializes missing bounds as configure_bounds(0,0).
        // GTK 4.14 then gdk_toplevel_size_init(0,0) and asks for a garbage
        // shm height (ADR-310: 2337935) even after preferred_scale=120.
        surface.with_pending_state(|state| {
            let size = (geo.width, geo.height).into();
            state.size = Some(size);
            state.bounds = Some(size);
        });
        surface.send_configure();
        self.toplevels.insert(surface.wl_surface().clone(), surface);
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        self.toplevels.remove(surface.wl_surface());
        self.focus_history
            .retain(|candidate| candidate != surface.wl_surface());
        #[cfg(feature = "panther-hardware")]
        self.surface_frames.remove(surface.wl_surface());
        if self.focused_surface.as_ref() == Some(surface.wl_surface()) {
            self.activate_toplevel(self.focus_history.last().cloned());
        }
    }

    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {}

    fn grab(&mut self, _surface: PopupSurface, _seat: WlSeat, _serial: Serial) {}

    fn reposition_request(
        &mut self,
        _surface: PopupSurface,
        _positioner: PositionerState,
        _token: u32,
    ) {
    }
}
delegate_xdg_shell!(State);

impl OutputHandler for State {}
delegate_output!(State);

impl SessionLockHandler for State {
    fn lock_state(&mut self) -> &mut SessionLockManagerState {
        &mut self.session_lock_state
    }

    fn lock(&mut self, confirmation: SessionLocker) {
        // Every client on this compositor is trusted for now (ADR-015's
        // PID-based global filter is deferred to S04 Change step 7, once
        // saai-displayd actually forks saai-shell and knows its real
        // pid) -- confirm immediately, no separate "clear the screen
        // first" step since hardware::init()'s own black fill already
        // covers the normal startup case.
        println!("saai-displayd: session lock requested");
        self.locked = true;
        confirmation.lock();
    }

    fn unlock(&mut self) {
        println!("saai-displayd: session unlocked");
        self.locked = false;
        self.lock_surface = None;
        // Forces a full scene recomposite (ADR-016) -- without this the
        // panel would keep showing the lock surface's last frame until
        // some other surface happens to commit something new on its
        // own, which for a static placeholder toplevel could be never.
        #[cfg(feature = "panther-hardware")]
        self.request_recomposite();
    }

    fn new_surface(&mut self, surface: LockSurface, _output: WlOutput) {
        println!("saai-displayd: lock surface created");
        let (width, height) = (self.output_width, self.output_height);
        surface.with_pending_state(|state| {
            state.size = Some((width as u32, height as u32).into());
        });
        surface.send_configure();
        self.lock_surface = Some(surface);
    }
}
delegate_session_lock!(State);

impl WlrLayerShellHandler for State {
    fn shell_state(&mut self) -> &mut WlrLayerShellState {
        &mut self.layer_shell_state
    }

    fn new_layer_surface(
        &mut self,
        surface: LayerSurface,
        _output: Option<WlOutput>,
        _layer: smithay::wayland::shell::wlr_layer::Layer,
        namespace: String,
    ) {
        // ADR-271: honor the client's size. 0 on an axis means the
        // output size (status bar width, OSK width). Do not force 120px
        // top for every layer — APP-04's keyboard is a bottom strip.
        println!("saai-displayd: new layer surface, namespace={namespace:?}");
        let output_w = self.output_width;
        let output_h = self.output_height;
        let (requested_w, requested_h, _, _, _, _) = Self::layer_client_size_anchor(&surface, true);
        let (width, height) =
            layer_geom::configure_size(output_w, output_h, requested_w, requested_h);
        surface.with_pending_state(|state| {
            state.size = Some((width, height).into());
        });
        surface.send_configure();
        self.layer_surfaces.push(surface);
    }

    fn layer_destroyed(&mut self, surface: LayerSurface) {
        self.layer_surfaces.retain(|ls| ls != &surface);
        #[cfg(feature = "panther-hardware")]
        {
            self.surface_frames.remove(surface.wl_surface());
            self.request_recomposite();
        }
    }
}
delegate_layer_shell!(State);

fn main() {
    // Rc<RefCell<>>, not a plain owned value moved into one closure: the
    // display needs to be reachable from every event source that can
    // queue outgoing protocol messages (the socket/client-readable
    // source, but also touch input below, and potentially more later),
    // plus the end-of-iteration flush that actually guarantees delivery
    // -- see the flush_clients() comment near the bottom of main() for
    // why that end-of-iteration call is the one that actually matters.
    let display: Rc<RefCell<Display<State>>> = Rc::new(RefCell::new(
        Display::new().expect("failed to create display"),
    ));
    let dh: DisplayHandle = display.borrow().handle();

    #[cfg(feature = "panther-hardware")]
    let privileged_shell_pid = Arc::new(AtomicU32::new(0));

    let compositor_state = CompositorState::new::<State>(&dh);
    let shm_state = ShmState::new::<State>(&dh, Vec::new());
    let mut dmabuf_state = DmabufState::new();
    let dmabuf_global = dmabuf_state.create_global::<State>(
        &dh,
        [
            Format {
                code: Fourcc::Xrgb8888,
                modifier: Modifier::Linear,
            },
            Format {
                code: Fourcc::Argb8888,
                modifier: Modifier::Linear,
            },
        ],
    );
    let xdg_shell_state = XdgShellState::new::<State>(&dh);
    let data_device_manager = data_device::SaaiDataDeviceManager::new::<State>(&dh);
    let text_input_manager_state = text_ime::SaaiTextInputManager::new::<State>(&dh);
    let input_method_manager_state = text_ime::SaaiInputMethodManager::new::<State>(&dh);
    let fractional_scale_manager_state = FractionalScaleManagerState::new::<State>(&dh);
    let viewporter_state = ViewporterState::new::<State>(&dh);
    let mut seat_state = SeatState::<State>::new();
    let mut seat = seat_state.new_wl_seat(&dh, "seat0");
    #[cfg(not(feature = "panther-hardware"))]
    let keyboard = seat
        .add_keyboard(XkbConfig::default(), 200, 25)
        .expect("failed to add keyboard capability");
    #[cfg(not(feature = "panther-hardware"))]
    let pointer = seat.add_pointer();
    #[cfg(feature = "panther-hardware")]
    let touch = seat.add_touch();

    let mut event_loop: EventLoop<'static, State> =
        EventLoop::try_new().expect("failed to create event loop");
    let handle = event_loop.handle();

    #[cfg(feature = "panther-hardware")]
    let hardware = match hardware::init() {
        Ok((hw, drm_notifier)) => {
            if let Err(err) = handle.insert_source(drm_notifier, |event, _, state: &mut State| {
                use smithay::backend::drm::DrmEvent;
                match event {
                    // The flip just submitted (either the initial
                    // modeset in hardware::init(), or the most recent
                    // recomposite()) is now confirmed on screen -- safe
                    // to submit the next one. ADR-016: this is the
                    // handler that used to be `|_event, _, _state| {}`,
                    // the root reason nothing tracked flip completion
                    // at all before this.
                    DrmEvent::VBlank(_crtc) => {
                        state.flip_pending = false;

                        // ADR-024 continued: the flip just confirmed by
                        // this VBlank passed its fence as IN_FENCE_FD --
                        // the kernel would not have completed it
                        // otherwise. Any `Dmabuf`-backed frame deferred
                        // into `pending_frame_trash` while that flip was
                        // pending (`stash_or_drop_old_frame`) is now
                        // provably safe to drop (releasing its
                        // `wl_buffer` back to the client).
                        state.pending_frame_trash.clear();

                        // A frame callback means the frame reached scanout,
                        // not merely that an atomic commit was submitted.
                        // The Smithay helper also handles subsurfaces.
                        let output = state._wl_output.clone();
                        let time = state.presentation_started.elapsed();
                        for surface in std::mem::take(&mut state.pending_frame_surfaces) {
                            send_frames_surface_tree(&surface, &output, time, None, |_, _| {
                                Some(output.clone())
                            });
                        }
                        if state.repaint_needed {
                            state.repaint_needed = false;
                            state.recomposite();
                        }
                    }
                    DrmEvent::Error(err) => {
                        eprintln!("saai-displayd: DRM event error: {err}");
                        // Assume the in-flight flip (if any) is dead
                        // rather than staying stuck with flip_pending
                        // permanently true and never presenting again.
                        state.flip_pending = false;
                    }
                }
            }) {
                eprintln!("saai-displayd: failed to register DRM notifier: {err}");
            }
            println!("saai-displayd: hardware output initialized");
            Some(hw)
        }
        Err(err) => {
            eprintln!("saai-displayd: hardware output unavailable: {err}");
            None
        }
    };

    #[cfg(feature = "panther-hardware")]
    let hardware_ok = hardware.is_some();
    #[cfg(feature = "panther-hardware")]
    let (output_width, output_height) = hardware
        .as_ref()
        .map(|hw| (hw.width as i32, hw.height as i32))
        .unwrap_or((1080, 2400));
    #[cfg(not(feature = "panther-hardware"))]
    let (output_width, output_height) = (1920, 1080);

    let wl_output = Output::new(
        "panel-0".to_string(),
        PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "SaaiOS".into(),
            model: output_model_for(cfg!(feature = "panther-hardware")).into(),
        },
    );
    wl_output.create_global::<State>(&dh);
    wl_output.change_current_state(
        Some(OutputMode {
            size: (output_width, output_height).into(),
            refresh: 60000,
        }),
        Some(Transform::Normal),
        Some(Scale::Integer(1)),
        Some((0, 0).into()),
    );

    /// S32: bumps `/run/saaios/last-input`'s mtime -- `saai-shell`
    /// polls it (own literal copy of the path, ADR-030's usual
    /// no-cross-runtime-dependency convention) to tell "nothing
    /// happening anywhere" apart from "nothing happening on MY
    /// surfaces specifically", since this compositor is the one
    /// process that sees every touch regardless of which client's
    /// surface it's routed to. Fire-and-forget: a failure here (e.g.
    /// `/run/saaios` not mounted yet during very early boot) must
    /// never take down touch routing itself.
    #[cfg(feature = "panther-hardware")]
    fn mark_global_input_activity() {
        let _ = std::fs::File::create("/run/saaios/last-input");
    }

    #[cfg(feature = "panther-hardware")]
    match touch::open() {
        Ok(touch_file) => {
            let mut touch_state = touch::TouchState::new();
            let source = Generic::new(touch_file, Interest::READ, Mode::Level);
            if let Err(err) = handle.insert_source(source, move |_, file, state: &mut State| {
                use smithay::input::touch::{DownEvent, MotionEvent, UpEvent};
                use smithay::utils::Point;
                use std::io::Read;

                let mut buf = [0u8; touch::RAW_EVENT_SIZE];
                let touch = state.touch.clone();
                // `file`'s metadata type only derefs to `&File` (no
                // `DerefMut`), but `std::fs::File` also implements `Read`
                // for a shared reference (it's just an fd, no internal
                // buffering state that would need exclusive access).
                let mut f: &std::fs::File = file;
                loop {
                    match f.read(&mut buf) {
                        Ok(n) if n == buf.len() => {
                            let event = touch::parse(buf);
                            let Some(update) = touch_state.feed(&event) else {
                                continue;
                            };
                            let serial = SERIAL_COUNTER.next_serial();
                            let time = 0;
                            // ADR-015: while locked, touch goes to the lock
                            // surface. Unlocked, ADR-271: topmost layer under
                            // the contact, else focused_surface.
                            match update {
                                touch::TouchUpdate::Down { x, y } => {
                                    let focus = state.touch_focus_at(x as f64, y as f64);
                                    println!(
                                        "saai-displayd: touch down at ({x}, {y}), routed to: {:?} (locked={})",
                                        focus.as_ref().map(|(s, _)| s.id()),
                                        state.locked
                                    );
                                    mark_global_input_activity();
                                    let location = Point::from((x as f64, y as f64));
                                    touch.down(
                                        state,
                                        focus,
                                        &DownEvent {
                                            slot: (None::<u32>).into(),
                                            location,
                                            serial,
                                            time,
                                        },
                                    );
                                    touch.frame(state);
                                }
                                touch::TouchUpdate::Motion { x, y } => {
                                    let focus = state.touch_focus_at(x as f64, y as f64);
                                    let location = Point::from((x as f64, y as f64));
                                    touch.motion(
                                        state,
                                        focus,
                                        &MotionEvent {
                                            slot: (None::<u32>).into(),
                                            location,
                                            time,
                                        },
                                    );
                                    touch.frame(state);
                                }
                                touch::TouchUpdate::Up => {
                                    println!("saai-displayd: touch up");
                                    mark_global_input_activity();
                                    touch.up(
                                        state,
                                        &UpEvent {
                                            slot: (None::<u32>).into(),
                                            serial,
                                            time,
                                        },
                                    );
                                    touch.frame(state);
                                }
                            }
                        }
                        _ => break,
                    }
                }
                Ok(PostAction::Continue)
            }) {
                eprintln!("saai-displayd: failed to register touch input source: {err}");
            } else {
                println!("saai-displayd: touch input initialized");
            }
        }
        Err(err) => eprintln!("saai-displayd: touch input unavailable: {err}"),
    }

    let mut state = State {
        compositor_state,
        shm_state,
        dmabuf_state,
        _dmabuf_global: dmabuf_global,
        xdg_shell_state,
        seat_state,
        seat,
        _data_device_manager: data_device_manager,
        _text_input_manager_state: text_input_manager_state,
        _input_method_manager_state: input_method_manager_state,
        _fractional_scale_manager_state: fractional_scale_manager_state,
        _viewporter_state: viewporter_state,
        #[cfg(not(feature = "panther-hardware"))]
        keyboard: keyboard.clone(),
        #[cfg(not(feature = "panther-hardware"))]
        pointer: pointer.clone(),
        focused_surface: None,
        focus_history: Vec::new(),
        toplevels: HashMap::new(),
        #[cfg(feature = "panther-hardware")]
        touch,
        #[cfg(feature = "panther-hardware")]
        hardware,
        #[cfg(feature = "panther-hardware")]
        surface_frames: HashMap::new(),
        #[cfg(feature = "panther-hardware")]
        next_frame_generation: 0,
        #[cfg(feature = "panther-hardware")]
        slot_generations: [HashMap::new(), HashMap::new()],
        // hardware::init() already submitted one flip (the initial
        // modeset) before this State even existed, if it succeeded --
        // start "pending" to match, so nothing calls recomposite()
        // again before that first flip's VBlank event is processed.
        #[cfg(feature = "panther-hardware")]
        flip_pending: hardware_ok,
        #[cfg(feature = "panther-hardware")]
        repaint_needed: false,
        #[cfg(feature = "panther-hardware")]
        pending_frame_surfaces: Vec::new(),
        #[cfg(feature = "panther-hardware")]
        pending_frame_trash: Vec::new(),
        #[cfg(feature = "panther-hardware")]
        shell_child: None,
        #[cfg(feature = "panther-hardware")]
        shell_socket_name: String::new(),
        #[cfg(feature = "panther-hardware")]
        shell_restart_budget: RestartBudget::default(),
        #[cfg(feature = "panther-hardware")]
        privileged_shell_pid: privileged_shell_pid.clone(),
        #[cfg(feature = "panther-hardware")]
        presentation_started: Instant::now(),
        _wl_output: wl_output,
        output_width,
        output_height,
        #[cfg(feature = "panther-hardware")]
        session_lock_state: {
            let filter_pid = privileged_shell_pid.clone();
            SessionLockManagerState::new::<State, _>(&dh, move |client| {
                is_privileged_shell(client, &filter_pid)
            })
        },
        #[cfg(not(feature = "panther-hardware"))]
        session_lock_state: SessionLockManagerState::new::<State, _>(&dh, |_| true),
        locked: false,
        lock_surface: None,
        #[cfg(feature = "panther-hardware")]
        layer_shell_state: {
            let filter_pid = privileged_shell_pid.clone();
            WlrLayerShellState::new_with_filter::<State, _>(&dh, move |client| {
                is_privileged_shell(client, &filter_pid)
            })
        },
        #[cfg(not(feature = "panther-hardware"))]
        layer_shell_state: WlrLayerShellState::new_with_filter::<State, _>(&dh, |_| true),
        layer_surfaces: Vec::new(),
    };

    let socket = ListeningSocketSource::new_auto().expect("failed to create listening socket");
    let socket_name = socket.socket_name().to_string_lossy().into_owned();

    let mut dh_for_socket = dh.clone();
    handle
        .insert_source(socket, move |client_stream, _, _state| {
            #[cfg(feature = "panther-hardware")]
            let peer_pid = match getsockopt(&client_stream, PeerCredentials) {
                Ok(credentials) => credentials.pid() as u32,
                Err(err) => {
                    eprintln!("saai-displayd: failed to read client credentials: {err}");
                    0
                }
            };
            let client_data = SaaiClientState {
                compositor_state: CompositorClientState::default(),
                #[cfg(feature = "panther-hardware")]
                peer_pid,
            };
            match dh_for_socket.insert_client(client_stream, Arc::new(client_data)) {
                Ok(_) => {
                    #[cfg(feature = "panther-hardware")]
                    println!("saai-displayd: client connected pid={peer_pid}");
                    #[cfg(not(feature = "panther-hardware"))]
                    println!("saai-displayd: client connected");
                }
                Err(err) => eprintln!("saai-displayd: failed to insert client: {err}"),
            }
        })
        .expect("failed to insert socket source");

    #[cfg(feature = "panther-hardware")]
    {
        handle
            .insert_source(
                Signals::new(&[Signal::SIGCHLD]).expect("failed to listen for SIGCHLD"),
                |event, _, state: &mut State| {
                    if event.signal() == Signal::SIGCHLD && !state.handle_shell_sigchld() {
                        eprintln!(
                            "saai-displayd: saai-shell restart budget exhausted; exiting for PID 1 fallback"
                        );
                        std::process::exit(SHELL_FAILURE_EXIT_CODE);
                    }
                },
            )
            .expect("failed to register SIGCHLD source");
        state.shell_socket_name = socket_name.clone();
        if !state.launch_shell() {
            eprintln!(
                "saai-displayd: saai-shell restart budget exhausted during launch; exiting for PID 1 fallback"
            );
            std::process::exit(SHELL_FAILURE_EXIT_CODE);
        }
    }

    let display_fd = display
        .borrow_mut()
        .backend()
        .poll_fd()
        .try_clone_to_owned()
        .expect("failed to clone display fd");
    let display_for_fd = display.clone();
    handle
        .insert_source(
            Generic::new(display_fd, Interest::READ, Mode::Level),
            move |_, _, state: &mut State| {
                display_for_fd
                    .borrow_mut()
                    .dispatch_clients(state)
                    .expect("dispatch_clients failed");
                // No flush_clients() here anymore -- moved to the
                // per-iteration callback at the bottom of main() instead,
                // so it runs regardless of which source fired (this one
                // only runs when a *client* has sent something, making
                // this fd readable -- a protocol event queued from some
                // *other* source, like touch input below, would never
                // reach that code path at all). That move is a real,
                // independent correctness fix, but it did NOT resolve
                // the touch-delivery bug documented below -- see the
                // known-limitations entry in the S04 sprint doc: with
                // this same per-iteration flush active, WAYLAND_DEBUG=1
                // on the client still showed zero incoming wl_touch
                // messages, even though server-side instrumentation
                // (added and removed during diagnosis) confirmed
                // touch.down()/up() dispatch all the way down to the
                // wl_touch resource's own protocol-send call succeeding.
                // Root cause not yet found.
                Ok(PostAction::Continue)
            },
        )
        .expect("failed to insert display source");

    // Debug/test-only synthetic input trigger: `echo inject-key | saai-displayd`
    // sends a press+release of a fixed key to whichever surface currently
    // holds keyboard focus, so the S02 "synthetic input delivered only to
    // the focused client" acceptance test can be driven from a shell script
    // without real hardware. Not built for the panther-hardware target at
    // all -- no keyboard capability there to inject into (ADR-012).
    #[cfg(not(feature = "panther-hardware"))]
    {
        let stdin_source = Generic::new(std::io::stdin(), Interest::READ, Mode::Level);
        match handle.insert_source(stdin_source, move |_, _, state: &mut State| {
            let mut line = String::new();
            if std::io::stdin().lock().read_line(&mut line).unwrap_or(0) == 0 {
                return Ok(PostAction::Remove);
            }
            if line.trim() == "inject-key" {
                if state.focused_surface.is_some() {
                    let time = 0;
                    // evdev KEY_A (30) + 8 = xkb keycode 38.
                    let keycode = Keycode::new(38);
                    keyboard.input::<(), _>(
                        state,
                        keycode,
                        smithay::backend::input::KeyState::Pressed,
                        SERIAL_COUNTER.next_serial(),
                        time,
                        |_, _, _| FilterResult::Forward,
                    );
                    keyboard.input::<(), _>(
                        state,
                        keycode,
                        smithay::backend::input::KeyState::Released,
                        SERIAL_COUNTER.next_serial(),
                        time,
                        |_, _, _| FilterResult::Forward,
                    );
                    println!("saai-displayd: injected synthetic key press+release");
                } else {
                    println!("saai-displayd: inject-key requested but no surface is focused yet");
                }
            }
            Ok(PostAction::Continue)
        }) {
            Ok(_) => {}
            Err(err) => {
                // Best-effort debug convenience only -- stdin is not always
                // pollable depending on how this process was launched (backgrounded
                // with an inherited fd, a plain file, etc). Losing it must never
                // take the compositor down.
                eprintln!(
                    "saai-displayd: inject-key debug trigger unavailable, continuing without it: {err}"
                );
            }
        }
    }

    #[cfg(not(feature = "panther-hardware"))]
    {
        let _ = &state.pointer;
        let profile = hid::seat_input_profile(false);
        println!(
            "saai-displayd: x86 seat pointer={} keyboard={} touch={}",
            profile.pointer, profile.keyboard, profile.touch
        );
        if let Ok(text) = std::fs::read_to_string("/proc/bus/input/devices") {
            for device in hid::usb_hid_keyboards(&text) {
                println!(
                    "saai-displayd: usb hid keyboard {} ({})",
                    device.name, device.event_node
                );
            }
            for device in hid::usb_hid_pointers(&text) {
                println!(
                    "saai-displayd: usb hid pointer {} ({})",
                    device.name, device.event_node
                );
            }
        }
    }

    println!("saai-displayd: listening on WAYLAND_DISPLAY={socket_name}");
    event_loop
        .run(None, &mut state, move |_| {
            // Runs after *every* event loop iteration regardless of
            // which source fired -- a real fix for the general "only the
            // client-readable source used to flush" gap (see the comment
            // above at display_for_fd's closure). Necessary, but proven
            // NOT sufficient on its own for the touch-delivery bug --
            // see the known-limitations entry in the S04 sprint doc.
            display
                .borrow_mut()
                .flush_clients()
                .expect("flush_clients failed");
        })
        .expect("event loop failed");
}

#[cfg(test)]
mod supervision_tests {
    use super::{RestartBudget, SHELL_RESTART_LIMIT, SHELL_RESTART_WINDOW};
    use std::time::{Duration, Instant};

    #[test]
    fn third_shell_failure_inside_window_exhausts_budget() {
        let mut budget = RestartBudget::default();
        let start = Instant::now();
        assert_eq!(budget.record_failure(start), 1);
        assert_eq!(budget.record_failure(start + Duration::from_secs(1)), 2);
        assert_eq!(
            budget.record_failure(start + Duration::from_secs(2)),
            SHELL_RESTART_LIMIT
        );
    }

    #[test]
    fn failures_older_than_window_do_not_count() {
        let mut budget = RestartBudget::default();
        let start = Instant::now();
        assert_eq!(budget.record_failure(start), 1);
        assert_eq!(budget.record_failure(start + SHELL_RESTART_WINDOW), 1);
    }
}

#[cfg(test)]
mod windowed_surface_tests {
    use super::{
        output_model_for, toplevel_geometry_for, ToplevelGeometry, WINDOWED_HEIGHT, WINDOWED_WIDTH,
    };

    #[test]
    fn x86_toplevel_is_windowed_not_fullscreen() {
        let geo = toplevel_geometry_for(false, 1920, 1080);
        assert_eq!(
            geo,
            ToplevelGeometry {
                width: WINDOWED_WIDTH,
                height: WINDOWED_HEIGHT,
                fullscreen: false,
            }
        );
        assert!(geo.width < 1920);
        assert!(geo.height < 1080);
        assert_eq!(output_model_for(false), "x86");
        assert_ne!(output_model_for(false), "panther");
    }

    #[test]
    fn panther_toplevel_stays_panel_fullscreen() {
        let geo = toplevel_geometry_for(true, 1080, 2400);
        assert_eq!(
            geo,
            ToplevelGeometry {
                width: 1080,
                height: 2400,
                fullscreen: true,
            }
        );
        assert_eq!(output_model_for(true), "panther");
    }

    #[test]
    fn bounds_are_the_toplevel_geometry_never_zero() {
        for (phone, out_w, out_h) in [(false, 1920, 1080), (true, 1080, 2400)] {
            let geo = toplevel_geometry_for(phone, out_w, out_h);
            assert!(geo.width > 0);
            assert!(geo.height > 0);
            assert_ne!((geo.width, geo.height), (0, 0));
        }
    }
}
