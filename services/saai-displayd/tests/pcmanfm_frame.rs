//! APP-04 / ADR-323: packed aarch64 PCManFM-Qt (Qt 5.15) binds
//! `zwp_text_input_manager_v2` on host `saai-displayd` and `enable`s.
//! ADR-324: a separate IME client can `commit_string` while that
//! enable is live. Empty `QT_IM_MODULE` blocks the path. Not a panthe
//! typed field.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

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

#[test]
fn pcmanfm_qt_binds_text_input_v2_when_im_module_is_unset() {
    let pkg = pcmanfm_package();
    let bin = pkg.join("bin/pcmanfm-qt");
    assert!(
        bin.is_file(),
        "missing packed pcmanfm-qt at {} — set PCMANFM_PACKAGE_DIR",
        bin.display()
    );
    assert!(
        pkg.join("lib/ld-musl-aarch64.so.1").is_file(),
        "missing musl loader in {}",
        pkg.display()
    );

    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    let home_dir = tempfile::tempdir().expect("failed to create PCManFM HOME");
    let cfg = home_dir.path().join(".config/pcmanfm-qt/default");
    std::fs::create_dir_all(&cfg).expect("pcmanfm config dir");
    std::fs::write(
        cfg.join("settings.conf"),
        "[FolderView]\nShowFilter=true\n[Window]\nPathBarButtons=false\n",
    )
    .expect("write settings.conf");

    let (mut displayd, log) = spawn_displayd(runtime_dir.path());
    let socket_name = wait_for_socket(&log);
    let path = std::env::var("PATH").unwrap_or_default();
    let pkg = pkg.canonicalize().expect("canonicalize pcmanfm package");
    let bin = pkg.join("bin/pcmanfm-qt");

    let mut child = Command::new("dbus-run-session")
        .arg("--")
        .arg("qemu-aarch64-static")
        .arg("-L")
        .arg(&pkg)
        .arg(&bin)
        .env_remove("QT_IM_MODULE")
        .env("PATH", &path)
        .env("XDG_RUNTIME_DIR", runtime_dir.path())
        .env("WAYLAND_DISPLAY", &socket_name)
        .env("HOME", home_dir.path())
        .env("QT_PLUGIN_PATH", pkg.join("plugins"))
        .env("QT_QPA_PLATFORM", "wayland")
        .env("XKB_CONFIG_ROOT", pkg.join("share/X11/xkb"))
        .env("QT_QPA_PLATFORMTHEME", "")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("dbus-run-session/qemu failed to spawn pcmanfm-qt");
    let stderr_rx = {
        let stderr = child.stderr.take().expect("pcmanfm stderr");
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
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline && !(saw_toplevel && saw_frame && saw_get && saw_enable) {
        match log.recv_timeout(Duration::from_millis(200)) {
            Ok(line) => {
                if line.contains("new xdg_toplevel") {
                    assert!(
                        line.contains("1280x800 fullscreen=false"),
                        "PCManFM must map the x86 windowed size: {line}"
                    );
                    saw_toplevel = true;
                }
                if line.contains("frame sha256=") {
                    saw_frame = true;
                }
                if line.contains("text-input-v2 get") {
                    saw_get = true;
                }
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let _ = child.kill();
    let _ = child.wait();
    let stderr = stderr_rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap_or_default();
    assert!(
        saw_toplevel && saw_frame,
        "PCManFM never framed; displayd={lines:?}; stderr={stderr}"
    );
    assert!(
        saw_get,
        "Qt 5.15 never zwp_text_input_manager_v2.get_text_input; displayd={lines:?}; stderr={stderr}"
    );
    assert!(
        saw_enable,
        "Qt 5.15 never zwp_text_input_v2.enable; displayd={lines:?}; stderr={stderr}"
    );
    let _ = displayd.kill();
    let _ = displayd.wait();
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

fn frame_hash(line: &str) -> Option<&str> {
    line.split("frame sha256=")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
}

#[test]
fn osk_ime_commit_string_reaches_pcmanfm_v2() {
    let pkg = pcmanfm_package();
    let bin = pkg.join("bin/pcmanfm-qt");
    assert!(
        bin.is_file(),
        "missing packed pcmanfm-qt at {} — set PCMANFM_PACKAGE_DIR",
        bin.display()
    );

    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    let home_dir = tempfile::tempdir().expect("failed to create PCManFM HOME");
    let cfg = home_dir.path().join(".config/pcmanfm-qt/default");
    std::fs::create_dir_all(&cfg).expect("pcmanfm config dir");
    std::fs::write(
        cfg.join("settings.conf"),
        "[FolderView]\nShowFilter=true\n[Window]\nPathBarButtons=false\n",
    )
    .expect("write settings.conf");

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

    let path = std::env::var("PATH").unwrap_or_default();
    let pkg = pkg.canonicalize().expect("canonicalize pcmanfm package");
    let bin = pkg.join("bin/pcmanfm-qt");

    let mut child = Command::new("dbus-run-session")
        .arg("--")
        .arg("qemu-aarch64-static")
        .arg("-L")
        .arg(&pkg)
        .arg(&bin)
        .env_remove("QT_IM_MODULE")
        .env("PATH", &path)
        .env("XDG_RUNTIME_DIR", runtime_dir.path())
        .env("WAYLAND_DISPLAY", &socket_name)
        .env("HOME", home_dir.path())
        .env("QT_PLUGIN_PATH", pkg.join("plugins"))
        .env("QT_QPA_PLATFORM", "wayland")
        .env("XKB_CONFIG_ROOT", pkg.join("share/X11/xkb"))
        .env("QT_QPA_PLATFORMTHEME", "")
        .env("QT_LOGGING_RULES", "qt.qpa.input.methods.debug=true")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("dbus-run-session/qemu failed to spawn pcmanfm-qt");
    let _stderr_rx = {
        let stderr = child.stderr.take().expect("pcmanfm stderr");
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
    let mut saw_disable = false;
    let mut saw_update = false;
    let mut last_hash = None;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline && !(saw_enable && ime_state.activate) {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                }
                if line.contains("text-input-v2 disable") {
                    saw_disable = true;
                }
                if line.contains("text-input-v2 update_state") {
                    saw_update = true;
                }
                if let Some(h) = frame_hash(&line) {
                    last_hash = Some(h.to_string());
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }

    assert!(
        saw_enable,
        "Qt never enabled v2 before IME commit; displayd={lines:?}; disable={saw_disable}"
    );
    assert!(
        ime_state.activate,
        "IME never Activate after PCManFM enable; displayd={lines:?}; disable={saw_disable}"
    );

    ime.commit_string(String::from("hi!"));
    ime.commit(0);
    queue
        .roundtrip(&mut ime_state)
        .expect("ime commit roundtrip");
    let mut saw_commit = false;
    let commit_deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < commit_deadline && !saw_commit {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("text-input-v2 commit_string") {
                    saw_commit = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }
    assert!(
        saw_commit,
        "IME commit_string did not reach the v2 field before Ctrl+I; displayd={lines:?}"
    );

    // Ctrl+I is Show/Focus Filter Bar. After it, Qt disables v2
    // (ADR-326). Pointer clicks at 300,70 / 200,755 opened a popup
    // surface, not the QLineEdit.
    if let Some(stdin) = displayd.stdin.as_mut() {
        writeln!(stdin, "inject-ctrl-i").expect("inject-ctrl-i");
        let _ = stdin.flush();
    }
    let mut saw_ctrl_i = false;
    let mut saw_disable_after = false;
    let settle = Instant::now() + Duration::from_millis(900);
    while Instant::now() < settle {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("injected ctrl-i") {
                    saw_ctrl_i = true;
                }
                if saw_ctrl_i && line.contains("text-input-v2 disable") {
                    saw_disable_after = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }
    assert!(
        saw_ctrl_i,
        "displayd never injected Ctrl+I; displayd={lines:?}"
    );
    assert!(
        saw_disable_after,
        "Ctrl+I did not disable v2; displayd={lines:?}"
    );
    let _ = (saw_update, last_hash, saw_disable);

    let _ = child.kill();
    let _ = child.wait();
    let _ = displayd.kill();
    let _ = displayd.wait();
}
