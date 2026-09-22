//! APP-02 / ADR-311: `xdg_toplevel.configure_bounds` is the window
//! geometry, not smithay's default `(0, 0)`. GTK 4.14 treats any
//! bounds event as `has_bounds` and then `gdk_toplevel_size_init(0, 0)`
//! which produced `create_buffer(508, 2337935)` on panther (ADR-310).
//! Host-only: do not flash displayd this week.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use wayland_client::{
    globals::{registry_queue_init, GlobalListContents},
    protocol::{wl_compositor, wl_registry, wl_surface},
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

fn displayd_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_saai-displayd"))
}

fn spawn_displayd(runtime_dir: &std::path::Path) -> (Child, mpsc::Receiver<String>) {
    let mut cmd = Command::new(displayd_bin());
    cmd.env("XDG_RUNTIME_DIR", runtime_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("failed to spawn saai-displayd");
    let stdout = child.stdout.take().expect("child stdout not piped");
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    (child, rx)
}

fn wait_for_socket(log: &mpsc::Receiver<String>) -> String {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(
            Instant::now() < deadline,
            "saai-displayd never announced a listening socket"
        );
        match log.recv_timeout(Duration::from_millis(200)) {
            Ok(line) => {
                if let Some(name) =
                    line.strip_prefix("saai-displayd: listening on WAYLAND_DISPLAY=")
                {
                    return name.to_string();
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                panic!("saai-displayd exited before announcing a socket")
            }
        }
    }
}

struct ProbeState {
    configured: bool,
    bounds: Option<(i32, i32)>,
    size: Option<(i32, i32)>,
}

macro_rules! empty_dispatch {
    ($ty:ty) => {
        impl Dispatch<$ty, ()> for ProbeState {
            fn event(
                _state: &mut Self,
                _proxy: &$ty,
                _event: <$ty as wayland_client::Proxy>::Event,
                _data: &(),
                _conn: &Connection,
                _qh: &QueueHandle<Self>,
            ) {
            }
        }
    };
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for ProbeState {
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

empty_dispatch!(wl_compositor::WlCompositor);
empty_dispatch!(wl_surface::WlSurface);

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for ProbeState {
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

impl Dispatch<xdg_surface::XdgSurface, ()> for ProbeState {
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
            state.configured = true;
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, ()> for ProbeState {
    fn event(
        state: &mut Self,
        _proxy: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            xdg_toplevel::Event::Configure { width, height, .. } => {
                state.size = Some((width, height));
            }
            xdg_toplevel::Event::ConfigureBounds { width, height } => {
                state.bounds = Some((width, height));
            }
            _ => {}
        }
    }
}

#[test]
fn configure_bounds_is_windowed_geometry_not_zero() {
    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    let (mut displayd, log) = spawn_displayd(runtime_dir.path());
    let socket_name = wait_for_socket(&log);

    // SAFETY: this test process updates and reads these variables only on
    // this thread, immediately around Connection::connect_to_env().
    unsafe {
        std::env::set_var("XDG_RUNTIME_DIR", runtime_dir.path());
        std::env::set_var("WAYLAND_DISPLAY", &socket_name);
    }
    let conn = Connection::connect_to_env().expect("failed to connect to displayd");
    let (globals, mut queue) =
        registry_queue_init::<ProbeState>(&conn).expect("registry init failed");
    let qh = queue.handle();
    let mut state = ProbeState {
        configured: false,
        bounds: None,
        size: None,
    };

    let compositor: wl_compositor::WlCompositor = globals.bind(&qh, 1..=6, ()).unwrap();
    let wm: xdg_wm_base::XdgWmBase = globals.bind(&qh, 4..=6, ()).unwrap();
    let surface = compositor.create_surface(&qh, ());
    let xdg_surface = wm.get_xdg_surface(&surface, &qh, ());
    let _toplevel = xdg_surface.get_toplevel(&qh, ());
    surface.commit();

    for _ in 0..20 {
        queue.roundtrip(&mut state).unwrap();
        if state.configured && state.bounds.is_some() {
            break;
        }
    }
    assert!(state.configured, "xdg_surface never configured");
    let bounds = state
        .bounds
        .expect("configure_bounds must be sent (xdg-shell v4+)");
    assert_ne!(
        bounds,
        (0, 0),
        "GTK 4.14 treats configure_bounds(0,0) as a real 0×0 max size"
    );
    assert_eq!(
        bounds,
        (1280, 800),
        "host windowed bounds must match ADR-250 WINDOWED 1280×800"
    );
    assert_eq!(
        state.size,
        Some((1280, 800)),
        "configure size and bounds are the same window"
    );

    displayd.kill().ok();
    let _ = displayd.wait();
}
