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
//! navigation-style system surfaces) is not implemented yet -- separate
//! follow-up within this same Change step.

use std::time::Duration;

use smithay_client_toolkit::reexports::client::{
    globals::registry_queue_init,
    protocol::{wl_output, wl_shm, wl_surface},
    Connection, QueueHandle,
};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_output, delegate_registry, delegate_session_lock, delegate_shm,
    delegate_xdg_shell, delegate_xdg_window,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    session_lock::{
        SessionLock, SessionLockHandler, SessionLockState, SessionLockSurface,
        SessionLockSurfaceConfigure,
    },
    shell::{
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

    let surface = compositor.create_surface(&qh);
    let window = xdg_shell.create_window(surface, WindowDecorations::ServerDefault, &qh);
    window.set_title("SaaiOS");
    window.set_app_id("org.saaios.shell");
    // Fullscreen, not a resizable desktop window -- saai-shell is the
    // system shell, not an app; matches drm-splash.c's own fixed
    // 1080x2400 panel assumption for now (real multi-output handling is
    // future work, not this vertical slice).
    window.set_min_size(Some((1080, 2400)));
    window.commit();

    let pool = SlotPool::new(1080 * 2400 * 4, &shm).expect("failed to create SHM pool");

    let mut shell = Shell {
        registry_state: RegistryState::new(&globals),
        output_state: OutputState::new(&globals, &qh),
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
    };

    println!("saai-shell: connected, toplevel created");

    // Change 4 test trigger: lock immediately so the security property
    // (touch reaches only the lock surface, never the toplevel
    // underneath) can be verified physically without needing a real
    // idle timeout yet -- that's Change step 5, ported from
    // drm-splash.c's own 60s idle-to-lock behavior.
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
    }
}

struct Shell {
    registry_state: RegistryState,
    output_state: OutputState,
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
        conn: &Connection,
        qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        self.draw(conn, qh);
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
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
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
        surface.frame(qh, surface.clone());
        buffer.attach_to(surface).expect("buffer attach");
        surface.commit();
        self.lock_buffer = Some(buffer);
    }
}

impl Shell {
    /// Solid placeholder fill -- proves the real client<->compositor
    /// vertical slice end to end (surface creation, configure, SHM
    /// buffer, commit, frame callback). Actual shell content (lock
    /// screen, four sections) is Change step 5/6.
    fn draw(&mut self, _conn: &Connection, qh: &QueueHandle<Self>) {
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

        // Dark slate placeholder -- distinguishable from both
        // saai-demo-surface's test pattern and a black/uninitialized
        // screen when observed physically.
        let pixel: [u8; 4] = [0x30, 0x2a, 0x20, 0x00];
        for chunk in canvas.chunks_exact_mut(4) {
            chunk.copy_from_slice(&pixel);
        }

        self.window
            .wl_surface()
            .damage_buffer(0, 0, width as i32, height as i32);
        self.window
            .wl_surface()
            .frame(qh, self.window.wl_surface().clone());
        buffer
            .attach_to(self.window.wl_surface())
            .expect("buffer attach");
        self.window.commit();
    }
}

delegate_compositor!(Shell);
delegate_output!(Shell);
delegate_session_lock!(Shell);
delegate_shm!(Shell);
delegate_xdg_shell!(Shell);
delegate_xdg_window!(Shell);
delegate_registry!(Shell);

impl ProvidesRegistryState for Shell {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}
