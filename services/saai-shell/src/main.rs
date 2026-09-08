//! `saai-shell`: the real system Wayland client for SaaiOS (ADR-005),
//! replacing drm-splash.c's hand-rolled UI. Run manually for now
//! (`WAYLAND_DISPLAY=... saai-shell`); ADR-014's auto-launch-as-
//! saai-displayd's-child wiring is Change step 7.
//!
//! S04 Change step 3 built the minimal vertical slice (normal
//! `xdg_shell` toplevel, no new protocols). Change step 4 adds
//! `ext-session-lock-v1` (ADR-015): right after the normal window comes
//! up, this test also requests a session lock, creates a lock surface
//! per output, and renders it in a visibly different color -- the point
//! is to physically confirm the *security* side of ADR-015 works, not
//! just the protocol plumbing: touch should reach only the lock surface
//! while locked, never the toplevel underneath (enforced in
//! saai-displayd's touch routing, not by this client).
//!
//! `wlr-layer-shell` (the other half of ADR-015, for status bar/
//! navigation-style system surfaces) rounds out Change step 4: this
//! test also creates a single top-anchored layer surface (namespace
//! "saai-shell-statusbar-test") and fills it a solid, empirically
//! distinct color -- proving the protocol renders end to end. No real
//! status bar content, and no touch dispatch to it yet (saai-displayd's
//! touch routing still only knows `focused_surface`/`lock_surface`,
//! not layer surfaces) -- both are follow-up work, not this step's
//! goal.
//!
//! Change step 5 ports drm-splash.c's lock/idle behavior: boots locked
//! (matching `bool locked = true` at the top of drm-splash's own main
//! loop -- this was never a Change-4-only test hack, it's the real
//! boot-to-lock-screen behavior a phone is expected to have), unlocks
//! on a touch-down-then-release over the lock surface, and re-locks
//! after 60s of no touch activity while unlocked. Two things
//! drm-splash.c also does are deliberately NOT ported here:
//! - **Display power off.** drm-splash calls `disable_display()`
//!   (blanks the CRTC) on the same idle timeout. That's a DRM
//!   operation only saai-displayd can perform (ADR-005/010: this
//!   client gets no DRM access), and no protocol/IPC to ask for it
//!   exists yet -- needs its own design (`ext-idle-notify-v1` +
//!   `wlr-output-power-management-v1`, or a private mechanism), not
//!   assumed here. The panel simply stays on and shows the lock
//!   surface indefinitely instead of the phone-realistic
//!   dim-then-blank sequence.
//! - **Haptic feedback on unlock.** drm-splash opens `/dev/input/haptic`
//!   directly. This client has no raw evdev access either (same
//!   ADR-005/010 boundary) and there's no existing path to ask
//!   saai-displayd to play a haptic effect on this client's behalf.
//!
//! Both are logged as known limitations in the S04 sprint doc, not
//! silently dropped.
//!
//! Change step 6 ports drm-splash.c's four root sections (`root_page()`,
//! `render_root_controls()`): "Сейчас"/"Входящие"/"Пространства"/"Я",
//! navigable via a bottom tab bar. No real per-section content yet
//! (placeholder-only is explicitly in scope for this step, per the S04
//! sprint doc) and no text rendering exists in this client at all
//! (drm-splash.c has its own bitmap font; porting that is out of scope
//! here) -- each section is a distinct solid color instead, same
//! "color as the physically-verifiable signal" approach already used
//! for the lock surface and layer-shell bar. The tab bar lives inside
//! the toplevel's own buffer, not a separate layer surface: touch
//! routing only knows `focused_surface`/`lock_surface` (a known
//! limitation from Change 4), so a real layer surface couldn't
//! receive the taps that switch pages.

use std::time::{Duration, Instant};

use smithay_client_toolkit::reexports::client::{
    globals::registry_queue_init,
    protocol::{wl_output, wl_seat, wl_shm, wl_surface, wl_touch},
    Connection, QueueHandle,
};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_seat,
    delegate_session_lock, delegate_shm, delegate_touch, delegate_xdg_shell, delegate_xdg_window,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{touch::TouchHandler, Capability, SeatHandler, SeatState},
    session_lock::{
        SessionLock, SessionLockHandler, SessionLockState, SessionLockSurface,
        SessionLockSurfaceConfigure,
    },
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
        xdg::{
            window::{Window, WindowConfigure, WindowDecorations, WindowHandler},
            XdgShell,
        },
        WaylandSurface,
    },
    shm::{
        slot::{Buffer, SlotPool},
        Shm, ShmHandler,
    },
};

