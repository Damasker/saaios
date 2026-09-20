//! APP-03 / ADR-267: `zwp_input_method_manager_v2` advertised, GetInputMethod
//! does not require a keyboard, and `commit_string` reaches an enabled
//! text-input-v3 field. This is the compositor half of APP-03, not an OSK.

use std::io::{BufRead, BufReader};
use std::os::fd::AsFd;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use wayland_client::{
    globals::{registry_queue_init, GlobalListContents},
    protocol::{wl_buffer, wl_compositor, wl_registry, wl_seat, wl_shm, wl_shm_pool, wl_surface},
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols::wp::text_input::zv3::client::{
    zwp_text_input_manager_v3::ZwpTextInputManagerV3,
    zwp_text_input_v3::{self, ZwpTextInputV3},
};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};
use wayland_protocols_misc::zwp_input_method_v2::client::{
    zwp_input_method_keyboard_grab_v2::ZwpInputMethodKeyboardGrabV2,
    zwp_input_method_manager_v2::ZwpInputMethodManagerV2,
    zwp_input_method_v2::{self, ZwpInputMethodV2},
    zwp_input_popup_surface_v2::ZwpInputPopupSurfaceV2,
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
    configured: bool,
    entered: bool,
    commit_string: Option<String>,
    ime_activate: bool,
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
empty_dispatch!(wl_seat::WlSeat);
empty_dispatch!(wl_shm::WlShm);
empty_dispatch!(wl_shm_pool::WlShmPool);
empty_dispatch!(wl_buffer::WlBuffer);
empty_dispatch!(ZwpTextInputManagerV3);
empty_dispatch!(ZwpInputMethodManagerV2);
empty_dispatch!(ZwpInputMethodKeyboardGrabV2);
empty_dispatch!(ZwpInputPopupSurfaceV2);
empty_dispatch!(xdg_toplevel::XdgToplevel);

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

impl Dispatch<ZwpTextInputV3, ()> for ProbeState {
    fn event(
        state: &mut Self,
        _proxy: &ZwpTextInputV3,
        event: zwp_text_input_v3::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwp_text_input_v3::Event::Enter { .. } => state.entered = true,
            zwp_text_input_v3::Event::CommitString { text } => state.commit_string = text,
            _ => {}
        }
    }
}

impl Dispatch<ZwpInputMethodV2, ()> for ProbeState {
    fn event(
        state: &mut Self,
        _proxy: &ZwpInputMethodV2,
        event: zwp_input_method_v2::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if matches!(event, zwp_input_method_v2::Event::Activate) {
            state.ime_activate = true;
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
fn advertises_input_method_v2() {
    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    let (mut displayd, log) = spawn_displayd(runtime_dir.path());
    let socket_name = wait_for_socket(&log);
    let conn = connect(runtime_dir.path(), &socket_name);
    let (globals, _queue) = registry_queue_init::<ProbeState>(&conn).expect("registry init failed");
    let advertised = globals.contents().with_list(|list| {
        list.iter()
            .any(|global| global.interface == "zwp_input_method_manager_v2")
    });
    let _ = displayd.kill();
    let _ = displayd.wait();
    assert!(
        advertised,
        "zwp_input_method_manager_v2 was not advertised -- ADR-267 regression"
    );
}

#[test]
fn get_input_method_does_not_require_keyboard() {
    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    let (mut displayd, log) = spawn_displayd(runtime_dir.path());
    let socket_name = wait_for_socket(&log);
    let conn = connect(runtime_dir.path(), &socket_name);
    let (globals, mut queue) =
        registry_queue_init::<ProbeState>(&conn).expect("registry init failed");
    let qh = queue.handle();
    let mut state = ProbeState {
        configured: false,
        entered: false,
        commit_string: None,
        ime_activate: false,
    };
    let seat: wl_seat::WlSeat = globals
        .bind(&qh, 1..=9, ())
        .expect("wl_seat not advertised");
    let manager: ZwpInputMethodManagerV2 = globals
        .bind(&qh, 1..=1, ())
        .expect("zwp_input_method_manager_v2 not advertised -- ADR-267 regression");
    let ime = manager.get_input_method(&seat, &qh, ());
    let _grab = ime.grab_keyboard(&qh, ());
    queue
        .roundtrip(&mut state)
        .expect("compositor did not survive GetInputMethod without a keymap");
    assert!(
        displayd
            .try_wait()
            .expect("failed to poll saai-displayd")
            .is_none(),
        "saai-displayd exited on GetInputMethod -- ADR-022 failure mode"
    );
    let _ = displayd.kill();
    let _ = displayd.wait();
}

#[test]
fn commit_string_reaches_enabled_text_input() {
    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    let (mut displayd, log) = spawn_displayd(runtime_dir.path());
    let socket_name = wait_for_socket(&log);
    let conn = connect(runtime_dir.path(), &socket_name);
    let (globals, mut queue) =
        registry_queue_init::<ProbeState>(&conn).expect("registry init failed");
    let qh = queue.handle();
    let mut state = ProbeState {
        configured: false,
        entered: false,
        commit_string: None,
        ime_activate: false,
    };

    let compositor: wl_compositor::WlCompositor = globals.bind(&qh, 1..=6, ()).unwrap();
    let shm: wl_shm::WlShm = globals.bind(&qh, 1..=1, ()).unwrap();
    let wm: xdg_wm_base::XdgWmBase = globals.bind(&qh, 1..=6, ()).unwrap();
    let seat: wl_seat::WlSeat = globals.bind(&qh, 1..=9, ()).unwrap();
    let text_mgr: ZwpTextInputManagerV3 = globals.bind(&qh, 1..=1, ()).unwrap();
    let ime_mgr: ZwpInputMethodManagerV2 = globals.bind(&qh, 1..=1, ()).unwrap();

    let surface = compositor.create_surface(&qh, ());
    let xdg_surface = wm.get_xdg_surface(&surface, &qh, ());
    let _toplevel = xdg_surface.get_toplevel(&qh, ());
    let text_input = text_mgr.get_text_input(&seat, &qh, ());
    let ime = ime_mgr.get_input_method(&seat, &qh, ());
    surface.commit();

    for _ in 0..20 {
        queue.roundtrip(&mut state).unwrap();
        if state.configured {
            break;
        }
    }
    assert!(state.configured, "xdg_surface never configured");

    let file = tempfile::tempfile().expect("shm file");
    file.set_len(4).expect("ftruncate shm");
    let pool = shm.create_pool(file.as_fd(), 4, &qh, ());
    let buffer = pool.create_buffer(0, 1, 1, 4, wl_shm::Format::Argb8888, &qh, ());
    surface.attach(Some(&buffer), 0, 0);
    surface.damage(0, 0, 1, 1);
    surface.commit();
    queue.roundtrip(&mut state).unwrap();

    text_input.enable();
    text_input.commit();
    queue.roundtrip(&mut state).unwrap();
    assert!(
        state.entered || state.ime_activate,
        "text-input never entered and IME never activated"
    );

    ime.commit_string(String::from("hello"));
    ime.commit(0);
    queue.roundtrip(&mut state).unwrap();

    assert_eq!(
        state.commit_string.as_deref(),
        Some("hello"),
        "commit_string did not reach the enabled text-input field"
    );
    assert!(
        displayd
            .try_wait()
            .expect("failed to poll saai-displayd")
            .is_none(),
        "saai-displayd exited during commit_string"
    );
    let _ = displayd.kill();
    let _ = displayd.wait();
}
