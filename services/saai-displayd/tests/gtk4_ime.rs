//! APP-04 / ADR-325: host GTK 4.18 Entry with grab_focus receives OSK
//! IME `commit_string` as widget text. Packed musl GTK 4.14.4 Entry
//! under qemu is the panther toolkit (ADR-343). Without wl_keyboard,
//! packed 4.14 is ADR-346 and host 4.18 is ADR-350. gtk4-demo
//! `--run=entry` click without wl_keyboard is ADR-356. Packed 4.14
//! competing pane is ADR-372. `--run=entry` is not a gtk4-demo
//! example name (ADR-373). `--run=search_entry` is ADR-374.
//! `--run=password_entry` is ADR-382. OSK on search_entry is
//! ADR-375. v3 commit_string log is ADR-376. v3 object ids on
//! search_entry are ADR-377. v3 surrounding/done on that OSK are
//! ADR-378. shm commits after that OSK are ADR-379. Click then
//! OSK on that demo is ADR-380. v3 cursor rectangle is ADR-381.
//! password_entry OSK is ADR-383. Host frame clock lets search_entry
//! and password_entry OSK commit shm (ADR-388). gtk4-demo
//! `--list` has `entry_completion`/`combobox`, not popover/menu;
//! `--run=entry_completion` enables v3 without xdg_popup (ADR-396).
//! OSK into that demo types without xdg_popup (ADR-397).
//! `--run=combobox` maps without xdg_popup until a tap (ADR-399).
//! One click `200 90` still has no xdg_popup (ADR-400).
//! Packed GtkMenuButton popover without a tap is ADR-401.
//! Not a panther field.

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
    spawn_displayd_with(runtime_dir, &[])
}