/// Matches drm-splash.c's own idle-to-lock constant.
const IDLE_TIMEOUT: Duration = Duration::from_secs(60);

/// The four root sections (drm-splash.c's `root_page()`/`root_pages`),
/// in bottom-tab-bar order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RootPage {
    Now,
    Inbox,
    Spaces,
    Me,
}

impl RootPage {
    const ALL: [RootPage; 4] = [
        RootPage::Now,
        RootPage::Inbox,
        RootPage::Spaces,
        RootPage::Me,
    ];

    /// Full-brightness fill for this page's content area, and for its
    /// own tab-bar segment when it's the active one. Each combines only
    /// the R and G byte positions the lock-surface diagnostic actually
    /// confirmed (see SessionLockHandler::configure's comment) --
    /// byte-index 0's real channel was never characterized, so it's
    /// left at 0 everywhere rather than guessed at.
    fn color(self) -> [u8; 4] {
        // Byte-index 1 = R, byte-index 2 = G (empirically confirmed by
        // the lock-surface diagnostic, SessionLockHandler::configure's
        // comment) -- an earlier version of this function had these
        // two positions swapped, which physically showed up as green
        // instead of red for `Now` (confirmed on hardware: the
        // constants below are the corrected version).
        match self {
            RootPage::Now => [0x00, 0xd0, 0x00, 0x00],
            RootPage::Inbox => [0x00, 0x00, 0xd0, 0x00],
            RootPage::Spaces => [0x00, 0x60, 0xd0, 0x00],
            RootPage::Me => [0x00, 0xd0, 0x60, 0x00],
        }
    }

    /// Dimmed version of `color()`, for this page's tab-bar segment
    /// when it's *not* the active page -- the only signal this client
    /// has for "which tab is selected" without any text rendering.
    fn dim_color(self) -> [u8; 4] {
        let [b, g, r, x] = self.color();
        [b / 3, g / 3, r / 3, x]
    }
}

/// Design-space layout of the bottom tab bar, matching drm-splash.c's
/// own `root_tab_at()` exactly (four 270px-wide segments starting at
/// y=2100, on the same 1080x2400 design canvas drm-splash.c always
/// assumed) -- scaled proportionally against whatever size this
/// surface is actually configured at, same as drm-splash.c scaled
/// against its own `create.width`/`create.height`.
fn tab_at(pos: (f64, f64), width: u32, height: u32) -> Option<RootPage> {
    if width == 0 || height == 0 {
        return None;
    }
    let design_x = (pos.0 * 1080.0 / width as f64) as i32;
    let design_y = (pos.1 * 2400.0 / height as f64) as i32;
    if !(2100..2400).contains(&design_y) || !(0..1080).contains(&design_x) {
        return None;
    }
    Some(RootPage::ALL[(design_x / 270).clamp(0, 3) as usize])
}

