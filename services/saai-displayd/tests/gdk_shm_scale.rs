//! APP-02 / ADR-286: after `preferred_scale=120`, a client using the
//! GDK formula (buffer = round(logical * preferred/120)) gets the
//! output size, not ADR-025's uninitialized `height=1776831`. This is
//! still not a GTK4 frame; it locks the compositor contract that
//! made the crash plausible.

use std::io::{BufRead, BufReader};
use std::os::fd::AsFd;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use wayland_client::{
    globals::{registry_queue_init, GlobalListContents},
    protocol::{wl_buffer, wl_compositor, wl_registry, wl_shm, wl_shm_pool, wl_surface},
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols::wp::fractional_scale::v1::client::{
    wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
    wp_fractional_scale_v1::{self, WpFractionalScaleV1},
};
use wayland_protocols::wp::viewporter::client::{
    wp_viewport::WpViewport, wp_viewporter::WpViewporter,
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

/// GDK `gdk_wayland_display_create_shm_surface` size when
/// `wp_fractional_scale_v1.preferred_scale` is the scale source.
fn gdk_buffer_size(logical_w: i32, logical_h: i32, preferred_scale: u32) -> (i32, i32) {
    let scale = f64::from(preferred_scale) / 120.0;
    (
        (f64::from(logical_w) * scale).round() as i32,
        (f64::from(logical_h) * scale).round() as i32,
    )
}

struct ProbeState {
    preferred_scale: Option<u32>,
    configured: bool,
    logical_width: i32,
    logical_height: i32,
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
empty_dispatch!(wl_shm::WlShm);
empty_dispatch!(wl_shm_pool::WlShmPool);
empty_dispatch!(wl_buffer::WlBuffer);
empty_dispatch!(WpFractionalScaleManagerV1);
empty_dispatch!(WpViewporter);
empty_dispatch!(WpViewport);

impl Dispatch<WpFractionalScaleV1, ()> for ProbeState {
    fn event(
        state: &mut Self,
        _proxy: &WpFractionalScaleV1,
        event: wp_fractional_scale_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = event {
            state.preferred_scale = Some(scale);
        }
    }
}

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
        if let xdg_toplevel::Event::Configure { width, height, .. } = event {
            state.logical_width = width;
            state.logical_height = height;
        }
    }
}

#[test]
fn gdk_scale_one_is_identity_not_adr025_height() {
    assert_eq!(gdk_buffer_size(1920, 1080, 120), (1920, 1080));
    assert_eq!(gdk_buffer_size(1080, 2400, 120), (1080, 2400));
    assert_ne!(gdk_buffer_size(1920, 1080, 120).1, 1_776_831);
}

#[test]
fn preferred_scale_yields_an_honest_shm_buffer() {
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
        preferred_scale: None,
        configured: false,
        logical_width: 0,
        logical_height: 0,
    };

    let compositor: wl_compositor::WlCompositor = globals.bind(&qh, 1..=6, ()).unwrap();
    let shm: wl_shm::WlShm = globals.bind(&qh, 1..=1, ()).unwrap();
    let wm: xdg_wm_base::XdgWmBase = globals.bind(&qh, 1..=6, ()).unwrap();
    let manager: WpFractionalScaleManagerV1 = globals
        .bind(&qh, 1..=1, ())
        .expect("wp_fractional_scale_manager_v1 not advertised");
    let viewporter: WpViewporter = globals.bind(&qh, 1..=1, ()).unwrap();

    let surface = compositor.create_surface(&qh, ());
    let _scale_obj = manager.get_fractional_scale(&surface, &qh, ());
    let _viewport = viewporter.get_viewport(&surface, &qh, ());
    let xdg_surface = wm.get_xdg_surface(&surface, &qh, ());
    let _toplevel = xdg_surface.get_toplevel(&qh, ());
    surface.commit();

    for _ in 0..20 {
        queue.roundtrip(&mut state).unwrap();
        if state.configured && state.preferred_scale.is_some() {
            break;
        }
    }
    assert!(state.configured, "xdg_surface never configured");
    assert_eq!(
        state.preferred_scale,
        Some(120),
        "preferred_scale must be 120 so GDK initializes *scale"
    );

    let logical_w = if state.logical_width > 0 {
        state.logical_width
    } else {
        1920
    };
    let logical_h = if state.logical_height > 0 {
        state.logical_height
    } else {
        1080
    };
    let (buf_w, buf_h) = gdk_buffer_size(logical_w, logical_h, state.preferred_scale.unwrap());
    assert_eq!(buf_h, logical_h, "scale 1.0 must not inflate height");
    assert_ne!(
        buf_h, 1_776_831,
        "ADR-025 uninitialized-scale height must not appear"
    );

    let stride = buf_w.checked_mul(4).expect("stride");
    let bytes = stride.checked_mul(buf_h).expect("shm bytes");
    let file = tempfile::tempfile().expect("shm file");
    file.set_len(bytes as u64).expect("ftruncate shm");
    let pool = shm.create_pool(file.as_fd(), bytes, &qh, ());
    let buffer = pool.create_buffer(0, buf_w, buf_h, stride, wl_shm::Format::Argb8888, &qh, ());
    surface.attach(Some(&buffer), 0, 0);
    surface.damage(0, 0, buf_w, buf_h);
    surface.commit();
    queue
        .roundtrip(&mut state)
        .expect("compositor did not survive an honest GDK-sized shm attach");

    assert!(
        displayd
            .try_wait()
            .expect("failed to poll saai-displayd")
            .is_none(),
        "saai-displayd exited during the GDK-sized shm roundtrip"
    );
    let _ = displayd.kill();
    let _ = displayd.wait();
}
