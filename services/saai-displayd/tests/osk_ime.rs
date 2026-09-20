//! APP-04 / ADR-270: a separate OSK client (not the field's client) delivers
//! Keyboard keystrokes through `zwp_input_method_v2`. Not wvkbd, not
//! `zwp_virtual_keyboard_v1`, not a panther flash.

use std::io::{BufRead, BufReader};
use std::os::fd::AsFd;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use saai_ui_core::{Keyboard, OskImeOp};
use wayland_client::{
    globals::{registry_queue_init, GlobalListContents},
    protocol::{wl_buffer, wl_compositor, wl_registry, wl_seat, wl_shm, wl_shm_pool, wl_surface},
    Connection, Dispatch, EventQueue, QueueHandle,
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
    ime_activate: bool,
    buffer: String,
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
            zwp_text_input_v3::Event::CommitString { text } => {
                if let Some(text) = text {
                    state.buffer.push_str(&text);
                }
            }
            zwp_text_input_v3::Event::DeleteSurroundingText {
                before_length,
                after_length,
            } => {
                let cursor = state.buffer.len();
                let start = cursor.saturating_sub(before_length as usize);
                let end = cursor
                    .saturating_add(after_length as usize)
                    .min(state.buffer.len());
                state.buffer.replace_range(start..end, "");
            }
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

fn empty_state() -> ProbeState {
    ProbeState {
        configured: false,
        entered: false,
        ime_activate: false,
        buffer: String::new(),
    }
}

fn roundtrip_both(
    app_queue: &mut EventQueue<ProbeState>,
    app: &mut ProbeState,
    osk_queue: &mut EventQueue<ProbeState>,
    osk: &mut ProbeState,
) {
    app_queue.roundtrip(app).expect("app roundtrip");
    osk_queue.roundtrip(osk).expect("osk roundtrip");
    // IME requests live on the OSK connection. The field client only
    // sees commit_string after a follow-up read.
    app_queue.roundtrip(app).expect("app follow-up roundtrip");
}

fn send_ime_op(ime: &ZwpInputMethodV2, op: &OskImeOp) {
    match op {
        OskImeOp::CommitString(text) => ime.commit_string(text.clone()),
        OskImeOp::DeleteSurrounding {
            before_bytes,
            after_bytes,
        } => ime.delete_surrounding_text(*before_bytes, *after_bytes),
    }
    ime.commit(0);
}

#[test]
fn separate_osk_client_types_hi_bang_into_foreign_field() {
    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    let (mut displayd, log) = spawn_displayd(runtime_dir.path());
    let socket_name = wait_for_socket(&log);

    let app_conn = connect(runtime_dir.path(), &socket_name);
    let (app_globals, mut app_queue) =
        registry_queue_init::<ProbeState>(&app_conn).expect("app registry");
    let app_qh = app_queue.handle();
    let mut app = empty_state();

    let osk_conn = connect(runtime_dir.path(), &socket_name);
    let (osk_globals, mut osk_queue) =
        registry_queue_init::<ProbeState>(&osk_conn).expect("osk registry");
    let osk_qh = osk_queue.handle();
    let mut osk = empty_state();

    let compositor: wl_compositor::WlCompositor = app_globals.bind(&app_qh, 1..=6, ()).unwrap();
    let shm: wl_shm::WlShm = app_globals.bind(&app_qh, 1..=1, ()).unwrap();
    let wm: xdg_wm_base::XdgWmBase = app_globals.bind(&app_qh, 1..=6, ()).unwrap();
    let app_seat: wl_seat::WlSeat = app_globals.bind(&app_qh, 1..=9, ()).unwrap();
    let text_mgr: ZwpTextInputManagerV3 = app_globals.bind(&app_qh, 1..=1, ()).unwrap();

    let osk_seat: wl_seat::WlSeat = osk_globals.bind(&osk_qh, 1..=9, ()).unwrap();
    let ime_mgr: ZwpInputMethodManagerV2 = osk_globals
        .bind(&osk_qh, 1..=1, ())
        .expect("zwp_input_method_manager_v2 not advertised");
    let ime = ime_mgr.get_input_method(&osk_seat, &osk_qh, ());

    let surface = compositor.create_surface(&app_qh, ());
    let xdg_surface = wm.get_xdg_surface(&surface, &app_qh, ());
    let _toplevel = xdg_surface.get_toplevel(&app_qh, ());
    let text_input = text_mgr.get_text_input(&app_seat, &app_qh, ());
    surface.commit();

    for _ in 0..20 {
        roundtrip_both(&mut app_queue, &mut app, &mut osk_queue, &mut osk);
        if app.configured {
            break;
        }
    }
    assert!(app.configured, "xdg_surface never configured");

    let file = tempfile::tempfile().expect("shm file");
    file.set_len(4).expect("ftruncate shm");
    let pool = shm.create_pool(file.as_fd(), 4, &app_qh, ());
    let buffer = pool.create_buffer(0, 1, 1, 4, wl_shm::Format::Argb8888, &app_qh, ());
    surface.attach(Some(&buffer), 0, 0);
    surface.damage(0, 0, 1, 1);
    surface.commit();
    roundtrip_both(&mut app_queue, &mut app, &mut osk_queue, &mut osk);

    text_input.enable();
    text_input.commit();
    for _ in 0..20 {
        roundtrip_both(&mut app_queue, &mut app, &mut osk_queue, &mut osk);
        if app.entered || osk.ime_activate {
            break;
        }
    }
    assert!(
        app.entered || osk.ime_activate,
        "foreign field never entered and OSK IME never activated"
    );

    let keyboard = Keyboard::bind_foreign_ime();
    assert!(keyboard.shows_panel());
    let actions = [
        "intent:key:h",
        "intent:key:i",
        "intent:mode:toggle",
        "intent:backspace",
        "intent:key:i",
        "intent:key:!",
    ];
    for action in actions {
        let stroke = Keyboard::keystroke_from_osk_action(action)
            .unwrap_or_else(|| panic!("unmapped OSK action {action}"));
        if let Some(op) = stroke.to_ime_op() {
            send_ime_op(&ime, &op);
        }
        roundtrip_both(&mut app_queue, &mut app, &mut osk_queue, &mut osk);
    }

    assert_eq!(
        app.buffer, "hi!",
        "separate OSK client did not type hi! into the foreign field"
    );
    assert!(
        displayd
            .try_wait()
            .expect("failed to poll saai-displayd")
            .is_none(),
        "saai-displayd exited during OSK commit_string"
    );
    let _ = displayd.kill();
    let _ = displayd.wait();
}