fn main() {
    let conn = Connection::connect_to_env().expect("failed to connect to Wayland display");
    let (globals, event_queue) = registry_queue_init(&conn).expect("failed to init registry");
    let qh = event_queue.handle();

    let mut event_loop: smithay_client_toolkit::reexports::calloop::EventLoop<'static, Shell> =
        smithay_client_toolkit::reexports::calloop::EventLoop::try_new()
            .expect("failed to create event loop");
    smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource::new(
        conn.clone(),
        event_queue,
    )
    .insert(event_loop.handle())
    .expect("failed to insert Wayland source");

    let compositor = CompositorState::bind(&globals, &qh).expect("wl_compositor not available");
    let xdg_shell = XdgShell::bind(&globals, &qh).expect("xdg_wm_base not available");
    let shm = Shm::bind(&globals, &qh).expect("wl_shm not available");
    let session_lock_state = SessionLockState::new(&globals, &qh);
    let layer_shell = LayerShell::bind(&globals, &qh).expect("wlr-layer-shell not available");

    let surface = compositor.create_surface(&qh);
    let window = xdg_shell.create_window(surface, WindowDecorations::ServerDefault, &qh);
    window.set_title("SaaiOS");
    window.set_app_id("org.saaios.shell");
    // Fullscreen, not a resizable desktop window -- saai-shell is the
    // system shell, not an app; matches drm-splash.c's own fixed
    // 1080x2400 panel assumption for now (real multi-output handling is
    // future work, not this vertical slice).
    //
    // set_min_size() alone does NOT request fullscreen -- it only
    // constrains resizing, so the server was free to configure whatever
    // size it wanted (observed on hardware: 800x480, the same default
    // saai-displayd hands out when it has no better information). This
    // was a real, previously-unnoticed bug: every visual test this
    // sprint ran against an 800x480 toplevel, not the real panel.
    window.set_min_size(Some((1080, 2400)));
    window.set_fullscreen(None);
    window.commit();

    // Second half of ADR-015 (Change 4): a real system-surface layer,
    // for the status bar/nav-style content the four root sections will
    // eventually need (Change 6) -- this test slice just proves the
    // protocol renders, no real content yet.
    let bar_surface = compositor.create_surface(&qh);
    let layer = layer_shell.create_layer_surface(
        &qh,
        bar_surface,
        Layer::Top,
        Some("saai-shell-statusbar-test"),
        None,
    );
    layer.set_anchor(Anchor::TOP | Anchor::LEFT | Anchor::RIGHT);
    layer.set_size(0, 120);
    layer.set_keyboard_interactivity(KeyboardInteractivity::None);
    // Initial commit with no attached buffer -- required by the
    // protocol before the compositor will send the first configure
    // (mirrors the toolkit's own simple_layer.rs example).
    layer.commit();

    let pool = SlotPool::new(1080 * 2400 * 4, &shm).expect("failed to create SHM pool");

    let mut shell = Shell {
        registry_state: RegistryState::new(&globals),
        output_state: OutputState::new(&globals, &qh),
        seat_state: SeatState::new(&globals, &qh),
        touch: None,
        compositor,
        shm,
        exit: false,
        first_configure: true,
        pool,
        width: 1080,
        height: 2400,
        buffer: None,
        window,
        session_lock_state,
        session_lock: None,
        lock_surfaces: Vec::new(),
        lock_pool: None,
        lock_buffer: None,
        locked: true,
        unlock_pending: false,
        last_activity: Instant::now(),
        current_page: RootPage::Now,
        last_touch_pos: (0.0, 0.0),
        tab_touch_pending: false,
        layer,
        layer_width: 0,
        layer_height: 120,
        layer_pool: None,
        layer_buffer: None,
    };

    println!("saai-shell: connected, toplevel created");

    // Boots locked, matching drm-splash.c's own `bool locked = true` at
    // the top of its main loop -- a phone that boots straight to an
    // unlocked launcher would be a real regression, not a simplification.
    shell.session_lock = Some(
        shell
            .session_lock_state
            .lock(&qh)
            .expect("ext-session-lock-v1 not supported by saai-displayd"),
    );
    println!("saai-shell: session lock requested");

    while !shell.exit {
        event_loop
            .dispatch(Duration::from_millis(16), &mut shell)
            .expect("event loop dispatch failed");
        shell.check_idle_timeout(&qh);
    }
}

struct Shell {
    registry_state: RegistryState,
    output_state: OutputState,
    seat_state: SeatState,
    touch: Option<wl_touch::WlTouch>,
    compositor: CompositorState,
    shm: Shm,

    exit: bool,
    first_configure: bool,
    pool: SlotPool,
    width: u32,
    height: u32,
    buffer: Option<Buffer>,
    window: Window,

    session_lock_state: SessionLockState,
    session_lock: Option<SessionLock>,
    lock_surfaces: Vec<SessionLockSurface>,
    /// Kept alive for as long as the lock surface's buffer is attached --
    /// dropping the pool would unmap the shared memory the compositor
    /// still needs to read after `commit()` returns.
    lock_pool: Option<SlotPool>,
    lock_buffer: Option<Buffer>,
    /// Mirrors saai-displayd's own `locked` bool -- this client is the
    /// only one that ever calls lock()/unlock(), so tracking it here
    /// (rather than round-tripping through the server) is enough to
    /// drive the idle timer and the touch-to-unlock gesture.
    locked: bool,
    /// Set on a touch-down that started on the lock surface while
    /// locked; the matching touch-up is what actually unlocks (mirrors
    /// drm-splash.c requiring touch *release* over the lock screen, not
    /// just a touch-start, so a drag-through or accidental brush
    /// doesn't unlock).
    unlock_pending: bool,
    last_activity: Instant,
    /// Currently visible root section (Change step 6).
    current_page: RootPage,
    /// Last known touch position (from `down()`/`motion()`) -- `up()`
    /// doesn't carry a position itself, so this is what tells it where
    /// the touch actually ended for tab-bar hit-testing.
    last_touch_pos: (f64, f64),
    /// Set on a touch-down over the toplevel's own tab bar while
    /// unlocked; the matching touch-up is what actually switches pages
    /// (same "release, not press" rule as `unlock_pending`, so a drag
    /// through the tab bar doesn't switch pages by accident).
    tab_touch_pending: bool,

