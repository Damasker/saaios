use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
#[cfg(not(feature = "panther-hardware"))]
use std::io::BufRead;
#[cfg(feature = "panther-hardware")]
use std::process::{Child, Command};
use std::rc::Rc;
#[cfg(feature = "panther-hardware")]
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(feature = "panther-hardware")]
use calloop::signals::{Signal, Signals};

#[cfg(feature = "panther-hardware")]
mod hardware;
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
    delegate_compositor, delegate_layer_shell, delegate_output, delegate_seat,
    delegate_session_lock, delegate_shm, delegate_xdg_shell,
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
        output::OutputHandler,
        session_lock::{LockSurface, SessionLockHandler, SessionLockManagerState, SessionLocker},
        shell::{
            wlr_layer::{LayerSurface, WlrLayerShellHandler, WlrLayerShellState},
            xdg::{PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState},
        },
        shm::{with_buffer_contents, ShmHandler, ShmState},
        socket::ListeningSocketSource,
    },
};

#[derive(Default)]
struct SaaiClientState {
    compositor_state: CompositorClientState,
}
impl ClientData for SaaiClientState {}

/// One surface's last committed frame, cached for `State::recomposite()`
/// (ADR-016). Plain owned pixels, not a reference into the Wayland
/// buffer -- the buffer itself may be reused/destroyed by the client
/// well before this compositor needs to redraw the scene again (e.g.
/// on unlock, with no new commit from anyone involved).
#[cfg(feature = "panther-hardware")]
struct SurfaceFrame {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    stride: u32,
}

struct State {
    compositor_state: CompositorState,
    shm_state: ShmState,
    xdg_shell_state: XdgShellState,
    seat_state: SeatState<State>,
    _seat: Seat<State>,
    // No keyboard capability on the real Pixel 7 build (ADR-012): this
    // device has no physical keyboard, and drm-splash.c's own on-screen
    // keyboard proves this architecture never needed wl_keyboard/xkbcommon
    // keysym translation to begin with. Kept for the headless S02 build,
    // where the keyboard-focus/synthetic-inject acceptance tests still use
    // a real KeyboardHandle and a normal host libxkbcommon works fine.
    #[cfg(not(feature = "panther-hardware"))]
    keyboard: smithay::input::keyboard::KeyboardHandle<State>,
    /// First surface to commit a real (non-null) buffer keeps input focus
    /// for the lifetime of this headless compositor -- single-window focus
    /// policy, matching the eventual fullscreen panther shell.
    focused_surface: Option<WlSurface>,
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
    /// Known layer surfaces (status bar / nav overlay per ADR-015) --
    /// checked in commit()'s should_present branch so their frames reach
    /// the panel. No spatial hit-testing wired up yet: touch routing
    /// still only knows about `focused_surface`/`lock_surface`, so a
    /// layer surface renders but cannot receive touch input in this
    /// step -- known limitation, not a goal of this vertical slice.
    layer_surfaces: Vec<LayerSurface>,
}

#[cfg(feature = "panther-hardware")]
const SAAI_SHELL_PATH: &str = "/saaios/saai-shell";

const SHELL_RESTART_LIMIT: usize = 3;
const SHELL_RESTART_WINDOW: Duration = Duration::from_secs(60);
#[cfg(feature = "panther-hardware")]
const SHELL_FAILURE_EXIT_CODE: i32 = 71;