fn spawn_displayd_with(
    runtime_dir: &std::path::Path,
    extra: &[(&str, &str)],
) -> (Child, mpsc::Receiver<String>) {
    let mut cmd = Command::new(displayd_bin());
    cmd.env("XDG_RUNTIME_DIR", runtime_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in extra {
        cmd.env(k, v);
    }
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

fn activated_surface_id(line: &str) -> Option<&str> {
    if !line.contains("xdg activated") {
        return None;
    }
    line.split("wl_surface@")
        .nth(1)?
        .split(|c: char| !c.is_ascii_digit())
        .next()
}

fn toplevel_frame_hash<'a>(line: &'a str, surface: &mut Option<String>) -> Option<&'a str> {
    if !line.contains("frame sha256=") {
        return None;
    }
    let id = line
        .split("wl_surface@")
        .nth(1)?
        .split(|c: char| !c.is_ascii_digit())
        .next()?;
    match surface {
        None => return None,
        Some(s) if s != id => return None,
        Some(_) => {}
    }
    line.split("frame sha256=")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
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

/// ADR-350: host GTK 4.18 Entry types OSK on a keyboard-less seat
/// (panther-like). Packed 4.14 already does (ADR-346). Not a panther field.
#[test]
fn osk_ime_types_hi_bang_into_gtk4_entry_without_seat_keyboard() {
    assert!(
        gtk4_import_available(),
        "host GTK4 probe needs python3 + gir1.2-gtk-4.0 (Gtk 4.0)"
    );
    let probe = gtk4_probe();
    assert!(probe.is_file(), "missing GTK4 probe at {}", probe.display());

    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    let (mut displayd, log) =
        spawn_displayd_with(runtime_dir.path(), &[("SAAIOS_SEAT_NO_KEYBOARD", "1")]);
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
    let gtk_err = {
        let stderr = gtk.stderr.take().expect("gtk stderr");
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

    let mut saw_enable = false;
    let mut saw_kbd_focus = false;
    let mut saw_focus = false;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline && !(saw_enable && ime_state.activate) {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
                } else if line.contains("focus set to") {
                    saw_focus = true;
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
    if !(saw_enable && ime_state.activate && saw_focus && !saw_kbd_focus) {
        let _ = gtk.kill();
        let _ = gtk.wait();
        let stderr = gtk_err
            .recv_timeout(Duration::from_secs(1))
            .unwrap_or_default();
        panic!(
            "host GTK 4.18 Entry on keyboard-less seat: enable={saw_enable} activate={} kbd={saw_kbd_focus} focus={saw_focus}; displayd={lines:?}; gtk={stderr}",
            ime_state.activate
        );
    }

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
    let stderr = gtk_err
        .recv_timeout(Duration::from_secs(1))
        .unwrap_or_default();
    assert_eq!(
        entry, "hi!",
        "OSK IME did not type hi! into host GTK 4.18 Entry without wl_keyboard; displayd={lines:?}; gtk={stderr}"
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

/// ADR-346: packed GTK 4.14 Entry types OSK on a host seat without
/// wl_keyboard (panther-like, ADR-012). Not a panther flash.
#[test]
fn osk_ime_types_hi_bang_into_alpine_gtk414_entry_without_seat_keyboard() {
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
    let (mut displayd, log) =
        spawn_displayd_with(runtime_dir.path(), &[("SAAIOS_SEAT_NO_KEYBOARD", "1")]);
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

    let mut saw_enable = false;
    let mut saw_kbd_focus = false;
    let mut saw_focus = false;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline && !(saw_enable && ime_state.activate) {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
                } else if line.contains("focus set to") {
                    saw_focus = true;
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
        saw_focus && !saw_kbd_focus,
        "expected compositor focus without wl_keyboard; kbd={saw_kbd_focus} focus={saw_focus}; displayd={lines:?}"
    );
    assert!(
        saw_enable && ime_state.activate,
        "GTK 4.14 Entry never enabled v3 on a keyboard-less seat; displayd={lines:?}"
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
        "OSK IME did not type hi! into GTK 4.14 Entry without wl_keyboard; displayd={lines:?}"
    );
    let _ = displayd.kill();
    let _ = displayd.wait();
}

fn interesting(line: &str) -> bool {
    line.contains("text-input-v3")
        || line.contains("injected click")
        || line.contains("new xdg_toplevel")
        || line.contains("frame sha256=")
        || line.contains("xdg activated")
        || line.contains("focus set to")
        || line.contains("host frame clock")
        || line.contains("xdg popup")
}

fn surrounding_bytes(line: &str) -> Option<usize> {
    line.split("text-input-v3 surrounding ")
        .nth(1)
        .and_then(|rest| rest.split("bytes=").nth(1))
        .and_then(|n| n.trim().parse().ok())
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

/// ADR-356: packed gtk4-demo `--run=entry` on a keyboard-less seat.
/// One center click `640 400` after xdg Activated binds v3 and does
/// not enable. Packed Entry types (ADR-346). Demo is not that field.
#[test]
fn packed_gtk4_demo_click_without_seat_keyboard_does_not_enable_v3() {
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
    let (mut displayd, log) =
        spawn_displayd_with(runtime_dir.path(), &[("SAAIOS_SEAT_NO_KEYBOARD", "1")]);
    let socket_name = wait_for_socket(&log);

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
    let mut saw_activated = false;
    let mut saw_kbd_focus = false;
    let mut saw_focus = false;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline && !(saw_toplevel && saw_frame && saw_activated && saw_focus) {
        match log.recv_timeout(Duration::from_millis(200)) {
            Ok(line) => {
                if line.contains("new xdg_toplevel") {
                    saw_toplevel = true;
                }
                if line.contains("frame sha256=") {
                    saw_frame = true;
                }
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
                } else if line.contains("focus set to") {
                    saw_focus = true;
                }
                if line.contains("xdg activated") {
                    saw_activated = true;
                }
                if line.contains("text-input-v3 get") {
                    saw_get = true;
                }
                if line.contains("text-input-v3 enable") {
                    saw_enable = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    let settle = Instant::now() + Duration::from_millis(700);
    while Instant::now() < settle {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
                } else if line.contains("focus set to") {
                    saw_focus = true;
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
    }

    if !(saw_toplevel && saw_frame && saw_activated && saw_focus && !saw_kbd_focus && !saw_enable) {
        let _ = gtk.kill();
        let _ = gtk.wait();
        let stderr = stderr_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap_or_default();
        let _ = displayd.kill();
        let _ = displayd.wait();
        panic!(
            "pre-click: expected framed gtk4-demo Activated without enable; kbd={saw_kbd_focus} focus={saw_focus} xdg={saw_activated} get={saw_get} enable={saw_enable}; displayd={:?}; gtk={stderr}",
            lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
        );
    }

    let mut saw_click = false;
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
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
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
    }
    let post = Instant::now() + Duration::from_millis(700);
    while Instant::now() < post {
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
    }

    let _ = gtk.kill();
    let _ = gtk.wait();
    let stderr = stderr_rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap_or_default();
    let _ = displayd.kill();
    let _ = displayd.wait();
    assert!(
        saw_click && !saw_kbd_focus,
        "center click 640 400 on keyboard-less seat; kbd={saw_kbd_focus} click={saw_click}; displayd={:?}; gtk={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    assert!(
        saw_get && !saw_enable,
        "gtk4-demo click 640 400 without wl_keyboard; get={saw_get} enable={saw_enable}; do not claim demo Entry typed; displayd={:?}; gtk={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
}

/// ADR-372: packed GTK 4.14 competing pane. Keyboard-less seat.
/// Entry in the tree auto-enables v3 without a tap. gtk4-demo does
/// not. Not a gtk4-demo click.
#[test]
fn osk_ime_types_hi_bang_into_alpine_gtk414_entry_competing_pane() {
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
    let (mut displayd, log) =
        spawn_displayd_with(runtime_dir.path(), &[("SAAIOS_SEAT_NO_KEYBOARD", "1")]);
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
        .env("GTK4_COMPETE", "1")
        .env("GDK_BACKEND", "wayland")
        .env("GSK_RENDERER", "cairo")
        .env("GTK_A11Y", "none")
        .env("NO_AT_BRIDGE", "1")
        .env("XKB_CONFIG_ROOT", probe.join("share/X11/xkb"))
        .env("GIO_USE_VFS", "local")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("qemu-aarch64-static failed to spawn gtk414 competing pane");
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
    let gtk_err = {
        let stderr = gtk.stderr.take().expect("gtk stderr");
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

    let mut saw_enable = false;
    let mut saw_activated = false;
    let mut saw_kbd_focus = false;
    let mut saw_focus = false;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline
        && !(saw_activated && saw_focus && saw_enable && ime_state.activate)
    {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
                } else if line.contains("focus set to") {
                    saw_focus = true;
                }
                if line.contains("xdg activated") {
                    saw_activated = true;
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
    if !(saw_activated && saw_focus && saw_enable && ime_state.activate && !saw_kbd_focus) {
        let _ = gtk.kill();
        let _ = gtk.wait();
        let stderr = gtk_err
            .recv_timeout(Duration::from_secs(1))
            .unwrap_or_default();
        panic!(
            "packed GTK competing pane never auto-enabled v3; enable={saw_enable} activate={} kbd={saw_kbd_focus} focus={saw_focus} xdg={saw_activated}; gtk4-demo class would be bind-without-enable; displayd={lines:?}; gtk={stderr}",
            ime_state.activate
        );
    }

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
    let stderr = gtk_err
        .recv_timeout(Duration::from_secs(1))
        .unwrap_or_default();
    assert_eq!(
        entry, "hi!",
        "OSK IME did not type hi! into packed GTK competing Entry without a tap; gtk4-demo still bind-without-enable; displayd={lines:?}; gtk={stderr}"
    );
    let _ = displayd.kill();
    let _ = displayd.wait();
}

/// ADR-373: packed gtk4-demo `--list` has `search_entry` /
/// `password_entry`, not `entry`. `--run=entry` never opens an
/// Entry demo window (main.c name match). ADR-344/356 clicked the
/// demo browser. Not a Y sweep.
#[test]
fn packed_gtk4_demo_run_entry_is_not_an_example_name() {
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
    let (mut displayd, log) =
        spawn_displayd_with(runtime_dir.path(), &[("SAAIOS_SEAT_NO_KEYBOARD", "1")]);
    let socket_name = wait_for_socket(&log);

    let probe = probe.canonicalize().expect("canonicalize gtk4 probe");
    let demo = probe.join("bin/gtk4-demo");
    let path = std::env::var("PATH").unwrap_or_default();
    let mut gtk = Command::new("qemu-aarch64-static")
        .arg("-L")
        .arg(&probe)
        .arg(&demo)
        .arg("--list")
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
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("qemu-aarch64-static failed to spawn gtk4-demo --list");
    let stdout_rx = {
        let stdout = gtk.stdout.take().expect("gtk4-demo stdout");
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            let mut buf = String::new();
            for line in reader.lines().map_while(Result::ok) {
                buf.push_str(&line);
                buf.push('\n');
            }
            let _ = tx.send(buf);
        });
        rx
    };
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

    let names = stdout_rx
        .recv_timeout(Duration::from_secs(20))
        .unwrap_or_default();
    let _ = gtk.wait();
    let stderr = stderr_rx
        .recv_timeout(Duration::from_secs(1))
        .unwrap_or_default();
    let _ = displayd.kill();
    let _ = displayd.wait();

    let listed: Vec<&str> = names.lines().filter(|l| !l.is_empty()).collect();
    assert!(
        listed.iter().any(|n| *n == "search_entry"),
        "gtk4-demo --list missing search_entry; names={listed:?}; stderr={stderr}"
    );
    assert!(
        listed.iter().any(|n| *n == "password_entry"),
        "gtk4-demo --list missing password_entry; names={listed:?}; stderr={stderr}"
    );
    assert!(
        listed.iter().all(|n| *n != "entry"),
        "gtk4-demo --list unexpectedly has entry; ADR-344/356 --run=entry was a real demo; names={listed:?}; stderr={stderr}"
    );
}

/// ADR-396: packed gtk4-demo `--list` has `entry_completion` /
/// `combobox` (dropdown chrome), not `popover`/`menu`. Not a click.
#[test]
fn packed_gtk4_demo_list_has_entry_completion() {
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
    let (mut displayd, log) =
        spawn_displayd_with(runtime_dir.path(), &[("SAAIOS_SEAT_NO_KEYBOARD", "1")]);
    let socket_name = wait_for_socket(&log);

    let probe = probe.canonicalize().expect("canonicalize gtk4 probe");
    let demo = probe.join("bin/gtk4-demo");
    let path = std::env::var("PATH").unwrap_or_default();
    let mut gtk = Command::new("qemu-aarch64-static")
        .arg("-L")
        .arg(&probe)
        .arg(&demo)
        .arg("--list")
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
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("qemu-aarch64-static failed to spawn gtk4-demo --list");
    let stdout_rx = {
        let stdout = gtk.stdout.take().expect("gtk4-demo stdout");
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            let mut buf = String::new();
            for line in reader.lines().map_while(Result::ok) {
                buf.push_str(&line);
                buf.push('\n');
            }
            let _ = tx.send(buf);
        });
        rx
    };
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
    let names = stdout_rx
        .recv_timeout(Duration::from_secs(20))
        .unwrap_or_default();
    let _ = gtk.wait();
    let stderr = stderr_rx
        .recv_timeout(Duration::from_secs(1))
        .unwrap_or_default();
    let _ = displayd.kill();
    let _ = displayd.wait();

    let listed: Vec<&str> = names.lines().filter(|l| !l.is_empty()).collect();
    assert!(
        listed.iter().any(|n| *n == "entry_completion"),
        "gtk4-demo --list missing entry_completion; names={listed:?}; stderr={stderr}"
    );
    assert!(
        listed.iter().any(|n| *n == "combobox"),
        "gtk4-demo --list missing combobox; names={listed:?}; stderr={stderr}"
    );
    assert!(
        listed.iter().all(|n| {
            let n = n.to_ascii_lowercase();
            !n.contains("popover") && n != "menus" && n != "menu"
        }),
        "gtk4-demo --list unexpectedly has popover/menu; names={listed:?}; stderr={stderr}"
    );
}

/// ADR-396: packed gtk4-demo `--run=entry_completion` enables v3
/// without a click and does not map xdg_popup until type. Not OSK
/// this slice. Not `--run=entry`.
#[test]
fn packed_gtk4_demo_entry_completion_enables_v3_without_popup() {
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
    let (mut displayd, log) =
        spawn_displayd_with(runtime_dir.path(), &[("SAAIOS_SEAT_NO_KEYBOARD", "1")]);
    let socket_name = wait_for_socket(&log);

    let probe = probe.canonicalize().expect("canonicalize gtk4 probe");
    let demo = probe.join("bin/gtk4-demo");
    let path = std::env::var("PATH").unwrap_or_default();
    let mut gtk = Command::new("qemu-aarch64-static")
        .arg("-L")
        .arg(&probe)
        .arg(&demo)
        .arg("--run=entry_completion")
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
        .expect("qemu-aarch64-static failed to spawn gtk4-demo --run=entry_completion");
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
    let mut saw_activated = false;
    let mut saw_focus = false;
    let mut saw_kbd_focus = false;
    let mut saw_enable = false;
    let mut saw_popup = false;
    let mut popup_failed = false;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(12);
    while Instant::now() < deadline
        && !(saw_toplevel && saw_frame && saw_activated && saw_focus && saw_enable)
    {
        match log.recv_timeout(Duration::from_millis(100)) {
            Ok(line) => {
                if line.contains("new xdg_toplevel") {
                    saw_toplevel = true;
                }
                if line.contains("frame sha256=") {
                    saw_frame = true;
                }
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
                } else if line.contains("focus set to") {
                    saw_focus = true;
                }
                if line.contains("xdg activated") {
                    saw_activated = true;
                }
                if line.contains("text-input-v3 enable") {
                    saw_enable = true;
                }
                if line.contains("xdg popup configure failed") {
                    popup_failed = true;
                    saw_popup = true;
                } else if line.contains("xdg popup") {
                    saw_popup = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    let settle = Instant::now() + Duration::from_millis(700);
    while Instant::now() < settle {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("text-input-v3 enable") {
                    saw_enable = true;
                }
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
                }
                if line.contains("xdg popup configure failed") {
                    popup_failed = true;
                    saw_popup = true;
                } else if line.contains("xdg popup") {
                    saw_popup = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let _ = gtk.kill();
    let _ = gtk.wait();
    let stderr = stderr_rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap_or_default();
    let _ = displayd.kill();
    let _ = displayd.wait();
    assert!(
        saw_toplevel && saw_frame && saw_activated && saw_focus && saw_enable && !saw_kbd_focus,
        "entry_completion demo did not enable v3; kbd={saw_kbd_focus} focus={saw_focus} xdg={saw_activated} enable={saw_enable}; displayd={:?}; gtk={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    assert!(
        !saw_popup && !popup_failed,
        "entry_completion mapped xdg_popup without type; popup={saw_popup} failed={popup_failed}; displayd={:?}; gtk={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
}

/// ADR-399: packed gtk4-demo `--run=combobox` on a keyboard-less seat,
/// no click. First assert: maps, no xdg_popup. Flip if the demo opens
/// a dropdown without a tap. Not `--run=entry`. Not Falkon.
#[test]
fn packed_gtk4_demo_combobox_without_click_has_no_popup() {
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
    let (mut displayd, log) =
        spawn_displayd_with(runtime_dir.path(), &[("SAAIOS_SEAT_NO_KEYBOARD", "1")]);
    let socket_name = wait_for_socket(&log);

    let probe = probe.canonicalize().expect("canonicalize gtk4 probe");
    let demo = probe.join("bin/gtk4-demo");
    let path = std::env::var("PATH").unwrap_or_default();
    let mut gtk = Command::new("qemu-aarch64-static")
        .arg("-L")
        .arg(&probe)
        .arg(&demo)
        .arg("--run=combobox")
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
        .expect("qemu-aarch64-static failed to spawn gtk4-demo --run=combobox");
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
    let mut saw_activated = false;
    let mut saw_focus = false;
    let mut saw_kbd_focus = false;
    let mut saw_enable = false;
    let mut saw_popup = false;
    let mut popup_failed = false;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(12);
    while Instant::now() < deadline && !(saw_toplevel && saw_frame && saw_activated && saw_focus) {
        match log.recv_timeout(Duration::from_millis(100)) {
            Ok(line) => {
                if line.contains("new xdg_toplevel") {
                    saw_toplevel = true;
                }
                if line.contains("frame sha256=") {
                    saw_frame = true;
                }
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
                } else if line.contains("focus set to") {
                    saw_focus = true;
                }
                if line.contains("xdg activated") {
                    saw_activated = true;
                }
                if line.contains("text-input-v3 enable") {
                    saw_enable = true;
                }
                if line.contains("xdg popup configure failed") {
                    popup_failed = true;
                    saw_popup = true;
                } else if line.contains("xdg popup") {
                    saw_popup = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    let settle = Instant::now() + Duration::from_secs(2);
    while Instant::now() < settle {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("text-input-v3 enable") {
                    saw_enable = true;
                }
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
                }
                if line.contains("xdg popup configure failed") {
                    popup_failed = true;
                    saw_popup = true;
                } else if line.contains("xdg popup") {
                    saw_popup = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let _ = gtk.kill();
    let _ = gtk.wait();
    let stderr = stderr_rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap_or_default();
    let _ = displayd.kill();
    let _ = displayd.wait();
    assert!(
        saw_toplevel && saw_frame && saw_activated && saw_focus && !saw_kbd_focus,
        "combobox demo did not map/activate; kbd={saw_kbd_focus} focus={saw_focus} xdg={saw_activated} frame={saw_frame}; displayd={:?}; gtk={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    assert!(
        !saw_popup && !popup_failed,
        "combobox mapped xdg_popup without a click; popup={saw_popup} failed={popup_failed} enable={saw_enable}; displayd={:?}; gtk={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
}

/// ADR-400: one combobox click `200 90`. Not a Y sweep. Click reaches
/// the compositor; no xdg_popup. Dropdown tap unproven.
#[test]
fn packed_gtk4_demo_combobox_click_200_90_has_no_popup() {
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
    let (mut displayd, log) =
        spawn_displayd_with(runtime_dir.path(), &[("SAAIOS_SEAT_NO_KEYBOARD", "1")]);
    let socket_name = wait_for_socket(&log);

    let probe = probe.canonicalize().expect("canonicalize gtk4 probe");
    let demo = probe.join("bin/gtk4-demo");
    let path = std::env::var("PATH").unwrap_or_default();
    let mut gtk = Command::new("qemu-aarch64-static")
        .arg("-L")
        .arg(&probe)
        .arg(&demo)
        .arg("--run=combobox")
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
        .expect("qemu-aarch64-static failed to spawn gtk4-demo --run=combobox");
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
    let mut saw_activated = false;
    let mut saw_focus = false;
    let mut saw_kbd_focus = false;
    let mut saw_click = false;
    let mut saw_popup = false;
    let mut popup_failed = false;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(12);
    while Instant::now() < deadline && !(saw_toplevel && saw_frame && saw_activated && saw_focus) {
        match log.recv_timeout(Duration::from_millis(100)) {
            Ok(line) => {
                if line.contains("new xdg_toplevel") {
                    saw_toplevel = true;
                }
                if line.contains("frame sha256=") {
                    saw_frame = true;
                }
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
                } else if line.contains("focus set to") {
                    saw_focus = true;
                }
                if line.contains("xdg activated") {
                    saw_activated = true;
                }
                if line.contains("xdg popup configure failed") {
                    popup_failed = true;
                    saw_popup = true;
                } else if line.contains("xdg popup") {
                    saw_popup = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    if !(saw_toplevel && saw_frame && saw_activated && saw_focus && !saw_kbd_focus) {
        let _ = gtk.kill();
        let _ = gtk.wait();
        let stderr = stderr_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap_or_default();
        let _ = displayd.kill();
        let _ = displayd.wait();
        panic!(
            "combobox demo did not map before click; kbd={saw_kbd_focus} focus={saw_focus} xdg={saw_activated}; displayd={:?}; gtk={stderr}",
            lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
        );
    }

    if let Some(stdin) = displayd.stdin.as_mut() {
        writeln!(stdin, "inject-click 200 90").expect("inject-click");
        let _ = stdin.flush();
    }
    let click_deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < click_deadline && !saw_popup {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("injected click") {
                    saw_click = true;
                }
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
                }
                if line.contains("xdg popup configure failed") {
                    popup_failed = true;
                    saw_popup = true;
                } else if line.contains("xdg popup") {
                    saw_popup = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let _ = gtk.kill();
    let _ = gtk.wait();
    let stderr = stderr_rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap_or_default();
    let _ = displayd.kill();
    let _ = displayd.wait();
    assert!(
        saw_click && !saw_kbd_focus,
        "combobox click 200 90; click={saw_click} kbd={saw_kbd_focus}; displayd={:?}; gtk={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    assert!(
        !saw_popup && !popup_failed,
        "combobox click 200 90 mapped xdg_popup; ADR-394 configure would be exercised; popup={saw_popup} failed={popup_failed}; displayd={:?}; gtk={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
}

fn packed_gtk414_popover() -> PathBuf {
    if let Ok(p) = std::env::var("PACKED_GTK414_POPOVER") {
        return PathBuf::from(p);
    }
    gtk4_alpine_probe().join("bin/gtk414-popover")
}

/// ADR-401: packed GTK 4.14 GtkMenuButton popover without a tap.
/// gtk_menu_button_popup after map. Not gtk4-demo. Not a Y sweep.
#[test]
fn packed_gtk414_popover_maps_xdg_popup_without_click() {
    let probe = gtk4_alpine_probe();
    let bin = packed_gtk414_popover();
    let loader = probe.join("lib/ld-musl-aarch64.so.1");
    let xkb = probe.join("share/X11/xkb");
    assert!(
        bin.is_file(),
        "missing Alpine GTK 4.14 popover at {} — set PACKED_GTK414_POPOVER",
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
    let (mut displayd, log) =
        spawn_displayd_with(runtime_dir.path(), &[("SAAIOS_SEAT_NO_KEYBOARD", "1")]);
    let socket_name = wait_for_socket(&log);

    let probe = probe.canonicalize().expect("canonicalize gtk4 probe");
    let bin = if bin.is_absolute() {
        bin
    } else {
        probe.join("bin/gtk414-popover")
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
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("qemu-aarch64-static failed to spawn gtk414-popover");
    let stderr_rx = {
        let stderr = gtk.stderr.take().expect("gtk414-popover stderr");
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
    let mut saw_activated = false;
    let mut saw_focus = false;
    let mut saw_kbd_focus = false;
    let mut saw_popup = false;
    let mut popup_failed = false;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(12);
    while Instant::now() < deadline
        && !(saw_toplevel && saw_frame && saw_activated && saw_focus && saw_popup)
    {
        match log.recv_timeout(Duration::from_millis(100)) {
            Ok(line) => {
                if line.contains("new xdg_toplevel") {
                    saw_toplevel = true;
                }
                if line.contains("frame sha256=") {
                    saw_frame = true;
                }
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
                } else if line.contains("focus set to") {
                    saw_focus = true;
                }
                if line.contains("xdg activated") {
                    saw_activated = true;
                }
                if line.contains("xdg popup configure failed") {
                    popup_failed = true;
                    saw_popup = true;
                } else if line.contains("xdg popup") {
                    saw_popup = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let _ = gtk.kill();
    let _ = gtk.wait();
    let stderr = stderr_rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap_or_default();
    let _ = displayd.kill();
    let _ = displayd.wait();
    assert!(
        saw_toplevel && saw_frame && saw_activated && saw_focus && !saw_kbd_focus,
        "popover probe did not map/activate; kbd={saw_kbd_focus} focus={saw_focus} xdg={saw_activated} frame={saw_frame}; displayd={:?}; gtk={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    assert!(
        saw_popup && !popup_failed,
        "popover probe never mapped a configured xdg_popup; popup={saw_popup} failed={popup_failed}; displayd={:?}; gtk={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
}

/// ADR-374: packed gtk4-demo `--run=search_entry` (a real `--list`
/// name). Keyboard-less seat, no click. Demo is a GtkEntry without
/// grab_focus plus a Find button. Not `--run=entry`. Not a Y sweep.
#[test]
fn packed_gtk4_demo_search_entry_enables_v3_without_click() {
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
    let (mut displayd, log) =
        spawn_displayd_with(runtime_dir.path(), &[("SAAIOS_SEAT_NO_KEYBOARD", "1")]);
    let socket_name = wait_for_socket(&log);

    let probe = probe.canonicalize().expect("canonicalize gtk4 probe");
    let demo = probe.join("bin/gtk4-demo");
    let path = std::env::var("PATH").unwrap_or_default();
    let mut gtk = Command::new("qemu-aarch64-static")
        .arg("-L")
        .arg(&probe)
        .arg(&demo)
        .arg("--run=search_entry")
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
        .expect("qemu-aarch64-static failed to spawn gtk4-demo --run=search_entry");
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
    let mut saw_activated = false;
    let mut saw_kbd_focus = false;
    let mut saw_focus = false;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline && !(saw_toplevel && saw_frame && saw_activated && saw_focus) {
        match log.recv_timeout(Duration::from_millis(200)) {
            Ok(line) => {
                if line.contains("new xdg_toplevel") {
                    saw_toplevel = true;
                }
                if line.contains("frame sha256=") {
                    saw_frame = true;
                }
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
                } else if line.contains("focus set to") {
                    saw_focus = true;
                }
                if line.contains("xdg activated") {
                    saw_activated = true;
                }
                if line.contains("text-input-v3 get") {
                    saw_get = true;
                }
                if line.contains("text-input-v3 enable") {
                    saw_enable = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
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
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let _ = gtk.kill();
    let _ = gtk.wait();
    let stderr = stderr_rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap_or_default();
    let _ = displayd.kill();
    let _ = displayd.wait();
    assert!(
        saw_toplevel && saw_frame && saw_activated && saw_focus && !saw_kbd_focus,
        "search_entry demo did not map/activate; kbd={saw_kbd_focus} focus={saw_focus} xdg={saw_activated} frame={saw_frame}; displayd={:?}; gtk={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    assert!(
        saw_enable,
        "gtk4-demo --run=search_entry never enabled v3 without a click; get={saw_get} enable={saw_enable}; packed Entry auto-enables, this demo has no grab_focus; displayd={:?}; gtk={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
}

/// ADR-382: packed gtk4-demo `--run=password_entry` (a real `--list`
/// name). Keyboard-less seat, no click. Not `--run=entry`. Not a Y
/// sweep. Not typed.
#[test]
fn packed_gtk4_demo_password_entry_enables_v3_without_click() {
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
    let (mut displayd, log) =
        spawn_displayd_with(runtime_dir.path(), &[("SAAIOS_SEAT_NO_KEYBOARD", "1")]);
    let socket_name = wait_for_socket(&log);

    let probe = probe.canonicalize().expect("canonicalize gtk4 probe");
    let demo = probe.join("bin/gtk4-demo");
    let path = std::env::var("PATH").unwrap_or_default();
    let mut gtk = Command::new("qemu-aarch64-static")
        .arg("-L")
        .arg(&probe)
        .arg(&demo)
        .arg("--run=password_entry")
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
        .expect("qemu-aarch64-static failed to spawn gtk4-demo --run=password_entry");
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
    let mut saw_activated = false;
    let mut saw_kbd_focus = false;
    let mut saw_focus = false;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline && !(saw_toplevel && saw_frame && saw_activated && saw_focus) {
        match log.recv_timeout(Duration::from_millis(200)) {
            Ok(line) => {
                if line.contains("new xdg_toplevel") {
                    saw_toplevel = true;
                }
                if line.contains("frame sha256=") {
                    saw_frame = true;
                }
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
                } else if line.contains("focus set to") {
                    saw_focus = true;
                }
                if line.contains("xdg activated") {
                    saw_activated = true;
                }
                if line.contains("text-input-v3 get") {
                    saw_get = true;
                }
                if line.contains("text-input-v3 enable") {
                    saw_enable = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
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
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let _ = gtk.kill();
    let _ = gtk.wait();
    let stderr = stderr_rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap_or_default();
    let _ = displayd.kill();
    let _ = displayd.wait();
    assert!(
        saw_toplevel && saw_frame && saw_activated && saw_focus && !saw_kbd_focus,
        "password_entry demo did not map/activate; kbd={saw_kbd_focus} focus={saw_focus} xdg={saw_activated} frame={saw_frame}; displayd={:?}; gtk={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    assert!(
        saw_enable,
        "gtk4-demo --run=password_entry never enabled v3 without a click; get={saw_get} enable={saw_enable}; displayd={:?}; gtk={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
}

/// ADR-375/376/377/378/379/388: OSK into packed gtk4-demo
/// `--run=search_entry`. One v3 object; commit_string target is that
/// enable. Host frame clock lets the demo commit a new shm after OSK
/// (surrounding `hi!` 3 bytes). Not a panther field.
#[test]
fn packed_gtk4_demo_search_entry_osk_commits_shm() {
    search_entry_osk_case("search_entry", None);
}

/// ADR-380/388: one inject-click 640 40 on packed gtk4-demo
/// `--run=search_entry` after v3 enable, then OSK. Surrounding
/// grows to 3. Toplevel shm changes. Not `--run=entry`.
#[test]
fn packed_gtk4_demo_search_entry_click_then_osk() {
    search_entry_osk_case("search_entry", Some((640, 40)));
}

/// ADR-383/388: OSK into packed gtk4-demo `--run=password_entry` after
/// v3 enable. Surrounding grows to 9 (bullet encoding). Toplevel shm
/// changes. Not a panther field.
#[test]
fn packed_gtk4_demo_password_entry_osk_commits_shm() {
    search_entry_osk_case("password_entry", None);
}

/// ADR-397: OSK into packed gtk4-demo `--run=entry_completion`.
/// Types `hi!` and commits shm. No xdg_popup. Not a click.
#[test]
fn packed_gtk4_demo_entry_completion_osk_types_without_popup() {
    search_entry_osk_case("entry_completion", None);
}

fn search_entry_osk_case(run: &str, click: Option<(i32, i32)>) {
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
    let (mut displayd, log) =
        spawn_displayd_with(runtime_dir.path(), &[("SAAIOS_SEAT_NO_KEYBOARD", "1")]);
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
    let demo = probe.join("bin/gtk4-demo");
    let path = std::env::var("PATH").unwrap_or_default();
    let mut gtk = Command::new("qemu-aarch64-static")
        .arg("-L")
        .arg(&probe)
        .arg(&demo)
        .arg(format!("--run={run}"))
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
        .unwrap_or_else(|e| {
            panic!("qemu-aarch64-static failed to spawn gtk4-demo --run={run}: {e}")
        });
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

    let mut saw_enable = false;
    let mut saw_activated = false;
    let mut saw_kbd_focus = false;
    let mut saw_focus = false;
    let mut activated: Option<String> = None;
    let mut hash_at_enable: Option<String> = None;
    let mut last_hash: Option<String> = None;
    let mut saw_v3_commit = false;
    let mut saw_v3_dropped = false;
    let mut surrounding_at_enable: Option<usize> = None;
    let mut surrounding_after_osk: Option<usize> = None;
    let mut n_commits_after_osk = 0;
    let mut saw_popup = false;
    let mut popup_failed = false;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline
        && !(saw_enable && ime_state.activate && hash_at_enable.is_some())
    {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
                } else if line.contains("focus set to") {
                    saw_focus = true;
                }
                if let Some(id) = activated_surface_id(&line) {
                    saw_activated = true;
                    activated = Some(id.to_string());
                }
                if line.contains("text-input-v3 enable") {
                    saw_enable = true;
                }
                if line.contains("xdg popup configure failed") {
                    popup_failed = true;
                    saw_popup = true;
                } else if line.contains("xdg popup") {
                    saw_popup = true;
                }
                if let Some(n) = surrounding_bytes(&line) {
                    surrounding_at_enable = Some(n);
                }
                if line.contains("text-input-v3 commit_string dropped") {
                    saw_v3_dropped = true;
                } else if line.contains("text-input-v3 commit_string") {
                    saw_v3_commit = true;
                }
                if let Some(h) = toplevel_frame_hash(&line, &mut activated) {
                    last_hash = Some(h.to_string());
                    if saw_enable {
                        hash_at_enable = Some(h.to_string());
                    }
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }
    if !(saw_enable && ime_state.activate && saw_activated && saw_focus && !saw_kbd_focus) {
        let _ = gtk.kill();
        let _ = gtk.wait();
        let stderr = stderr_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap_or_default();
        panic!(
            "gtk4-demo --run={run} OSK: enable never ready; enable={saw_enable} activate={} kbd={saw_kbd_focus} hash={hash_at_enable:?}; displayd={:?}; gtk={stderr}",
            ime_state.activate,
            lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
        );
    }
    let mut before = hash_at_enable
        .clone()
        .or(last_hash.clone())
        .expect("no shm on activated search_entry surface");

    if let Some((x, y)) = click {
        let mut saw_click = false;
        if let Some(stdin) = displayd.stdin.as_mut() {
            writeln!(stdin, "inject-click {x} {y}").expect("inject-click");
            let _ = stdin.flush();
        }
        let click_deadline = Instant::now() + Duration::from_millis(800);
        while Instant::now() < click_deadline {
            match log.recv_timeout(Duration::from_millis(50)) {
                Ok(line) => {
                    if line.contains("injected click") {
                        saw_click = true;
                    }
                    if let Some(n) = surrounding_bytes(&line) {
                        surrounding_at_enable = Some(n);
                    }
                    if let Some(h) = toplevel_frame_hash(&line, &mut activated) {
                        last_hash = Some(h.to_string());
                    }
                    lines.push(line);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
            let _ = queue.roundtrip(&mut ime_state);
        }
        if !saw_click {
            let _ = gtk.kill();
            let _ = gtk.wait();
            let stderr = stderr_rx
                .recv_timeout(Duration::from_secs(1))
                .unwrap_or_default();
            panic!(
                "search_entry click {x} {y} never reached compositor; displayd={:?}; gtk={stderr}",
                lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
            );
        }
        before = last_hash.clone().unwrap_or(before);
    }

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

    let osk_deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < osk_deadline {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("text-input-v3 commit_string dropped") {
                    saw_v3_dropped = true;
                } else if line.contains("text-input-v3 commit_string") {
                    saw_v3_commit = true;
                }
                if let Some(n) = surrounding_bytes(&line) {
                    surrounding_after_osk = Some(n);
                }
                if line.contains("xdg popup configure failed") {
                    popup_failed = true;
                    saw_popup = true;
                } else if line.contains("xdg popup") {
                    saw_popup = true;
                }
                if line.contains("commit on surface") {
                    n_commits_after_osk += 1;
                }
                if let Some(h) = toplevel_frame_hash(&line, &mut activated) {
                    last_hash = Some(h.to_string());
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
    let after = last_hash.as_deref().unwrap_or(before.as_str());
    let n_gets = lines
        .iter()
        .filter(|l| l.contains("text-input-v3 get"))
        .count();
    let enable_id = lines.iter().rev().find_map(|l| {
        l.split("text-input-v3 enable ")
            .nth(1)
            .map(|s| s.trim().to_string())
    });
    let commit_id = lines.iter().rev().find_map(|l| {
        l.split("text-input-v3 commit_string ")
            .nth(1)
            .filter(|s| !s.starts_with("dropped"))
            .map(|s| s.trim().to_string())
    });
    let n_done = lines
        .iter()
        .filter(|l| l.contains("text-input-v3 done") && !l.contains("dropped"))
        .count();
    let n_cursor = lines
        .iter()
        .filter(|l| l.contains("text-input-v3 cursor"))
        .count();
    let n_client_commit = lines
        .iter()
        .filter(|l| l.contains("text-input-v3 commit "))
        .count();
    assert!(
        saw_v3_commit && !saw_v3_dropped,
        "search_entry OSK v3 commit_string forwarded={saw_v3_commit} dropped={saw_v3_dropped}; displayd={:?}; gtk={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    assert_eq!(
        n_gets, 1,
        "search_entry v3 get count; expected one seat object; gets={n_gets} enable={enable_id:?} commit={commit_id:?}; displayd={:?}; gtk={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    assert_eq!(
        enable_id.as_deref(),
        commit_id.as_deref(),
        "search_entry OSK commit_string target != enable; enable={enable_id:?} commit={commit_id:?}; displayd={:?}; gtk={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    assert!(
        n_done >= 1,
        "search_entry OSK v3 done missing; done={n_done} client_commit={n_client_commit} surrounding_enable={surrounding_at_enable:?} surrounding_osk={surrounding_after_osk:?}; displayd={:?}; gtk={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    assert!(
        surrounding_after_osk.unwrap_or(0) > surrounding_at_enable.unwrap_or(0),
        "search_entry OSK surrounding did not grow; GTK did not report applied text; click={click:?} done={n_done} client_commit={n_client_commit} surrounding_enable={surrounding_at_enable:?} surrounding_osk={surrounding_after_osk:?}; displayd={:?}; gtk={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    assert!(
        n_cursor >= 1,
        "search_entry OSK v3 cursor rectangle missing; mapped widget unproven; click={click:?} cursor={n_cursor} surrounding_enable={surrounding_at_enable:?} surrounding_osk={surrounding_after_osk:?}; displayd={:?}; gtk={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    assert!(
        lines.iter().any(|l| l.contains("host frame clock")),
        "host frame clock never logged; gtk4-demo OSK paint depends on it; click={click:?} run={run}; displayd={:?}; gtk={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    if click.is_some() {
        assert_eq!(
            surrounding_after_osk,
            Some(3),
            "search_entry click then OSK surrounding; expected hi! 3 bytes; click={click:?} surrounding_enable={surrounding_at_enable:?} surrounding_osk={surrounding_after_osk:?}; displayd={:?}; gtk={stderr}",
            lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
        );
        assert!(
            n_commits_after_osk >= 1,
            "search_entry click then OSK shm commit count after OSK; expected toplevel redraw; commits={n_commits_after_osk} click={click:?} surrounding_enable={surrounding_at_enable:?} surrounding_osk={surrounding_after_osk:?}; displayd={:?}; gtk={stderr}",
            lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
        );
        assert_ne!(
            after, before.as_str(),
            "search_entry click then OSK did not attach a new toplevel shm; before={before} after={after}; click={click:?}; displayd={:?}; gtk={stderr}",
            lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
        );
    } else if run == "password_entry" {
        assert_eq!(
            surrounding_after_osk,
            Some(9),
            "gtk4-demo password_entry OSK surrounding; expected 9-byte bullets for hi!; surrounding_enable={surrounding_at_enable:?} surrounding_osk={surrounding_after_osk:?} commits={n_commits_after_osk}; displayd={:?}; gtk={stderr}",
            lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
        );
        assert!(
            n_commits_after_osk >= 1,
            "gtk4-demo password_entry OSK shm commit count after OSK; expected redraw; commits={n_commits_after_osk} surrounding_osk={surrounding_after_osk:?}; displayd={:?}; gtk={stderr}",
            lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
        );
        assert_ne!(
            after, before.as_str(),
            "gtk4-demo password_entry OSK did not attach a new shm; before={before} after={after}; displayd={:?}; gtk={stderr}",
            lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
        );
    } else if run == "entry_completion" {
        assert!(
            !saw_popup && !popup_failed,
            "entry_completion OSK mapped xdg_popup; ADR-394 configure would be exercised; popup={saw_popup} failed={popup_failed}; surrounding_osk={surrounding_after_osk:?}; displayd={:?}; gtk={stderr}",
            lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
        );
        assert_eq!(
            surrounding_after_osk,
            Some(3),
            "entry_completion OSK surrounding; expected hi! 3 bytes; surrounding_enable={surrounding_at_enable:?} surrounding_osk={surrounding_after_osk:?}; displayd={:?}; gtk={stderr}",
            lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
        );
        assert_ne!(
            after, before.as_str(),
            "entry_completion OSK did not attach a new shm; before={before} after={after}; displayd={:?}; gtk={stderr}",
            lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
        );
    } else {
        assert_eq!(
            surrounding_after_osk,
            Some(3),
            "search_entry OSK surrounding; expected hi! 3 bytes; surrounding_enable={surrounding_at_enable:?} surrounding_osk={surrounding_after_osk:?}; displayd={:?}; gtk={stderr}",
            lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
        );
        assert!(
            n_commits_after_osk >= 1,
            "search_entry OSK shm commit count after OSK; expected redraw; commits={n_commits_after_osk} surrounding_enable={surrounding_at_enable:?} surrounding_osk={surrounding_after_osk:?}; displayd={:?}; gtk={stderr}",
            lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
        );
        assert_ne!(
            after, before.as_str(),
            "gtk4-demo search_entry OSK did not attach a new shm; before={before} after={after}; displayd={:?}; gtk={stderr}",
            lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
        );
    }
}