    layer: LayerSurface,
    layer_width: u32,
    layer_height: u32,
    layer_pool: Option<SlotPool>,
    layer_buffer: Option<Buffer>,
}

impl CompositorHandler for Shell {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        // The shell is event-driven. Presentation feedback must not
        // redraw an unchanged full-screen scene forever.
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for Shell {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl WindowHandler for Shell {
    fn request_close(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _window: &Window) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        _window: &Window,
        configure: WindowConfigure,
        _serial: u32,
    ) {
        self.buffer = None;
        self.width = configure.new_size.0.map(|v| v.get()).unwrap_or(1080);
        self.height = configure.new_size.1.map(|v| v.get()).unwrap_or(2400);

        if self.first_configure {
            self.first_configure = false;
            println!(
                "saai-shell: first configure at {}x{}, drawing placeholder",
                self.width, self.height
            );
            self.draw(conn, qh);
        }
    }
}

impl ShmHandler for Shell {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl SessionLockHandler for Shell {
    fn locked(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, session_lock: SessionLock) {
        println!("saai-shell: session locked, creating lock surface(s)");
        self.locked = true;
        for output in self.output_state.outputs() {
            let surface = self.compositor.create_surface(qh);
            let lock_surface = session_lock.create_lock_surface(surface, &output, qh);
            self.lock_surfaces.push(lock_surface);
        }
    }

    fn finished(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _session_lock: SessionLock,
    ) {
        // Compositor refused or dropped the lock (e.g. ext-session-lock-v1
        // not implemented). Not fatal for this test client -- keep
        // running as a plain toplevel.
        println!("saai-shell: session lock finished (refused or dropped)");
        self.session_lock = None;
        self.lock_surfaces.clear();
        self.locked = false;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        session_lock_surface: SessionLockSurface,
        configure: SessionLockSurfaceConfigure,
        _serial: u32,
    ) {
        let (width, height) = configure.new_size;
        let (width, height) = (width.max(1), height.max(1));
        println!("saai-shell: lock surface configure at {width}x{height}");

        let stride = width as i32 * 4;
        let pool = self.lock_pool.get_or_insert_with(|| {
            SlotPool::new(width as usize * height as usize * 4, &self.shm)
                .expect("create lock surface pool")
        });
        let (buffer, canvas) = pool
            .create_buffer(
                width as i32,
                height as i32,
                stride,
                wl_shm::Format::Xrgb8888,
            )
            .expect("create buffer");

        // Bright red -- deliberately unmistakable against the toplevel's
        // dark slate placeholder, so a photo of the panel makes it
        // obvious which surface is actually receiving the compositor's
        // output while locked.
        //
        // Byte order here is [0x00, 0xd0, 0x00, 0x00], *not* the
        // [B, G, R, X] a standard XRGB8888 LE layout would predict for
        // red. A three-band on-device diagnostic (one solid color per
        // byte position, read back directly from this pool's memfd via
        // /proc/<pid>/fd to confirm the client-side write itself before
        // ever trusting the photo) proved this panel's pipeline reads R
        // from byte-index 1 and G from byte-index 2 -- swapped from the
        // conventional B,G,R,X -- while byte-index 0 produced no visible
        // output at all in the same test (untested whether that's a true
        // "blue" that just read as too dark to name, or genuinely
        // unused; not re-verified here since only red is needed for this
        // milestone). Root cause on the DRM/driver side not identified --
        // no standard fourcc swaps R and G while leaving B in place, so
        // this is applied as an empirically-verified byte order, not a
        // fourcc fix.
        let pixel: [u8; 4] = [0x00, 0xd0, 0x00, 0x00];
        for chunk in canvas.chunks_exact_mut(4) {
            chunk.copy_from_slice(&pixel);
        }

        let surface = session_lock_surface.wl_surface();
        surface.damage_buffer(0, 0, width as i32, height as i32);
        buffer.attach_to(surface).expect("buffer attach");
        surface.commit();
        self.lock_buffer = Some(buffer);
    }
}

impl LayerShellHandler for Shell {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        // Not fatal for this test client, same stance as SessionLockHandler::finished --
        // keep running as a plain toplevel if the compositor takes the layer surface away.
        println!("saai-shell: layer surface closed");
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let (width, height) = configure.new_size;
        let (width, height) = (width.max(1), height.max(1));
        println!("saai-shell: layer surface configure at {width}x{height}");
        self.layer_width = width;
        self.layer_height = height;

