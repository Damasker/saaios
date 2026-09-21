//! APP-04: host Qt 5.15 QLineEdit with setFocus receives OSK IME
//! `commit_string` as widget text (ADR-328). Packed musl Qt 5.15.10
//! QLineEdit under qemu does the same (ADR-330). Packed musl Qt 6.6.3
//! from the Falkon package is ADR-336. Without wl_keyboard that same
//! probe binds v2 and does not enable (ADR-347). Packed musl Qt 5.15.10
//! on that same seat is the same class (ADR-348). Host glibc Qt 5.15
//! without wl_keyboard is ADR-349. A pointer click on that same seat
//! is ADR-351. Not a panther field.

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

fn qt5_src() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/qt5_lineedit.cpp")
}

fn qt5_available() -> bool {
    Command::new("pkg-config")
        .args(["--exists", "Qt5Widgets"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn compile_qt5_probe() -> PathBuf {
    let src = qt5_src();
    assert!(src.is_file(), "missing Qt5 probe at {}", src.display());
    let out = std::env::temp_dir().join("saaios-qt5-lineedit");
    let cflags = Command::new("pkg-config")
        .args(["--cflags", "--libs", "Qt5Widgets"])
        .output()
        .expect("pkg-config Qt5Widgets");
    assert!(
        cflags.status.success(),
        "pkg-config Qt5Widgets failed: {}",
        String::from_utf8_lossy(&cflags.stderr)
    );
    let extra = String::from_utf8(cflags.stdout).expect("pkg-config utf8");
    let mut cmd = Command::new("g++");
    cmd.arg("-O1")
        .arg("-fPIC")
        .arg("-std=c++17")
        .arg("-o")
        .arg(&out)
        .arg(&src);
    for tok in extra.split_whitespace() {
        cmd.arg(tok);
    }
    let status = cmd.status().expect("g++ spawn");
    assert!(status.success(), "g++ failed to build {}", src.display());
    out
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
fn osk_ime_types_hi_bang_into_qt5_lineedit() {
    assert!(
        qt5_available(),
        "host Qt5 probe needs g++ and qtbase5-dev (Qt5Widgets)"
    );
    let probe = compile_qt5_probe();

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

    let mut qt = Command::new(&probe)
        .env_remove("QT_IM_MODULE")
        .env_remove("DISPLAY")
        .env("XDG_RUNTIME_DIR", runtime_dir.path())
        .env("WAYLAND_DISPLAY", &socket_name)
        .env("QT_QPA_PLATFORM", "wayland")
        .env(
            "QT_LOGGING_RULES",
            "qt.qpa.wayland.textinput.debug=true;qt.qpa.input.methods.debug=true",
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn qt5_lineedit");
    let qt_out = {
        let stdout = qt.stdout.take().expect("qt stdout");
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
    let qt_err = {
        let stderr = qt.stderr.take().expect("qt stderr");
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
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline && !(saw_enable && ime_state.activate) {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }
    if !(saw_enable && ime_state.activate) {
        let _ = qt.kill();
        let _ = qt.wait();
        let stderr = qt_err
            .recv_timeout(Duration::from_secs(1))
            .unwrap_or_default();
        panic!(
            "Qt QLineEdit never enabled v2 / IME never Activate; displayd={lines:?}; qt={stderr}"
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
        match qt_out.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if let Some(text) = line.strip_prefix("QT_LINEEDIT_TEXT=") {
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

    let _ = qt.kill();
    let _ = qt.wait();
    let stderr = qt_err
        .recv_timeout(Duration::from_secs(1))
        .unwrap_or_default();
    assert_eq!(
        entry, "hi!",
        "OSK IME did not type hi! into Qt 5.15 QLineEdit; displayd={lines:?}; qt={stderr}"
    );
    let _ = displayd.kill();
    let _ = displayd.wait();
}

fn pcmanfm_package() -> PathBuf {
    if let Ok(p) = std::env::var("PCMANFM_PACKAGE_DIR") {
        return PathBuf::from(p);
    }
    let from_crate = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../dist/panther/packages/org.saaios.demo.pcmanfm");
    if from_crate.join("bin/pcmanfm-qt").is_file() {
        return from_crate;
    }
    PathBuf::from("/home/mike/worktrees/saaios-som/dist/panther/packages/org.saaios.demo.pcmanfm")
}

fn packed_qt5_probe() -> PathBuf {
    if let Ok(p) = std::env::var("PACKED_QT5_LINEEDIT") {
        return PathBuf::from(p);
    }
    PathBuf::from("/tmp/qt5-lineedit-aarch64")
}

#[test]
fn osk_ime_types_hi_bang_into_packed_qt5_lineedit() {
    let pkg = pcmanfm_package();
    let probe = packed_qt5_probe();
    assert!(
        pkg.join("lib/ld-musl-aarch64.so.1").is_file(),
        "missing packed musl loader in {} — set PCMANFM_PACKAGE_DIR",
        pkg.display()
    );
    assert!(
        probe.is_file(),
        "missing packed QLineEdit at {} — set PACKED_QT5_LINEEDIT",
        probe.display()
    );

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

    let pkg = pkg.canonicalize().expect("canonicalize pcmanfm package");
    let path = std::env::var("PATH").unwrap_or_default();
    let mut qt = Command::new("qemu-aarch64-static")
        .arg("-L")
        .arg(&pkg)
        .arg(&probe)
        .env_remove("QT_IM_MODULE")
        .env_remove("DISPLAY")
        .env("PATH", &path)
        .env("XDG_RUNTIME_DIR", runtime_dir.path())
        .env("WAYLAND_DISPLAY", &socket_name)
        .env("QT_QPA_PLATFORM", "wayland")
        .env("QT_PLUGIN_PATH", pkg.join("plugins"))
        .env("XKB_CONFIG_ROOT", pkg.join("share/X11/xkb"))
        .env("QT_QPA_PLATFORMTHEME", "")
        .env(
            "QT_LOGGING_RULES",
            "qt.qpa.wayland.textinput.debug=true;qt.qpa.input.methods.debug=true",
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("qemu-aarch64-static failed to spawn packed QLineEdit");
    let qt_out = {
        let stdout = qt.stdout.take().expect("qt stdout");
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
    let qt_err = {
        let stderr = qt.stderr.take().expect("qt stderr");
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
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline && !(saw_enable && ime_state.activate) {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }
    if !(saw_enable && ime_state.activate) {
        let _ = qt.kill();
        let _ = qt.wait();
        let stderr = qt_err
            .recv_timeout(Duration::from_secs(1))
            .unwrap_or_default();
        panic!(
            "packed QLineEdit never enabled v2 / IME never Activate; displayd={lines:?}; qt={stderr}"
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
        match qt_out.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if let Some(text) = line.strip_prefix("QT_LINEEDIT_TEXT=") {
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

    let _ = qt.kill();
    let _ = qt.wait();
    let stderr = qt_err
        .recv_timeout(Duration::from_secs(1))
        .unwrap_or_default();
    assert_eq!(
        entry, "hi!",
        "OSK IME did not type hi! into packed musl Qt 5.15 QLineEdit; displayd={lines:?}; qt={stderr}"
    );
    let _ = displayd.kill();
    let _ = displayd.wait();
}

fn falkon_package() -> PathBuf {
    if let Ok(p) = std::env::var("FALKON_PACKAGE_DIR") {
        return PathBuf::from(p);
    }
    let from_crate = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../dist/panther/packages/org.saaios.demo.falkon");
    if from_crate.join("bin/falkon").is_file() {
        return from_crate;
    }
    PathBuf::from("/tmp/saaios-b2/dist/panther/packages/org.saaios.demo.falkon")
}

fn packed_qt6_probe() -> PathBuf {
    if let Ok(p) = std::env::var("PACKED_QT6_LINEEDIT") {
        return PathBuf::from(p);
    }
    PathBuf::from("/tmp/qt6-lineedit-aarch64")
}

#[test]
fn osk_ime_types_hi_bang_into_packed_qt6_lineedit() {
    let pkg = falkon_package();
    let probe = packed_qt6_probe();
    assert!(
        pkg.join("lib/ld-musl-aarch64.so.1").is_file(),
        "missing packed musl loader in {} — set FALKON_PACKAGE_DIR",
        pkg.display()
    );
    assert!(
        probe.is_file(),
        "missing packed Qt6 QLineEdit at {} — set PACKED_QT6_LINEEDIT",
        probe.display()
    );

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

    let pkg = pkg.canonicalize().expect("canonicalize falkon package");
    let path = std::env::var("PATH").unwrap_or_default();
    let mut qt = Command::new("qemu-aarch64-static")
        .arg("-L")
        .arg(&pkg)
        .arg(&probe)
        .env_remove("QT_IM_MODULE")
        .env_remove("DISPLAY")
        .env("PATH", &path)
        .env("XDG_RUNTIME_DIR", runtime_dir.path())
        .env("WAYLAND_DISPLAY", &socket_name)
        .env("QT_QPA_PLATFORM", "wayland")
        .env("QT_PLUGIN_PATH", pkg.join("plugins"))
        .env("XKB_CONFIG_ROOT", pkg.join("share/X11/xkb"))
        .env("QT_QPA_PLATFORMTHEME", "")
        .env(
            "QT_LOGGING_RULES",
            "qt.qpa.wayland.textinput.debug=true;qt.qpa.input.methods.debug=true",
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("qemu-aarch64-static failed to spawn packed Qt6 QLineEdit");
    let qt_out = {
        let stdout = qt.stdout.take().expect("qt stdout");
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
    let qt_err = {
        let stderr = qt.stderr.take().expect("qt stderr");
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
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline && !(saw_enable && ime_state.activate) {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }
    if !(saw_enable && ime_state.activate) {
        let _ = qt.kill();
        let _ = qt.wait();
        let stderr = qt_err
            .recv_timeout(Duration::from_secs(1))
            .unwrap_or_default();
        panic!(
            "packed Qt6 QLineEdit never enabled v2 / IME never Activate; displayd={lines:?}; qt={stderr}"
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
        match qt_out.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if let Some(text) = line.strip_prefix("QT_LINEEDIT_TEXT=") {
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

    let _ = qt.kill();
    let _ = qt.wait();
    let stderr = qt_err
        .recv_timeout(Duration::from_secs(1))
        .unwrap_or_default();
    assert_eq!(
        entry, "hi!",
        "OSK IME did not type hi! into packed musl Qt 6.6.3 QLineEdit; displayd={lines:?}; qt={stderr}"
    );
    let _ = displayd.kill();
    let _ = displayd.wait();
}

/// ADR-347: packed musl Qt 6.6.3 QLineEdit with setFocus binds v2 on a
/// keyboard-less seat and does not enable. GTK 4.14 still types
/// (ADR-346). Not a panther field.
#[test]
fn packed_qt6_lineedit_binds_v2_without_seat_keyboard_and_does_not_enable() {
    let pkg = falkon_package();
    let probe = packed_qt6_probe();
    assert!(
        pkg.join("lib/ld-musl-aarch64.so.1").is_file(),
        "missing packed musl loader in {} — set FALKON_PACKAGE_DIR",
        pkg.display()
    );
    assert!(
        probe.is_file(),
        "missing packed Qt6 QLineEdit at {} — set PACKED_QT6_LINEEDIT",
        probe.display()
    );

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
    let _ime = ime_mgr.get_input_method(&seat, &qh, ());
    queue
        .roundtrip(&mut ime_state)
        .expect("ime first roundtrip");

    let pkg = pkg.canonicalize().expect("canonicalize falkon package");
    let path = std::env::var("PATH").unwrap_or_default();
    let mut qt = Command::new("qemu-aarch64-static")
        .arg("-L")
        .arg(&pkg)
        .arg(&probe)
        .env_remove("QT_IM_MODULE")
        .env_remove("DISPLAY")
        .env("PATH", &path)
        .env("XDG_RUNTIME_DIR", runtime_dir.path())
        .env("WAYLAND_DISPLAY", &socket_name)
        .env("QT_QPA_PLATFORM", "wayland")
        .env("QT_PLUGIN_PATH", pkg.join("plugins"))
        .env("XKB_CONFIG_ROOT", pkg.join("share/X11/xkb"))
        .env("QT_QPA_PLATFORMTHEME", "")
        .env(
            "QT_LOGGING_RULES",
            "qt.qpa.wayland.textinput.debug=true;qt.qpa.input.methods.debug=true",
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("qemu-aarch64-static failed to spawn packed Qt6 QLineEdit");
    let _qt_out = {
        let stdout = qt.stdout.take().expect("qt stdout");
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
    let qt_err = {
        let stderr = qt.stderr.take().expect("qt stderr");
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
    let mut saw_get = false;
    let mut saw_kbd_focus = false;
    let mut saw_focus = false;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline && !saw_focus {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
                } else if line.contains("focus set to") {
                    saw_focus = true;
                }
                if line.contains("text-input-v2 get") {
                    saw_get = true;
                }
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
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
                if line.contains("text-input-v2 get") {
                    saw_get = true;
                }
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }

    let _ = qt.kill();
    let _ = qt.wait();
    let stderr = qt_err
        .recv_timeout(Duration::from_secs(1))
        .unwrap_or_default();
    let _ = displayd.kill();
    let _ = displayd.wait();
    assert!(
        saw_focus && !saw_kbd_focus,
        "expected compositor focus without wl_keyboard; kbd={saw_kbd_focus} focus={saw_focus}; displayd={lines:?}; qt={stderr}"
    );
    assert!(
        saw_get,
        "packed Qt6 QLineEdit never zwp_text_input_v2.get; displayd={lines:?}; qt={stderr}"
    );
    assert!(
        !saw_enable && !ime_state.activate,
        "packed Qt6 QLineEdit enabled v2 without wl_keyboard; compositor IME is not the gap; displayd={lines:?}; qt={stderr}"
    );
}

/// ADR-348: packed musl Qt 5.15.10 QLineEdit with setFocus binds v2 on a
/// keyboard-less seat. Expect no enable, same class as ADR-347.
#[test]
fn packed_qt5_lineedit_binds_v2_without_seat_keyboard_and_does_not_enable() {
    let pkg = pcmanfm_package();
    let probe = packed_qt5_probe();
    assert!(
        pkg.join("lib/ld-musl-aarch64.so.1").is_file(),
        "missing packed musl loader in {} — set PCMANFM_PACKAGE_DIR",
        pkg.display()
    );
    assert!(
        probe.is_file(),
        "missing packed QLineEdit at {} — set PACKED_QT5_LINEEDIT",
        probe.display()
    );

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
    let _ime = ime_mgr.get_input_method(&seat, &qh, ());
    queue
        .roundtrip(&mut ime_state)
        .expect("ime first roundtrip");

    let pkg = pkg.canonicalize().expect("canonicalize pcmanfm package");
    let path = std::env::var("PATH").unwrap_or_default();
    let mut qt = Command::new("qemu-aarch64-static")
        .arg("-L")
        .arg(&pkg)
        .arg(&probe)
        .env_remove("QT_IM_MODULE")
        .env_remove("DISPLAY")
        .env("PATH", &path)
        .env("XDG_RUNTIME_DIR", runtime_dir.path())
        .env("WAYLAND_DISPLAY", &socket_name)
        .env("QT_QPA_PLATFORM", "wayland")
        .env("QT_PLUGIN_PATH", pkg.join("plugins"))
        .env("XKB_CONFIG_ROOT", pkg.join("share/X11/xkb"))
        .env("QT_QPA_PLATFORMTHEME", "")
        .env(
            "QT_LOGGING_RULES",
            "qt.qpa.wayland.textinput.debug=true;qt.qpa.input.methods.debug=true",
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("qemu-aarch64-static failed to spawn packed Qt5 QLineEdit");
    let _qt_out = {
        let stdout = qt.stdout.take().expect("qt stdout");
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
    let qt_err = {
        let stderr = qt.stderr.take().expect("qt stderr");
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
    let mut saw_get = false;
    let mut saw_kbd_focus = false;
    let mut saw_focus = false;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline && !saw_focus {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
                } else if line.contains("focus set to") {
                    saw_focus = true;
                }
                if line.contains("text-input-v2 get") {
                    saw_get = true;
                }
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
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
                if line.contains("text-input-v2 get") {
                    saw_get = true;
                }
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }

    let _ = qt.kill();
    let _ = qt.wait();
    let stderr = qt_err
        .recv_timeout(Duration::from_secs(1))
        .unwrap_or_default();
    let _ = displayd.kill();
    let _ = displayd.wait();
    assert!(
        saw_focus && !saw_kbd_focus,
        "expected compositor focus without wl_keyboard; kbd={saw_kbd_focus} focus={saw_focus}; displayd={lines:?}; qt={stderr}"
    );
    assert!(
        saw_get,
        "packed Qt5 QLineEdit never zwp_text_input_v2.get; displayd={lines:?}; qt={stderr}"
    );
    assert!(
        !saw_enable && !ime_state.activate,
        "packed Qt5 QLineEdit enabled v2 without wl_keyboard; not the same class as ADR-347; displayd={lines:?}; qt={stderr}"
    );
}

/// ADR-349: host glibc Qt 5.15 QLineEdit with setFocus binds v2 on a
/// keyboard-less seat. Expect no enable, same class as ADR-347/348.
#[test]
fn host_qt5_lineedit_binds_v2_without_seat_keyboard_and_does_not_enable() {
    assert!(
        qt5_available(),
        "host Qt5 probe needs g++ and qtbase5-dev (Qt5Widgets)"
    );
    let probe = compile_qt5_probe();

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
    let _ime = ime_mgr.get_input_method(&seat, &qh, ());
    queue
        .roundtrip(&mut ime_state)
        .expect("ime first roundtrip");

    let mut qt = Command::new(&probe)
        .env_remove("QT_IM_MODULE")
        .env_remove("DISPLAY")
        .env("XDG_RUNTIME_DIR", runtime_dir.path())
        .env("WAYLAND_DISPLAY", &socket_name)
        .env("QT_QPA_PLATFORM", "wayland")
        .env(
            "QT_LOGGING_RULES",
            "qt.qpa.wayland.textinput.debug=true;qt.qpa.input.methods.debug=true",
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn qt5_lineedit");
    let _qt_out = {
        let stdout = qt.stdout.take().expect("qt stdout");
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
    let qt_err = {
        let stderr = qt.stderr.take().expect("qt stderr");
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
    let mut saw_get = false;
    let mut saw_kbd_focus = false;
    let mut saw_focus = false;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline && !saw_focus {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
                } else if line.contains("focus set to") {
                    saw_focus = true;
                }
                if line.contains("text-input-v2 get") {
                    saw_get = true;
                }
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
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
                if line.contains("text-input-v2 get") {
                    saw_get = true;
                }
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }

    let _ = qt.kill();
    let _ = qt.wait();
    let stderr = qt_err
        .recv_timeout(Duration::from_secs(1))
        .unwrap_or_default();
    let _ = displayd.kill();
    let _ = displayd.wait();
    assert!(
        saw_focus && !saw_kbd_focus,
        "expected compositor focus without wl_keyboard; kbd={saw_kbd_focus} focus={saw_focus}; displayd={lines:?}; qt={stderr}"
    );
    assert!(
        saw_get,
        "host Qt5 QLineEdit never zwp_text_input_v2.get; displayd={lines:?}; qt={stderr}"
    );
    assert!(
        !saw_enable && !ime_state.activate,
        "host Qt5 QLineEdit enabled v2 without wl_keyboard; not the same class as packed Qt; displayd={lines:?}; qt={stderr}"
    );
}

/// ADR-351: host glibc Qt 5.15 QLineEdit on a keyboard-less seat.
/// setFocus does not enable v2 (ADR-349). One pointer click `160 20`
/// (widget origin, not a Y sweep) is the panther-class activation.
#[test]
fn host_qt5_lineedit_click_without_seat_keyboard_does_not_enable_v2() {
    assert!(
        qt5_available(),
        "host Qt5 probe needs g++ and qtbase5-dev (Qt5Widgets)"
    );
    let probe = compile_qt5_probe();

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
    let _ime = ime_mgr.get_input_method(&seat, &qh, ());
    queue
        .roundtrip(&mut ime_state)
        .expect("ime first roundtrip");

    let mut qt = Command::new(&probe)
        .env_remove("QT_IM_MODULE")
        .env_remove("DISPLAY")
        .env("XDG_RUNTIME_DIR", runtime_dir.path())
        .env("WAYLAND_DISPLAY", &socket_name)
        .env("QT_QPA_PLATFORM", "wayland")
        .env("QT_LOGGING_TO_CONSOLE", "1")
        .env("QT_ASSUME_STDERR_HAS_CONSOLE", "1")
        .env(
            "QT_LOGGING_RULES",
            "qt.qpa.wayland.textinput.debug=true;qt.qpa.input.methods.debug=true",
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn qt5_lineedit");
    let _qt_out = {
        let stdout = qt.stdout.take().expect("qt stdout");
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
    let qt_err = {
        let stderr = qt.stderr.take().expect("qt stderr");
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
    let mut saw_get = false;
    let mut saw_kbd_focus = false;
    let mut saw_focus = false;
    let mut saw_click = false;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline && !saw_focus {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
                } else if line.contains("focus set to") {
                    saw_focus = true;
                }
                if line.contains("text-input-v2 get") {
                    saw_get = true;
                }
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }
    let settle = Instant::now() + Duration::from_millis(700);
    while Instant::now() < settle {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("text-input-v2 get") {
                    saw_get = true;
                }
                if line.contains("text-input-v2 enable") {
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
        saw_focus && !saw_kbd_focus && saw_get && !saw_enable,
        "pre-click: expected v2 get without enable; kbd={saw_kbd_focus} focus={saw_focus} get={saw_get} enable={saw_enable}; displayd={lines:?}"
    );

    if let Some(stdin) = displayd.stdin.as_mut() {
        writeln!(stdin, "inject-click 160 20").expect("inject-click");
        let _ = stdin.flush();
    }
    let click_deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < click_deadline && !saw_click {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("injected click") {
                    saw_click = true;
                }
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }
    let settle = Instant::now() + Duration::from_millis(700);
    while Instant::now() < settle {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }

    let _ = qt.kill();
    let _ = qt.wait();
    let stderr = qt_err
        .recv_timeout(Duration::from_secs(1))
        .unwrap_or_default();
    let _ = displayd.kill();
    let _ = displayd.wait();
    assert!(
        saw_click,
        "displayd never injected click 160 20; displayd={lines:?}; qt={stderr}"
    );
    assert!(
        !saw_enable && !ime_state.activate,
        "host Qt5 QLineEdit enabled v2 after pointer click without wl_keyboard; compositor IME is not the remaining gap; displayd={lines:?}; qt={stderr}"
    );
}
