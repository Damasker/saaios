//! APP-04 / ADR-325: host GTK 4.18 Entry with grab_focus receives OSK
//! IME `commit_string` as widget text. Packed musl GTK 4.14.4 Entry
//! under qemu is the panther toolkit (ADR-343). Not a panther typed field.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use saai_ui_core::{Keyboard, OskImeOp};
use wayland_client::{
    globals::{registry_queue_init, GlobalListContents},
    protocol::{wl_registry, wl_seat},
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols_misc::zwp_input_method_v2::client::{
    zwp_input_method_keyboard_grab_v2::ZwpInputMethodKeyboardGrabV2,
    zwp_input_method_manager_v2::ZwpInputMethodManagerV2,
    zwp_input_method_v2::{self, ZwpInputMethodV2},
    zwp_input_popup_surface_v2::ZwpInputPopupSurfaceV2,
};

fn displayd_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_saai-displayd"))
}

fn gtk4_probe() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/gtk4_hello.py")
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

fn gtk4_import_available() -> bool {
    Command::new("python3")
        .args([
            "-c",
            "import gi; gi.require_version('Gtk','4.0'); from gi.repository import Gtk",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

struct ImeState {
    activate: bool,
}

macro_rules! empty_dispatch {
    ($ty:ty) => {
        impl Dispatch<$ty, ()> for ImeState {
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

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for ImeState {
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

empty_dispatch!(wl_seat::WlSeat);
empty_dispatch!(ZwpInputMethodManagerV2);
empty_dispatch!(ZwpInputMethodKeyboardGrabV2);
empty_dispatch!(ZwpInputPopupSurfaceV2);

impl Dispatch<ZwpInputMethodV2, ()> for ImeState {
    fn event(
        state: &mut Self,
        _proxy: &ZwpInputMethodV2,
        event: zwp_input_method_v2::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if matches!(event, zwp_input_method_v2::Event::Activate) {
            state.activate = true;
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
fn osk_ime_types_hi_bang_into_gtk4_entry() {
    assert!(
        gtk4_import_available(),
        "host GTK4 probe needs python3 + gir1.2-gtk-4.0 (Gtk 4.0)"
    );
    let probe = gtk4_probe();
    assert!(probe.is_file(), "missing GTK4 probe at {}", probe.display());

    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    let (mut displayd, log) = spawn_displayd(runtime_dir.path());
    let socket_name = wait_for_socket(&log);

    let conn = connect(runtime_dir.path(), &socket_name);
    let (globals, mut queue) = registry_queue_init::<ImeState>(&conn).expect("ime registry");
    let qh = queue.handle();
    let mut ime_state = ImeState { activate: false };
    let seat: wl_seat::WlSeat = globals.bind(&qh, 1..=9, ()).unwrap();
    let ime_mgr: ZwpInputMethodManagerV2 = globals
        .bind(&qh, 1..=1, ())
        .expect("zwp_input_method_manager_v2 not advertised");
    let ime = ime_mgr.get_input_method(&seat, &qh, ());
    queue
        .roundtrip(&mut ime_state)
        .expect("ime first roundtrip");

    let mut gtk = Command::new("python3")
        .arg(&probe)
        .env("XDG_RUNTIME_DIR", runtime_dir.path())
        .env("WAYLAND_DISPLAY", &socket_name)
        .env("GTK4_PROBE_HOLD", "1")
        .env("GDK_BACKEND", "wayland")
        .env("GSK_RENDERER", "cairo")
        .env("GTK_A11Y", "none")
        .env("NO_AT_BRIDGE", "1")
        .env_remove("DISPLAY")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn gtk4_hello.py");
    let gtk_out = {
        let stdout = gtk.stdout.take().expect("gtk stdout");
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        rx
    };

    let mut saw_enable = false;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline && !(saw_enable && ime_state.activate) {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("text-input-v3 enable") {
                    saw_enable = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }
    assert!(
        saw_enable && ime_state.activate,
        "GTK Entry never enabled v3 / IME never Activate; displayd={lines:?}"
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
        let _ = queue.roundtrip(&mut ime_state);
    }

    let mut entry = String::new();
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        match gtk_out.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if let Some(text) = line.strip_prefix("GTK_ENTRY_TEXT=") {
                    entry = text.to_string();
                    if entry == "hi!" {
                        break;
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }

    let _ = gtk.kill();
    let _ = gtk.wait();
    assert_eq!(
        entry, "hi!",
        "OSK IME did not type hi! into GTK 4.18 Entry; displayd={lines:?}"
    );
    let _ = displayd.kill();
    let _ = displayd.wait();
}

fn gtk4_alpine_probe() -> PathBuf {
    std::env::var("GTK4_ALPINE_PROBE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/gtk4-probe"))
}

fn packed_gtk414_entry() -> PathBuf {
    if let Ok(p) = std::env::var("PACKED_GTK414_ENTRY") {
        return PathBuf::from(p);
    }
    gtk4_alpine_probe().join("bin/gtk414-entry")
}

#[test]
fn osk_ime_types_hi_bang_into_alpine_gtk414_entry() {
    let probe = gtk4_alpine_probe();
    let bin = packed_gtk414_entry();
    let loader = probe.join("lib/ld-musl-aarch64.so.1");
    let xkb = probe.join("share/X11/xkb");
    assert!(
        bin.is_file(),
        "missing Alpine GTK 4.14 Entry at {} — set PACKED_GTK414_ENTRY",
        bin.display()
    );
    assert!(
        loader.is_file(),
        "missing musl loader at {}",
        loader.display()
    );
    assert!(xkb.is_dir(), "missing XKB_CONFIG_ROOT at {}", xkb.display());

    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(runtime_dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("XDG_RUNTIME_DIR 0700");
    }
    let (mut displayd, log) = spawn_displayd(runtime_dir.path());
    let socket_name = wait_for_socket(&log);

    let conn = connect(runtime_dir.path(), &socket_name);
    let (globals, mut queue) = registry_queue_init::<ImeState>(&conn).expect("ime registry");
    let qh = queue.handle();
    let mut ime_state = ImeState { activate: false };
    let seat: wl_seat::WlSeat = globals.bind(&qh, 1..=9, ()).unwrap();
    let ime_mgr: ZwpInputMethodManagerV2 = globals
        .bind(&qh, 1..=1, ())
        .expect("zwp_input_method_manager_v2 not advertised");
    let ime = ime_mgr.get_input_method(&seat, &qh, ());
    queue
        .roundtrip(&mut ime_state)
        .expect("ime first roundtrip");

    let probe = probe.canonicalize().expect("canonicalize gtk4 probe");
    let bin = if bin.is_absolute() {
        bin
    } else {
        probe.join("bin/gtk414-entry")
    };
    let path = std::env::var("PATH").unwrap_or_default();
    let mut gtk = Command::new("qemu-aarch64-static")
        .arg("-L")
        .arg(&probe)
        .arg(&bin)
        .env_clear()
        .env("PATH", &path)
        .env("XDG_RUNTIME_DIR", runtime_dir.path())
        .env("WAYLAND_DISPLAY", &socket_name)
        .env("GTK4_PROBE_HOLD", "1")
        .env("GDK_BACKEND", "wayland")
        .env("GSK_RENDERER", "cairo")
        .env("GTK_A11Y", "none")
        .env("NO_AT_BRIDGE", "1")
        .env("XKB_CONFIG_ROOT", probe.join("share/X11/xkb"))
        .env("GIO_USE_VFS", "local")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("qemu-aarch64-static failed to spawn gtk414-entry");
    let gtk_out = {
        let stdout = gtk.stdout.take().expect("gtk stdout");
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        rx
    };

    let mut saw_enable = false;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline && !(saw_enable && ime_state.activate) {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("text-input-v3 enable") {
                    saw_enable = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }
    assert!(
        saw_enable && ime_state.activate,
        "Alpine GTK 4.14 Entry never enabled v3 / IME never Activate; displayd={lines:?}"
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
        let _ = queue.roundtrip(&mut ime_state);
    }

    let mut entry = String::new();
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        match gtk_out.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if let Some(text) = line.strip_prefix("GTK_ENTRY_TEXT=") {
                    entry = text.to_string();
                    if entry == "hi!" {
                        break;
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }

    let _ = gtk.kill();
    let _ = gtk.wait();
    assert_eq!(
        entry, "hi!",
        "OSK IME did not type hi! into Alpine GTK 4.14 Entry; displayd={lines:?}"
    );
    let _ = displayd.kill();
    let _ = displayd.wait();
}

/// ADR-345: packed GTK 4.14 Entry without grab_focus still enables v3
/// once the host seat gives keyboard focus. Not a panther touch seat.
#[test]
fn osk_ime_types_hi_bang_into_alpine_gtk414_entry_without_grab() {
    let probe = gtk4_alpine_probe();
    let bin = packed_gtk414_entry();
    let loader = probe.join("lib/ld-musl-aarch64.so.1");
    let xkb = probe.join("share/X11/xkb");
    assert!(
        bin.is_file(),
        "missing Alpine GTK 4.14 Entry at {} — set PACKED_GTK414_ENTRY",
        bin.display()
    );
    assert!(
        loader.is_file(),
        "missing musl loader at {}",
        loader.display()
    );
    assert!(xkb.is_dir(), "missing XKB_CONFIG_ROOT at {}", xkb.display());

    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(runtime_dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("XDG_RUNTIME_DIR 0700");
    }
    let (mut displayd, log) = spawn_displayd(runtime_dir.path());
    let socket_name = wait_for_socket(&log);

    let conn = connect(runtime_dir.path(), &socket_name);
    let (globals, mut queue) = registry_queue_init::<ImeState>(&conn).expect("ime registry");
    let qh = queue.handle();
    let mut ime_state = ImeState { activate: false };
    let seat: wl_seat::WlSeat = globals.bind(&qh, 1..=9, ()).unwrap();
    let ime_mgr: ZwpInputMethodManagerV2 = globals
        .bind(&qh, 1..=1, ())
        .expect("zwp_input_method_manager_v2 not advertised");
    let ime = ime_mgr.get_input_method(&seat, &qh, ());
    queue
        .roundtrip(&mut ime_state)
        .expect("ime first roundtrip");

    let probe = probe.canonicalize().expect("canonicalize gtk4 probe");
    let bin = if bin.is_absolute() {
        bin
    } else {
        probe.join("bin/gtk414-entry")
    };
    let path = std::env::var("PATH").unwrap_or_default();
    let mut gtk = Command::new("qemu-aarch64-static")
        .arg("-L")
        .arg(&probe)
        .arg(&bin)
        .env_clear()
        .env("PATH", &path)
        .env("XDG_RUNTIME_DIR", runtime_dir.path())
        .env("WAYLAND_DISPLAY", &socket_name)
        .env("GTK4_PROBE_HOLD", "1")
        .env("GTK4_NO_GRAB", "1")
        .env("GDK_BACKEND", "wayland")
        .env("GSK_RENDERER", "cairo")
        .env("GTK_A11Y", "none")
        .env("NO_AT_BRIDGE", "1")
        .env("XKB_CONFIG_ROOT", probe.join("share/X11/xkb"))
        .env("GIO_USE_VFS", "local")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("qemu-aarch64-static failed to spawn gtk414-entry");
    let gtk_out = {
        let stdout = gtk.stdout.take().expect("gtk stdout");
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        rx
    };

    let mut saw_toplevel = false;
    let mut saw_enable = false;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline && !(saw_enable && ime_state.activate) {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("new xdg_toplevel") {
                    saw_toplevel = true;
                }
                if line.contains("text-input-v3 enable") {
                    saw_enable = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }
    assert!(
        saw_toplevel && saw_enable && ime_state.activate,
        "GTK 4.14 Entry without grab_focus never enabled v3 after seat keyboard focus; displayd={lines:?}"
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
        let _ = queue.roundtrip(&mut ime_state);
    }

    let mut entry = String::new();
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        match gtk_out.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if let Some(text) = line.strip_prefix("GTK_ENTRY_TEXT=") {
                    entry = text.to_string();
                    if entry == "hi!" {
                        break;
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }

    let _ = gtk.kill();
    let _ = gtk.wait();
    assert_eq!(
        entry, "hi!",
        "OSK IME did not type hi! into GTK 4.14 Entry without grab_focus; displayd={lines:?}"
    );
    let _ = displayd.kill();
    let _ = displayd.wait();
}

fn interesting(line: &str) -> bool {
    line.contains("text-input-v3")
        || line.contains("injected click")
        || line.contains("new xdg_toplevel")
        || line.contains("frame sha256=")
}

/// ADR-344: packed gtk4-demo `--run=entry` binds v3; a center click
/// does not enable. Not a Y sweep. Not a panther typed field.
#[test]
fn osk_ime_reaches_alpine_gtk4_demo_entry() {
    let probe = gtk4_alpine_probe();
    let demo = probe.join("bin/gtk4-demo");
    let loader = probe.join("lib/ld-musl-aarch64.so.1");
    let xkb = probe.join("share/X11/xkb");
    assert!(
        demo.is_file(),
        "missing Alpine gtk4-demo at {} — set GTK4_ALPINE_PROBE",
        demo.display()
    );
    assert!(
        loader.is_file(),
        "missing musl loader at {}",
        loader.display()
    );
    assert!(xkb.is_dir(), "missing XKB_CONFIG_ROOT at {}", xkb.display());

    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(runtime_dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("XDG_RUNTIME_DIR 0700");
    }
    let (mut displayd, log) = spawn_displayd(runtime_dir.path());
    let socket_name = wait_for_socket(&log);

    let conn = connect(runtime_dir.path(), &socket_name);
    let (globals, mut queue) = registry_queue_init::<ImeState>(&conn).expect("ime registry");
    let qh = queue.handle();
    let mut ime_state = ImeState { activate: false };
    let seat: wl_seat::WlSeat = globals.bind(&qh, 1..=9, ()).unwrap();
    let ime_mgr: ZwpInputMethodManagerV2 = globals
        .bind(&qh, 1..=1, ())
        .expect("zwp_input_method_manager_v2 not advertised");
    let _ime = ime_mgr.get_input_method(&seat, &qh, ());
    queue
        .roundtrip(&mut ime_state)
        .expect("ime first roundtrip");

    let probe = probe.canonicalize().expect("canonicalize gtk4 probe");
    let demo = probe.join("bin/gtk4-demo");
    let path = std::env::var("PATH").unwrap_or_default();
    let mut gtk = Command::new("qemu-aarch64-static")
        .arg("-L")
        .arg(&probe)
        .arg(&demo)
        .arg("--run=entry")
        .env_clear()
        .env("PATH", &path)
        .env("XDG_RUNTIME_DIR", runtime_dir.path())
        .env("WAYLAND_DISPLAY", &socket_name)
        .env("GDK_BACKEND", "wayland")
        .env("GSK_RENDERER", "cairo")
        .env("GTK_A11Y", "none")
        .env("NO_AT_BRIDGE", "1")
        .env("XKB_CONFIG_ROOT", probe.join("share/X11/xkb"))
        .env("GIO_USE_VFS", "local")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("qemu-aarch64-static failed to spawn gtk4-demo");
    let stderr_rx = {
        let stderr = gtk.stderr.take().expect("gtk4-demo stderr");
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            let mut buf = String::new();
            for line in reader.lines().map_while(Result::ok) {
                buf.push_str(&line);
                buf.push('\n');
            }
            let _ = tx.send(buf);
        });
        rx
    };

    let mut saw_toplevel = false;
    let mut saw_frame = false;
    let mut saw_get = false;
    let mut saw_enable = false;
    let mut saw_click = false;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline && !(saw_toplevel && saw_frame) {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("new xdg_toplevel") {
                    saw_toplevel = true;
                }
                if line.contains("frame sha256=") {
                    saw_frame = true;
                }
                if line.contains("text-input-v3 get") {
                    saw_get = true;
                }
                if line.contains("text-input-v3 enable") {
                    saw_enable = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }
    assert!(
        saw_toplevel && saw_frame,
        "gtk4-demo --run=entry never framed; displayd={:?}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );

    if let Some(stdin) = displayd.stdin.as_mut() {
        writeln!(stdin, "inject-click 640 400").expect("inject-click");
        let _ = stdin.flush();
    }
    let click_deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < click_deadline && !saw_click {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("injected click") {
                    saw_click = true;
                }
                if line.contains("text-input-v3 get") {
                    saw_get = true;
                }
                if line.contains("text-input-v3 enable") {
                    saw_enable = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }
    assert!(
        saw_click,
        "displayd never injected click 640 400; displayd={:?}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    let settle = Instant::now() + Duration::from_millis(700);
    while Instant::now() < settle {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("text-input-v3 get") {
                    saw_get = true;
                }
                if line.contains("text-input-v3 enable") {
                    saw_enable = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }

    let _ = gtk.kill();
    let _ = gtk.wait();
    let stderr = stderr_rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap_or_default();
    let _ = displayd.kill();
    let _ = displayd.wait();
    assert!(
        saw_get,
        "gtk4-demo never zwp_text_input_v3.get; displayd={:?}; gtk={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    assert!(
        !saw_enable,
        "gtk4-demo entry enabled v3 unexpectedly after 640 400; do not sweep Y; displayd={:?}; gtk={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
}