        let stride = width as i32 * 4;
        let pool = self.layer_pool.get_or_insert_with(|| {
            SlotPool::new(width as usize * height as usize * 4, &self.shm)
                .expect("create layer surface pool")
        });
        let (buffer, canvas) = pool
            .create_buffer(
                width as i32,
                height as i32,
                stride,
                wl_shm::Format::Xrgb8888,
            )
            .expect("create buffer");

        // Yellow (R + G, both empirically confirmed by the lock-surface
        // diagnostic above) -- deliberately avoids byte-index 0, whose
        // real channel was never characterized (it produced no visible
        // output in that test; unclear if that's a true "blue" too dark
        // to read or genuinely unused). Distinct from both the
        // toplevel's dark slate and the lock surface's red.
        let pixel: [u8; 4] = [0x00, 0xd0, 0xd0, 0x00];
        for chunk in canvas.chunks_exact_mut(4) {
            chunk.copy_from_slice(&pixel);
        }

        let surface = self.layer.wl_surface();
        surface.damage_buffer(0, 0, width as i32, height as i32);
        buffer.attach_to(surface).expect("buffer attach");
        self.layer.commit();
        self.layer_buffer = Some(buffer);
    }
}

impl SeatHandler for Shell {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        // No pointer/keyboard handling -- this is a touchscreen-only
        // device (ADR-012 already made the same call for saai-displayd).
        if capability == Capability::Touch && self.touch.is_none() {
            match self.seat_state.get_touch(qh, &seat) {
                Ok(touch) => self.touch = Some(touch),
                Err(err) => eprintln!("saai-shell: failed to get wl_touch: {err}"),
            }
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Touch {
            if let Some(touch) = self.touch.take() {
                touch.release();
            }
        }
    }

    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {
    }
}

impl TouchHandler for Shell {
    fn down(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &wl_touch::WlTouch,
        _serial: u32,
        _time: u32,
        surface: wl_surface::WlSurface,
        _id: i32,
        position: (f64, f64),
    ) {
        self.last_activity = Instant::now();
        self.last_touch_pos = position;
        self.unlock_pending = self.locked
            && self
                .lock_surfaces
                .iter()
                .any(|ls| *ls.wl_surface() == surface);
        self.tab_touch_pending = !self.locked && surface == *self.window.wl_surface();
    }

    fn up(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        _touch: &wl_touch::WlTouch,
        _serial: u32,
        _time: u32,
        _id: i32,
    ) {
        self.last_activity = Instant::now();
        // Release, not just touch-start, is what unlocks -- matches
        // drm-splash.c's own `touch_released` gate, so a drag that
        // starts on the lock surface but ends elsewhere (or a
        // multi-touch gesture) doesn't unlock by accident.
        if self.unlock_pending {
            self.unlock_pending = false;
            if let Some(session_lock) = self.session_lock.take() {
                session_lock.unlock();
            }
            self.lock_surfaces.clear();
            self.locked = false;
            println!("saai-shell: unlocked by touch");
        } else if self.tab_touch_pending {
            self.tab_touch_pending = false;
            if let Some(page) = tab_at(self.last_touch_pos, self.width, self.height) {
                if page != self.current_page {
                    println!("saai-shell: switched to {page:?}");
                    self.current_page = page;
                    self.draw(conn, qh);
                }
            }
        }
    }

    fn motion(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &wl_touch::WlTouch,
        _time: u32,
        _id: i32,
        position: (f64, f64),
    ) {
        self.last_touch_pos = position;
    }

    fn shape(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &wl_touch::WlTouch,
        _id: i32,
        _major: f64,
        _minor: f64,
    ) {
    }

    fn orientation(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _touch: &wl_touch::WlTouch,
        _id: i32,
        _orientation: f64,
    ) {
    }

