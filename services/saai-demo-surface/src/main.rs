use std::io::Write;
use std::os::fd::AsFd;
use std::time::{Duration, Instant};

use wayland_client::{
    delegate_noop,
    globals::{registry_queue_init, GlobalListContents},
    protocol::{wl_compositor, wl_keyboard, wl_registry, wl_seat, wl_shm, wl_shm_pool, wl_surface},
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

const WIDTH: i32 = 800;
const HEIGHT: i32 = 480;
const STRIDE: i32 = WIDTH * 4;

struct AppState {
    label: String,
    running: bool,
    configured: bool,
    received_key: bool,
    _xdg_surface: Option<xdg_surface::XdgSurface>,
    surface: Option<wl_surface::WlSurface>,
    shm: Option<wl_shm::WlShm>,
}

fn draw_test_pattern(buf: &mut [u8]) {
    // Unambiguous two-color split so a compositor-side hash check has a
    // deterministic, easily distinguishable frame to verify against.
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let offset = (y * STRIDE + x * 4) as usize;
            let color: u32 = if x < WIDTH / 2 {
                0xFF6C63FF
            } else {
                0xFF00CFA0
            };
            buf[offset..offset + 4].copy_from_slice(&color.to_le_bytes());
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for AppState {
    fn event(
        _state: &mut Self,
        proxy: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            proxy.pong(serial);
        }
    }
}

impl Dispatch<xdg_surface::XdgSurface, ()> for AppState {
    fn event(
        state: &mut Self,
        proxy: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            proxy.ack_configure(serial);
            if !state.configured {
                state.configured = true;
                println!(
                    "saai-demo-surface[{}]: received first configure, attaching test pattern buffer",
                    state.label
                );
            }
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, ()> for AppState {
    fn event(
        state: &mut Self,
        _proxy: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            xdg_toplevel::Event::Close => {
                println!(
                    "saai-demo-surface[{}]: compositor requested close",
                    state.label
                );
                state.running = false;
            }
            xdg_toplevel::Event::Configure { width, height, .. } => {
                println!(
                    "saai-demo-surface[{}]: toplevel configure {width}x{height}",
                    state.label
                );
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for AppState {
    fn event(
        _state: &mut Self,
        proxy: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities {
            capabilities: wayland_client::WEnum::Value(caps),
        } = event
        {
            if caps.contains(wl_seat::Capability::Keyboard) {
                proxy.get_keyboard(qh, ());
            }
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for AppState {
    fn event(
        state: &mut Self,
        _proxy: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_keyboard::Event::Key {
                key,
                state: key_state,
                ..
            } => {
                state.received_key = true;
                println!(
                    "saai-demo-surface[{}]: received key event, key={key} state={key_state:?}",
                    state.label
                );
            }
            wl_keyboard::Event::Enter { .. } => {
                println!(
                    "saai-demo-surface[{}]: keyboard focus entered this surface",
                    state.label
                );
            }
            wl_keyboard::Event::Leave { .. } => {
                println!(
                    "saai-demo-surface[{}]: keyboard focus left this surface",
                    state.label
                );
            }
            _ => {}
        }
    }
}

delegate_noop!(AppState: ignore wl_compositor::WlCompositor);
delegate_noop!(AppState: ignore wl_surface::WlSurface);
delegate_noop!(AppState: ignore wl_shm::WlShm);
delegate_noop!(AppState: wl_shm_pool::WlShmPool);
delegate_noop!(AppState: ignore wayland_client::protocol::wl_buffer::WlBuffer);

fn main() {
    let label = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "demo".to_string());
    let conn = Connection::connect_to_env().expect(
        "failed to connect to Wayland display -- set WAYLAND_DISPLAY to saai-displayd's socket",
    );
    let (globals, mut queue) =
        registry_queue_init::<AppState>(&conn).expect("registry init failed");
    let qh = queue.handle();

    let compositor: wl_compositor::WlCompositor = globals
        .bind(&qh, 1..=6, ())
        .expect("wl_compositor not advertised");
    let shm: wl_shm::WlShm = globals.bind(&qh, 1..=1, ()).expect("wl_shm not advertised");
    let wm_base: xdg_wm_base::XdgWmBase = globals
        .bind(&qh, 1..=6, ())
        .expect("xdg_wm_base not advertised");
    let _seat: wl_seat::WlSeat = globals
        .bind(&qh, 1..=1, ())
        .expect("wl_seat not advertised");

    let surface = compositor.create_surface(&qh, ());
    let xdg_surface = wm_base.get_xdg_surface(&surface, &qh, ());
    let toplevel = xdg_surface.get_toplevel(&qh, ());
    toplevel.set_title(format!("saai-demo-surface-{label}"));
    toplevel.set_app_id(format!("dev.saaios.demo-surface.{label}"));

    // Initial commit with no buffer attached triggers the first configure,
    // per xdg-shell's configure/ack/commit lifecycle.
    surface.commit();

    let mut state = AppState {
        label: label.clone(),
        running: true,
        configured: false,
        received_key: false,
        _xdg_surface: Some(xdg_surface),
        surface: Some(surface),
        shm: Some(shm),
    };

    let mut buffer_attached = false;
    let deadline = Instant::now() + Duration::from_secs(8);

    // Wait for the first configure with a real deadline: once the frame is
    // attached and committed there is nothing further to wait for, and
    // blocking_dispatch has no timeout of its own, so it must not be called
    // again after that point.
    while state.running && !buffer_attached && Instant::now() < deadline {
        queue
            .blocking_dispatch(&mut state)
            .expect("dispatch failed");

        if state.configured && !buffer_attached {
            let size = (STRIDE * HEIGHT) as usize;
            let mut file = tempfile::tempfile().expect("failed to create anonymous shm file");
            let mut pixels = vec![0u8; size];
            draw_test_pattern(&mut pixels);
            file.write_all(&pixels).expect("failed to write pixel data");
            file.flush().ok();

            let pool = state
                .shm
                .as_ref()
                .unwrap()
                .create_pool(file.as_fd(), size as i32, &qh, ());
            let buffer =
                pool.create_buffer(0, WIDTH, HEIGHT, STRIDE, wl_shm::Format::Argb8888, &qh, ());

            let surface = state.surface.as_ref().unwrap();
            surface.attach(Some(&buffer), 0, 0);
            surface.damage_buffer(0, 0, WIDTH, HEIGHT);
            surface.commit();
            buffer_attached = true;
            println!(
                "saai-demo-surface[{}]: committed {WIDTH}x{HEIGHT} test pattern frame",
                state.label
            );
        }
    }

    // After the frame is committed, poll a bounded number of roundtrips so a
    // synthetic key injected into the compositor around this time (see
    // saai-displayd's stdin "inject-key" trigger) has a real chance to
    // arrive before this process exits -- each roundtrip is itself bounded
    // by the compositor's responsiveness, never an indefinite wait.
    if buffer_attached {
        while !state.received_key && Instant::now() < deadline {
            let _ = queue.roundtrip(&mut state);
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    if state.received_key {
        println!(
            "saai-demo-surface[{}]: observed the injected key event",
            state.label
        );
    }
    println!("saai-demo-surface[{}]: exiting cleanly", state.label);
}