#[derive(Default)]
struct RestartBudget {
    failures: VecDeque<Instant>,
}

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
fn is_privileged_shell(client: &Client, dh: &DisplayHandle, shell_pid: &AtomicU32) -> bool {
    let expected = shell_pid.load(Ordering::Acquire);
    expected != 0
        && client
            .get_credentials(dh)
            .is_ok_and(|credentials| credentials.pid as u32 == expected)
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
    fn recomposite(&mut self) {
        let Some(hw) = self.hardware.as_mut() else {
            return;
        };
        hw.fill(0x00, 0x00, 0x00);
        let mut shown: Vec<WlSurface> = Vec::new();
        if self.locked {
            if let Some((s, frame)) = self.lock_surface.as_ref().and_then(|ls| {
                let s = ls.wl_surface().clone();
                self.surface_frames.get(&s).map(|f| (s, f))
            }) {
                hw.blit(&frame.pixels, frame.width, frame.height, frame.stride);
                shown.push(s);
            }
        } else {
            if let Some((s, frame)) = self.focused_surface.clone().and_then(|s| {
                let frame = self.surface_frames.get(&s)?;
                Some((s, frame))
            }) {
                hw.blit(&frame.pixels, frame.width, frame.height, frame.stride);
                shown.push(s);
            }
            for layer in &self.layer_surfaces {
                let s = layer.wl_surface().clone();
                if let Some(frame) = self.surface_frames.get(&s) {
                    hw.blit(&frame.pixels, frame.width, frame.height, frame.stride);
                    shown.push(s);
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

        let result = with_buffer_contents(&buffer, |ptr, len, data| {
            let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
            let mut hasher = Sha256::new();
            hasher.update(bytes);
            #[cfg(feature = "panther-hardware")]
            let pixels = bytes.to_vec();
            #[cfg(not(feature = "panther-hardware"))]
            let pixels = ();
            (
                hasher.finalize(),
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

        // Must run before the hardware-blit check below: on a surface's very
        // first buffer-carrying commit, focus is still None, so checking
        // focus first would skip blitting that frame -- the display would
        // only ever show the initial fill from hardware::init() and never
        // this (or any) client's actual content.
        if self.focused_surface.is_none() {
            self.focused_surface = Some(surface.clone());
            #[cfg(not(feature = "panther-hardware"))]
            {
                let serial = SERIAL_COUNTER.next_serial();
                let keyboard = self.keyboard.clone();
                keyboard.set_focus(self, Some(surface.clone()), serial);
                println!(
                    "saai-displayd: keyboard focus set to surface {:?}",
                    surface.id()
                );
            }
            #[cfg(feature = "panther-hardware")]
            println!("saai-displayd: focus set to surface {:?}", surface.id());
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
                    self.surface_frames.insert(
                        surface.clone(),
                        SurfaceFrame {
                            pixels: _pixels,
                            width: _width,
                            height: _height,
                            stride: _stride,
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

impl SeatHandler for State {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<State> {
        &mut self.seat_state
    }
}
delegate_seat!(State);

impl XdgShellHandler for State {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        println!("saai-displayd: new xdg_toplevel");
        // Was hardcoded to 800x480 (an S02 headless-test leftover, from
        // before the real panther panel size was known) regardless of
        // what the client actually asked for -- real bug, confirmed on
        // hardware: saai-shell's own fullscreen request was silently
        // overridden by this every single time, so every visual test
        // this sprint ran against an 800x480 toplevel, not the real
        // 1080x2400 panel.
        let (width, height) = (self.output_width, self.output_height);
        surface.with_pending_state(|state| {
            state.size = Some((width, height).into());
        });
        surface.send_configure();
        self.toplevels.insert(surface.wl_surface().clone(), surface);
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        self.toplevels.remove(surface.wl_surface());
        #[cfg(feature = "panther-hardware")]
        self.surface_frames.remove(surface.wl_surface());
        if self.focused_surface.as_ref() == Some(surface.wl_surface()) {
            self.focused_surface = None;
            #[cfg(feature = "panther-hardware")]
            self.request_recomposite();
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
        // Minimal vertical slice (S04 Change 4, second half of ADR-015):
        // prove the protocol renders end to end, same standard as the
        // session-lock slice above. No real status-bar content yet and
        // no per-client anchor/size negotiation -- every layer surface
        // gets a fixed top strip, panel width x a fixed height, exactly
        // like `new_surface` above always overrides with the real panel
        // size rather than trusting client hints.
        println!("saai-displayd: new layer surface, namespace={namespace:?}");
        let width = self.output_width;
        let height = 120;
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
    let xdg_shell_state = XdgShellState::new::<State>(&dh);
    let mut seat_state = SeatState::<State>::new();
    let mut seat = seat_state.new_wl_seat(&dh, "seat0");
    #[cfg(not(feature = "panther-hardware"))]
    let keyboard = seat
        .add_keyboard(XkbConfig::default(), 200, 25)
        .expect("failed to add keyboard capability");
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
            model: "panther".into(),
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
                            // Computed once as an owned value (not a
                            // closure over `state`): `touch.down(state, ...)`
                            // needs `state` by mutable reference, which
                            // would conflict with a closure still borrowing
                            // it for this same call's other argument.
                            //
                            // ADR-015's security invariant enforced here, not
                            // just in the session_lock protocol handlers:
                            // while locked, touch goes to the lock surface
                            // (or nowhere, if the client hasn't created one
                            // yet) and never falls through to
                            // `focused_surface` -- a locked screen must not
                            // pass input to the app underneath.
                            let focus = if state.locked {
                                state
                                    .lock_surface
                                    .as_ref()
                                    .map(|ls| (ls.wl_surface().clone(), Point::from((0.0, 0.0))))
                            } else {
                                state
                                    .focused_surface
                                    .clone()
                                    .map(|s| (s, Point::from((0.0, 0.0))))
                            };
                            match update {
                                touch::TouchUpdate::Down { x, y } => {
                                    println!(
                                        "saai-displayd: touch down at ({x}, {y}), routed to: {:?} (locked={})",
                                        focus.as_ref().map(|(s, _)| s.id()),
                                        state.locked
                                    );
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
        xdg_shell_state,
        seat_state,
        _seat: seat,
        #[cfg(not(feature = "panther-hardware"))]
        keyboard: keyboard.clone(),
        focused_surface: None,
        toplevels: HashMap::new(),
        #[cfg(feature = "panther-hardware")]
        touch,
        #[cfg(feature = "panther-hardware")]
        hardware,
        #[cfg(feature = "panther-hardware")]
        surface_frames: HashMap::new(),
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
            let filter_dh = dh.clone();
            SessionLockManagerState::new::<State, _>(&dh, move |client| {
                is_privileged_shell(client, &filter_dh, &filter_pid)
            })
        },
        #[cfg(not(feature = "panther-hardware"))]
        session_lock_state: SessionLockManagerState::new::<State, _>(&dh, |_| true),
        locked: false,
        lock_surface: None,
        #[cfg(feature = "panther-hardware")]
        layer_shell_state: {
            let filter_pid = privileged_shell_pid.clone();
            let filter_dh = dh.clone();
            WlrLayerShellState::new_with_filter::<State, _>(&dh, move |client| {
                is_privileged_shell(client, &filter_dh, &filter_pid)
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
        .insert_source(socket, move |client_stream, _, _state| match dh_for_socket
            .insert_client(client_stream, Arc::new(SaaiClientState::default()))
        {
            Ok(_) => println!("saai-displayd: client connected"),
            Err(err) => eprintln!("saai-displayd: failed to insert client: {err}"),
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
