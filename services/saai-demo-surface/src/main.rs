use std::io::Write;
use std::os::fd::AsFd;
use std::time::{Duration, Instant};

use wayland_client::{
    delegate_noop,
    globals::{registry_queue_init, GlobalListContents},
    protocol::{
        wl_compositor, wl_keyboard, wl_registry, wl_seat, wl_shm, wl_shm_pool, wl_surface, wl_touch,
    },
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

const WIDTH: i32 = 800;
const HEIGHT: i32 = 480;
const STRIDE: i32 = WIDTH * 4;

mod render;

struct AppState {
    label: String,
    running: bool,
    configured: bool,
    received_key: bool,
    installed_mode: bool,
    configured_width: i32,
    configured_height: i32,
    touch: Option<wl_touch::WlTouch>,
    touch_started: bool,
    touch_position: (f64, f64),
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
                if width > 0 && height > 0 {
                    state.configured_width = width;
                    state.configured_height = height;
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for AppState {
    fn event(
        state: &mut Self,
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
            if caps.contains(wl_seat::Capability::Touch) && state.touch.is_none() {
                state.touch = Some(proxy.get_touch(qh, ()));
            }
        }
    }
}

impl Dispatch<wl_touch::WlTouch, ()> for AppState {
    fn event(
        state: &mut Self,
        _proxy: &wl_touch::WlTouch,
        event: wl_touch::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_touch::Event::Down { x, y, .. } => {
                state.touch_started = true;
                state.touch_position = (x, y);
            }
            wl_touch::Event::Motion { x, y, .. } => state.touch_position = (x, y),
            wl_touch::Event::Up { .. } if state.touch_started => {
                state.touch_started = false;
                let close_top = state.configured_height.saturating_sub(360) as f64;
                if state.installed_mode && state.touch_position.1 >= close_top {
                    println!("saai-demo-surface[{}]: close action tapped", state.label);
                    state.running = false;
                }
            }
            wl_touch::Event::Cancel => state.touch_started = false,
            _ => {}
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

/// Builds a wl_shm-backed buffer of the fixed test-pattern size and
/// attaches it to `surface`, committing immediately. Shared by the normal
/// lifecycle path and the `violate-configure` negative-protocol test, which
/// calls this before any configure has been received.
fn attach_test_pattern(
    shm: &wl_shm::WlShm,
    surface: &wl_surface::WlSurface,
    qh: &QueueHandle<AppState>,
    installed_mode: bool,
    width: i32,
    height: i32,
) {
    let (width, height) = if installed_mode {
        (width.max(1), height.max(1))
    } else {
        (WIDTH, HEIGHT)
    };
    let stride = width * 4;
    let size = (stride * height) as usize;
    let mut file = tempfile::tempfile().expect("failed to create anonymous shm file");
    let mut pixels = vec![0u8; size];
    if installed_mode {
        render::draw(&mut pixels, width as u32, height as u32);
    } else {
        draw_test_pattern(&mut pixels);
    }
    file.write_all(&pixels).expect("failed to write pixel data");
    file.flush().ok();

    let pool = shm.create_pool(file.as_fd(), size as i32, qh, ());
    let format = if installed_mode {
        wl_shm::Format::Xrgb8888
    } else {
        wl_shm::Format::Argb8888
    };
    let buffer = pool.create_buffer(0, width, height, stride, format, qh, ());

    surface.attach(Some(&buffer), 0, 0);
    surface.damage_buffer(0, 0, width, height);
    surface.commit();
}

/// S07 Change 7's one complete end-to-end portal scenario: connects to
/// `saai-shell`'s portal socket as this real (possibly sandboxed) process,
/// writes a fixed string to the clipboard, reads it back, and confirms it
/// matches -- exercising the actual authorization path (this process's own
/// pid, resolved by `saai-shell` via `SO_PEERCRED` to whatever app_id
/// `saai-appd` launched it under, checked against that app's real granted
/// capabilities) rather than a synthetic/mocked one. Triggered only by
/// `SAAIOS_PORTAL_ROUNDTRIP` so normal launches (and the existing S02/S05
/// acceptance scripts) are unaffected.
fn run_portal_roundtrip() {
    use saai_portal_protocol::{
        encode_request, ClientRequest, ResponseResult, ServerMessage, PORTAL_WIRE_SCHEMA_V1,
    };
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;

    const TEST_TEXT: &str = "saaios-portal-roundtrip-test";

    let socket_path = std::env::var_os("SAAIOS_PORTAL_SOCKET")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/run/saaios/portal.sock"));

    let result = (|| -> Result<String, String> {
        let mut stream =
            UnixStream::connect(&socket_path).map_err(|error| format!("connect: {error}"))?;
        let mut reader = BufReader::new(stream.try_clone().map_err(|error| error.to_string())?);

        let write_request = ClientRequest::ClipboardWrite {
            schema: PORTAL_WIRE_SCHEMA_V1,
            request_id: "demo-surface:portal-write".into(),
            text: TEST_TEXT.into(),
        };
        stream
            .write_all(&encode_request(&write_request).map_err(|error| error.to_string())?)
            .map_err(|error| format!("write request: {error}"))?;
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .map_err(|error| format!("read write-response: {error}"))?;
        match serde_json::from_str(&line).map_err(|error| error.to_string())? {
            ServerMessage::Response {
                ok: true,
                result: Some(ResponseResult::ClipboardWritten),
                ..
            } => {}
            other => return Err(format!("clipboard_write did not succeed: {other:?}")),
        }

        let read_request = ClientRequest::ClipboardRead {
            schema: PORTAL_WIRE_SCHEMA_V1,
            request_id: "demo-surface:portal-read".into(),
        };
        stream
            .write_all(&encode_request(&read_request).map_err(|error| error.to_string())?)
            .map_err(|error| format!("write request: {error}"))?;
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .map_err(|error| format!("read read-response: {error}"))?;
        match serde_json::from_str(&line).map_err(|error| error.to_string())? {
            ServerMessage::Response {
                ok: true,
                result: Some(ResponseResult::ClipboardText { text }),
                ..
            } if text == TEST_TEXT => Ok(text),
            other => Err(format!(
                "clipboard_read did not return the written text: {other:?}"
            )),
        }
    })();

    match result {
        Ok(text) => println!("saai-demo-surface: portal round-trip OK, read back {text:?}"),
        Err(error) => println!("saai-demo-surface: portal round-trip FAILED: {error}"),
    }
}

fn main() {
    let label = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "demo".to_string());
    if std::env::var_os("SAAIOS_PORTAL_ROUNDTRIP").is_some() {
        run_portal_roundtrip();
        return;
    }
    // Negative-protocol test mode (S02 acceptance: "protocol: configure до
    // buffer attach"): attach a buffer on the very first commit, before any
    // configure was ever received. xdg-shell requires the first commit to be
    // buffer-less; a compliant compositor must reject this with a protocol
    // error and disconnect only this client.
    let violate_configure = std::env::args().nth(2).as_deref() == Some("violate-configure");
    let installed_mode = std::env::var_os("SAAIOS_APP_ID").is_some();
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

    let mut state = AppState {
        label: label.clone(),
        running: true,
        configured: false,
        received_key: false,
        installed_mode,
        configured_width: WIDTH,
        configured_height: HEIGHT,
        touch: None,
        touch_started: false,
        touch_position: (0.0, 0.0),
        _xdg_surface: Some(xdg_surface),
        surface: Some(surface),
        shm: Some(shm),
    };

    let mut buffer_attached = false;
    let deadline = Instant::now() + Duration::from_secs(8);

    if violate_configure {
        println!(
            "saai-demo-surface[{}]: violating protocol -- attaching buffer before any configure",
            state.label
        );
        attach_test_pattern(
            state.shm.as_ref().unwrap(),
            state.surface.as_ref().unwrap(),
            &qh,
            false,
            WIDTH,
            HEIGHT,
        );
        buffer_attached = true;
        // A compliant compositor answers this with a protocol error and
        // disconnects us; give it a bounded window to do so and observe
        // whichever happens (disconnect vs. some other reply) rather than
        // assuming a specific error shape.
        while Instant::now() < deadline {
            match queue.roundtrip(&mut state) {
                Ok(_) => std::thread::sleep(Duration::from_millis(100)),
                Err(err) => {
                    println!(
                        "saai-demo-surface[{}]: connection ended after protocol violation: {err}",
                        state.label
                    );
                    break;
                }
            }
        }
    } else {
        // Initial commit with no buffer attached triggers the first configure,
        // per xdg-shell's configure/ack/commit lifecycle.
        state.surface.as_ref().unwrap().commit();

        // Wait for the first configure with a real deadline: once the frame is
        // attached and committed there is nothing further to wait for, and
        // blocking_dispatch has no timeout of its own, so it must not be called
        // again after that point.
        while state.running && !buffer_attached && Instant::now() < deadline {
            queue
                .blocking_dispatch(&mut state)
                .expect("dispatch failed");

            if state.configured && !buffer_attached {
                attach_test_pattern(
                    state.shm.as_ref().unwrap(),
                    state.surface.as_ref().unwrap(),
                    &qh,
                    state.installed_mode,
                    state.configured_width,
                    state.configured_height,
                );
                buffer_attached = true;
                println!(
                    "saai-demo-surface[{}]: committed {}x{} {} frame",
                    state.label,
                    if state.installed_mode {
                        state.configured_width
                    } else {
                        WIDTH
                    },
                    if state.installed_mode {
                        state.configured_height
                    } else {
                        HEIGHT
                    },
                    if state.installed_mode {
                        "application"
                    } else {
                        "test pattern"
                    }
                );
            }
        }
    }

    // After the frame is committed, poll a bounded number of roundtrips so a
    // synthetic key injected into the compositor around this time (see
    // saai-displayd's stdin "inject-key" trigger) has a real chance to
    // arrive before this process exits -- each roundtrip is itself bounded
    // by the compositor's responsiveness, never an indefinite wait.
    if buffer_attached && !violate_configure && !installed_mode {
        while !state.received_key && Instant::now() < deadline {
            let _ = queue.roundtrip(&mut state);
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    if buffer_attached && installed_mode {
        while state.running {
            queue
                .blocking_dispatch(&mut state)
                .expect("installed application dispatch failed");
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
