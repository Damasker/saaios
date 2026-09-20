//! APP-04 / ADR-271: a bottom layer-shell surface keeps the height it
//! asked for. The compositor used to force every layer to a 120px top
//! strip, which cannot host an OSK.

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
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::{self, ZwlrLayerShellV1},
    zwlr_layer_surface_v1::{self, ZwlrLayerSurfaceV1},
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
    configured_width: u32,
    configured_height: u32,
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

empty_dispatch!(wl_compositor::WlCompositor);
empty_dispatch!(wl_surface::WlSurface);
empty_dispatch!(ZwlrLayerShellV1);

impl Dispatch<ZwlrLayerSurfaceV1, ()> for ProbeState {
    fn event(
        state: &mut Self,
        proxy: &ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let zwlr_layer_surface_v1::Event::Configure {
            serial,
            width,
            height,
        } = event
        {
            proxy.ack_configure(serial);
            state.configured_width = width;
            state.configured_height = height;
        }
    }
}

fn connect(runtime_dir: &std::path::Path, socket_name: &str) -> Connection {
    unsafe {
        std::env::set_var("XDG_RUNTIME_DIR", runtime_dir);
        std::env::set_var("WAYLAND_DISPLAY", socket_name);
    }
    Connection::connect_to_env().expect("failed to connect to saai-displayd")
}

#[test]
fn bottom_layer_keeps_requested_height() {
    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    let (mut displayd, log) = spawn_displayd(runtime_dir.path());
    let socket_name = wait_for_socket(&log);
    let conn = connect(runtime_dir.path(), &socket_name);
    let (globals, mut queue) =
        registry_queue_init::<ProbeState>(&conn).expect("registry init failed");
    let qh = queue.handle();
    let mut state = ProbeState {
        configured_width: 0,
        configured_height: 0,
    };

    let compositor: wl_compositor::WlCompositor = globals.bind(&qh, 1..=6, ()).unwrap();
    let shell: ZwlrLayerShellV1 = globals
        .bind(&qh, 1..=4, ())
        .expect("zwlr_layer_shell_v1 not advertised");
    let surface = compositor.create_surface(&qh, ());
    let layer = shell.get_layer_surface(
        &surface,
        None,
        zwlr_layer_shell_v1::Layer::Top,
        String::from("osk"),
        &qh,
        (),
    );
    layer.set_anchor(
        zwlr_layer_surface_v1::Anchor::Bottom
            | zwlr_layer_surface_v1::Anchor::Left
            | zwlr_layer_surface_v1::Anchor::Right,
    );
    layer.set_size(0, 800);
    surface.commit();

    for _ in 0..40 {
        queue.roundtrip(&mut state).unwrap();
        if state.configured_height == 800 {
            break;
        }
    }

    assert_eq!(
        state.configured_height, 800,
        "bottom OSK layer was forced to the 120px status-bar height"
    );
    assert!(
        state.configured_width > 0,
        "layer width stayed zero after configure"
    );
    let _ = displayd.kill();
    let _ = displayd.wait();
}