    fn cancel(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _touch: &wl_touch::WlTouch) {
        self.unlock_pending = false;
        self.tab_touch_pending = false;
    }
}

impl Shell {
    /// Renders the active root section's placeholder content plus the
    /// bottom tab bar (Change step 6) -- proves the real
    /// client<->compositor vertical slice end to end (surface
    /// creation, configure, SHM buffer, commit, frame callback), same
    /// as the single dark-slate fill this replaced, just with content
    /// that actually changes on navigation instead of a static color.
    fn draw(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>) {
        let width = self.width;
        let height = self.height;
        let stride = width as i32 * 4;

        let buffer = self.buffer.get_or_insert_with(|| {
            self.pool
                .create_buffer(
                    width as i32,
                    height as i32,
                    stride,
                    wl_shm::Format::Xrgb8888,
                )
                .expect("create buffer")
                .0
        });

        let canvas = match self.pool.canvas(buffer) {
            Some(canvas) => canvas,
            None => {
                let (second_buffer, canvas) = self
                    .pool
                    .create_buffer(
                        width as i32,
                        height as i32,
                        stride,
                        wl_shm::Format::Xrgb8888,
                    )
                    .expect("create buffer");
                *buffer = second_buffer;
                canvas
            }
        };

        // Tab bar starts at design y=2100 (drm-splash.c's own
        // `root_page`/`render_root_controls` layout), scaled
        // proportionally against whatever size this surface actually
        // is -- same design-canvas convention as `tab_at()`.
        let tab_bar_start = ((height as u64 * 2100) / 2400) as u32;
        let content_pixel = self.current_page.color();
        let seg_width = (width / 4).max(1);
        for y in 0..height {
            let row_start = (y * width) as usize * 4;
            let row = &mut canvas[row_start..row_start + width as usize * 4];
            if y < tab_bar_start {
                for chunk in row.chunks_exact_mut(4) {
                    chunk.copy_from_slice(&content_pixel);
                }
            } else {
                for (x, chunk) in row.chunks_exact_mut(4).enumerate() {
                    let seg = ((x as u32 / seg_width) as usize).min(3);
                    let page = RootPage::ALL[seg];
                    let pixel = if page == self.current_page {
                        page.color()
                    } else {
                        page.dim_color()
                    };
                    chunk.copy_from_slice(&pixel);
                }
            }
        }

        self.window
            .wl_surface()
            .damage_buffer(0, 0, width as i32, height as i32);
        buffer
            .attach_to(self.window.wl_surface())
            .expect("buffer attach");
        self.window.commit();
    }

    /// Re-locks after `IDLE_TIMEOUT` of no touch activity while
    /// unlocked -- matches drm-splash.c's own 1s-granularity idle poll,
    /// just driven by this event loop's existing 16ms tick instead of a
    /// separate timer source.
    fn check_idle_timeout(&mut self, qh: &QueueHandle<Self>) {
        if self.locked || self.last_activity.elapsed() < IDLE_TIMEOUT {
            return;
        }
        println!("saai-shell: idle timeout, locking");
        self.last_activity = Instant::now();
        match self.session_lock_state.lock(qh) {
            Ok(session_lock) => self.session_lock = Some(session_lock),
            Err(err) => eprintln!("saai-shell: failed to re-lock: {err}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{tab_at, RootPage};

    #[test]
    fn bottom_bar_maps_all_four_tabs() {
        assert_eq!(tab_at((135.0, 2250.0), 1080, 2400), Some(RootPage::Now));
        assert_eq!(tab_at((405.0, 2250.0), 1080, 2400), Some(RootPage::Inbox));
        assert_eq!(tab_at((675.0, 2250.0), 1080, 2400), Some(RootPage::Spaces));
        assert_eq!(tab_at((945.0, 2250.0), 1080, 2400), Some(RootPage::Me));
    }

    #[test]
    fn content_area_is_not_a_tab() {
        assert_eq!(tab_at((540.0, 1200.0), 1080, 2400), None);
    }
}

delegate_compositor!(Shell);
delegate_layer!(Shell);
delegate_output!(Shell);
delegate_seat!(Shell);
delegate_session_lock!(Shell);
delegate_shm!(Shell);
delegate_touch!(Shell);
delegate_xdg_shell!(Shell);
delegate_xdg_window!(Shell);
delegate_registry!(Shell);

impl ProvidesRegistryState for Shell {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}
