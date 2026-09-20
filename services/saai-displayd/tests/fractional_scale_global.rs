//! APP-02 / ADR-266: `wp_fractional_scale_manager_v1` advertised, and a
//! client receives `preferred_scale` = 120 (scale 1.0) after binding it on
//! a `wl_surface`. `wp_viewporter` is advertised as the pair protocol GDK
//! expects. This does not run GTK4; it guards the compositor half of
//! ADR-025's uninitialized-scale path.

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
use wayland_protocols::wp::fractional_scale::v1::client::{
    wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
    wp_fractional_scale_v1::{self, WpFractionalScaleV1},
};
use wayland_protocols::wp::viewporter::client::{
    wp_viewport::WpViewport, wp_viewporter::WpViewporter,
};

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

struct ProbeState {
    preferred_scale: Option<u32>,
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

impl Dispatch<wl_compositor::WlCompositor, ()> for ProbeState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_compositor::WlCompositor,
        _event: wl_compositor::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_surface::WlSurface, ()> for ProbeState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_surface::WlSurface,
        _event: wl_surface::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WpFractionalScaleManagerV1, ()> for ProbeState {
    fn event(
        _state: &mut Self,
        _proxy: &WpFractionalScaleManagerV1,
        _event: <WpFractionalScaleManagerV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

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

impl Dispatch<WpViewporter, ()> for ProbeState {
    fn event(
        _state: &mut Self,
        _proxy: &WpViewporter,
        _event: <WpViewporter as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WpViewport, ()> for ProbeState {
    fn event(
        _state: &mut Self,
        _proxy: &WpViewport,
        _event: <WpViewport as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
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

#[test]
fn advertises_fractional_scale_and_viewporter() {
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
    let (globals, _queue) = registry_queue_init::<ProbeState>(&conn).expect("registry init failed");
    let (has_fractional, has_viewporter) = globals.contents().with_list(|list| {
        let fractional = list
            .iter()
            .any(|global| global.interface == "wp_fractional_scale_manager_v1");
        let viewporter = list
            .iter()
            .any(|global| global.interface == "wp_viewporter");
        (fractional, viewporter)
    });

    let _ = displayd.kill();
    let _ = displayd.wait();
    assert!(
        has_fractional,
        "wp_fractional_scale_manager_v1 was not advertised -- ADR-266 regression"
    );
    assert!(
        has_viewporter,
        "wp_viewporter was not advertised -- ADR-266 regression"
    );
}

#[test]
fn preferred_scale_is_one_after_bind() {
    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    let (mut displayd, log) = spawn_displayd(runtime_dir.path());
    let socket_name = wait_for_socket(&log);

    // SAFETY: this test process does not touch these vars from another
    // thread concurrently -- Connection::connect_to_env() reads them once,
    // synchronously, right below.
    unsafe {
        std::env::set_var("XDG_RUNTIME_DIR", runtime_dir.path());
        std::env::set_var("WAYLAND_DISPLAY", &socket_name);
    }
    let conn = Connection::connect_to_env().expect("failed to connect to saai-displayd");
    let (globals, mut queue) =
        registry_queue_init::<ProbeState>(&conn).expect("registry init failed");
    let qh = queue.handle();

    let mut state = ProbeState {
        preferred_scale: None,
    };
    let compositor: wl_compositor::WlCompositor = globals
        .bind(&qh, 1..=6, ())
        .expect("wl_compositor not advertised");
    let surface = compositor.create_surface(&qh, ());
    let manager: WpFractionalScaleManagerV1 = globals
        .bind(&qh, 1..=1, ())
        .expect("wp_fractional_scale_manager_v1 not advertised -- ADR-266 regression");
    let _scale_obj = manager.get_fractional_scale(&surface, &qh, ());
    let viewporter: WpViewporter = globals
        .bind(&qh, 1..=1, ())
        .expect("wp_viewporter not advertised -- ADR-266 regression");
    let _viewport = viewporter.get_viewport(&surface, &qh, ());

    queue
        .roundtrip(&mut state)
        .expect("compositor did not survive get_fractional_scale");

    assert_eq!(
        state.preferred_scale,
        Some(120),
        "preferred_scale must be 120 (scale 1.0) so GDK initializes *scale"
    );
    assert!(
        displayd
            .try_wait()
            .expect("failed to poll saai-displayd")
            .is_none(),
        "saai-displayd exited unexpectedly during the fractional-scale roundtrip"
    );

    let _ = displayd.kill();
    let _ = displayd.wait();
}
