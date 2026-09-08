//! `saai-shell`: the real system Wayland client for SaaiOS (ADR-005),
//! replacing drm-splash.c's hand-rolled UI. S04 Change step 3: minimal
//! vertical slice only -- connects to `saai-displayd` over the existing
//! `xdg_shell` path (no new protocols yet, those are Change 4/5's
//! ext-session-lock-v1/wlr-layer-shell per ADR-015) and presents a
//! solid placeholder frame. Run manually for now (`WAYLAND_DISPLAY=...
//! saai-shell`); ADR-014's auto-launch-as-saai-displayd's-child wiring
//! is Change step 7.
//!
//! Built on smithay-client-toolkit rather than raw wayland-client (unlike
//! the throwaway saai-demo-surface from S02): it already wraps both
//! protocols ADR-015 settled on, saving real rework once this client
//! needs them.

use std::time::Duration;

use smithay_client_toolkit::reexports::client::{
    globals::registry_queue_init,
    protocol::{wl_output, wl_shm, wl_surface},
    Connection, QueueHandle,
};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_output, delegate_registry, delegate_shm, delegate_xdg_shell,
    delegate_xdg_window,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
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
        shm,
        exit: false,
        first_configure: true,
        pool,
        width: 1080,
        height: 2400,
        buffer: None,
        window,
    };

    println!("saai-shell: connected, toplevel created");

    while !shell.exit {
        event_loop
            .dispatch(Duration::from_millis(16), &mut shell)
            .expect("event loop dispatch failed");
    }
}

struct Shell {
    registry_state: RegistryState,
    output_state: OutputState,
    shm: Shm,

    exit: bool,
    first_configure: bool,
    pool: SlotPool,
    width: u32,
    height: u32,
    buffer: Option<Buffer>,
    window: Window,
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
