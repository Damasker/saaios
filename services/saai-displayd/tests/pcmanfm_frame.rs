//! APP-04 / ADR-323: packed aarch64 PCManFM-Qt (Qt 5.15) binds
//! `zwp_text_input_manager_v2` on host `saai-displayd` and `enable`s.
//! ADR-324: IME `commit_string` while that enable is live. ADR-332:
//! Ctrl+L (PathEdit QShortcut) disables v2 with no second enable.
//! ADR-341: Qt 5.15 IME debug has no `discard commit_string` (same
//! silent `focusObject() == null` class as Falkon ADR-340).
//! ADR-353: without wl_keyboard, xdg Activated still enables chrome v2.
//! ADR-357: that enable stays through a 2 s quiet (not Falkon ADR-337).
//! ADR-363: OSK into that live enable without wl_keyboard hits
//! FolderViewListView, not Filter. ADR-364: Filter-band click on that
//! seat. ADR-365: PathEdit-band click. Empty `QT_IM_MODULE` blocks the path.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

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

fn reap_pcmanfm(child: &mut Child) {
    #[cfg(unix)]
    {
        let pid = child.id();
        let _ = Command::new("kill")
            .args(["-KILL", &format!("-{pid}")])
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
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
        .env("XDG_CONFIG_HOME", home_dir.path().join(".config"))
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
        None => *surface = Some(id.to_string()),
        Some(s) if s != id => return None,
        Some(_) => {}
    }
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
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(runtime_dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("XDG_RUNTIME_DIR 0700");
    }
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
        .env("XDG_CONFIG_HOME", home_dir.path().join(".config"))
        .env("QT_PLUGIN_PATH", pkg.join("plugins"))
        .env("QT_QPA_PLATFORM", "wayland")
        .env("XKB_CONFIG_ROOT", pkg.join("share/X11/xkb"))
        .env("QT_QPA_PLATFORMTHEME", "")
        .env("QT_LOGGING_TO_CONSOLE", "1")
        .env("QT_ASSUME_STDERR_HAS_CONSOLE", "1")
        .env(
            "QT_LOGGING_RULES",
            "qt.qpa.wayland.textinput.debug=true;qt.qpa.input.methods.debug=true",
        )
        .process_group(0)
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
        "IME commit_string did not reach the v2 field before Ctrl+L; displayd={lines:?}"
    );

    // Ctrl+L is QShortcut → PathEdit::setFocus+selectAll. With Ctrl held,
    // Qt disables v2 and does not enable again after release (ADR-332).
    if let Some(stdin) = displayd.stdin.as_mut() {
        writeln!(stdin, "inject-ctrl-l").expect("inject-ctrl-l");
        let _ = stdin.flush();
    }
    let mut saw_ctrl_l = false;
    let mut saw_disable_after = false;
    let mut saw_enable_after = false;
    let ctrl_deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < ctrl_deadline && !saw_ctrl_l {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("injected ctrl-l") {
                    saw_ctrl_l = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }
    assert!(
        saw_ctrl_l,
        "displayd never injected Ctrl+L; displayd={lines:?}"
    );
    let settle = Instant::now() + Duration::from_millis(700);
    while Instant::now() < settle {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("text-input-v2 disable") {
                    saw_disable_after = true;
                }
                if saw_disable_after && line.contains("text-input-v2 enable") {
                    saw_enable_after = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }
    reap_pcmanfm(&mut child);
    let stderr = _stderr_rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap_or_default();
    assert!(
        saw_disable_after,
        "Ctrl+L did not disable v2; displayd={lines:?}; qt={stderr}"
    );
    assert!(
        !saw_enable_after,
        "Ctrl+L re-enabled v2 unexpectedly; displayd={lines:?}; qt={stderr}"
    );
    assert!(
        stderr.contains("qt.qpa.input.methods"),
        "PCManFM OSK spawn must carry QT_LOGGING_RULES; qt={stderr}"
    );
    assert!(
        !stderr.contains("discard commit_string"),
        "Qt discarded commit_string despite ADR-339 defer; qt={stderr}"
    );

    let _ = displayd.kill();
    let _ = displayd.wait();
}

/// ADR-353: packed PCManFM on a keyboard-less seat gets xdg Activated
/// (ADR-352) and enables v2. Not a typed PathEdit/Filter. Not a panther
/// field. Do not click. Do not Ctrl+L.
#[test]
fn packed_pcmanfm_enables_v2_without_seat_keyboard() {
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
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(runtime_dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("XDG_RUNTIME_DIR 0700");
    }
    let home_dir = tempfile::tempdir().expect("failed to create PCManFM HOME");
    let cfg = home_dir.path().join(".config/pcmanfm-qt/default");
    std::fs::create_dir_all(&cfg).expect("pcmanfm config dir");
    std::fs::write(
        cfg.join("settings.conf"),
        "[FolderView]\nShowFilter=true\n[Window]\nPathBarButtons=false\n",
    )
    .expect("write settings.conf");

    let (mut displayd, log) =
        spawn_displayd_with(runtime_dir.path(), &[("SAAIOS_SEAT_NO_KEYBOARD", "1")]);
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
        .env("XDG_CONFIG_HOME", home_dir.path().join(".config"))
        .env("QT_PLUGIN_PATH", pkg.join("plugins"))
        .env("QT_QPA_PLATFORM", "wayland")
        .env("XKB_CONFIG_ROOT", pkg.join("share/X11/xkb"))
        .env("QT_QPA_PLATFORMTHEME", "")
        .process_group(0)
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
    let mut saw_activated = false;
    let mut saw_kbd_focus = false;
    let mut saw_focus = false;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline && !(saw_toplevel && saw_frame && saw_enable) {
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

    reap_pcmanfm(&mut child);
    let stderr = stderr_rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap_or_default();
    let _ = displayd.kill();
    let _ = displayd.wait();
    assert!(
        saw_toplevel && saw_frame && saw_activated && saw_focus && !saw_kbd_focus,
        "expected framed PCManFM with xdg Activated and no wl_keyboard; kbd={saw_kbd_focus} focus={saw_focus} xdg={saw_activated}; displayd={lines:?}; stderr={stderr}"
    );
    assert!(
        saw_get,
        "PCManFM never zwp_text_input_v2.get; displayd={lines:?}; stderr={stderr}"
    );
    assert!(
        saw_enable,
        "PCManFM never enabled v2 on a keyboard-less seat after xdg Activated; displayd={lines:?}; stderr={stderr}"
    );
}

/// ADR-357: packed PCManFM v2 enable on a keyboard-less seat stays
/// through 2 s quiet. Not Falkon URL disable (ADR-337). No click.
/// No OSK. Not typed PathEdit/Filter.
#[test]
fn packed_pcmanfm_v2_stays_enabled_without_seat_keyboard() {
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
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(runtime_dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("XDG_RUNTIME_DIR 0700");
    }
    let home_dir = tempfile::tempdir().expect("failed to create PCManFM HOME");
    let cfg = home_dir.path().join(".config/pcmanfm-qt/default");
    std::fs::create_dir_all(&cfg).expect("pcmanfm config dir");
    std::fs::write(
        cfg.join("settings.conf"),
        "[FolderView]\nShowFilter=true\n[Window]\nPathBarButtons=false\n",
    )
    .expect("write settings.conf");

    let (mut displayd, log) =
        spawn_displayd_with(runtime_dir.path(), &[("SAAIOS_SEAT_NO_KEYBOARD", "1")]);
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
        .env("XDG_CONFIG_HOME", home_dir.path().join(".config"))
        .env("QT_PLUGIN_PATH", pkg.join("plugins"))
        .env("QT_QPA_PLATFORM", "wayland")
        .env("XKB_CONFIG_ROOT", pkg.join("share/X11/xkb"))
        .env("QT_QPA_PLATFORMTHEME", "")
        .process_group(0)
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
    let mut saw_enable = false;
    let mut saw_disable = false;
    let mut saw_activated = false;
    let mut saw_kbd_focus = false;
    let mut saw_focus = false;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline && !saw_enable {
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
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                }
                if line.contains("text-input-v2 disable") {
                    saw_disable = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    if !saw_enable {
        reap_pcmanfm(&mut child);
        let stderr = stderr_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap_or_default();
        let _ = displayd.kill();
        let _ = displayd.wait();
        panic!(
            "PCManFM never enabled v2 before quiet; kbd={saw_kbd_focus} xdg={saw_activated}; displayd={lines:?}; stderr={stderr}"
        );
    }
    let quiet = Instant::now() + Duration::from_secs(2);
    while Instant::now() < quiet {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("text-input-v2 disable") {
                    saw_disable = true;
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

    reap_pcmanfm(&mut child);
    let stderr = stderr_rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap_or_default();
    let _ = displayd.kill();
    let _ = displayd.wait();
    assert!(
        saw_toplevel && saw_frame && saw_activated && saw_focus && !saw_kbd_focus,
        "expected framed PCManFM Activated without wl_keyboard; kbd={saw_kbd_focus} focus={saw_focus} xdg={saw_activated}; displayd={lines:?}; stderr={stderr}"
    );
    assert!(
        !saw_disable,
        "PCManFM disabled v2 within 2 s on a keyboard-less seat; Falkon-class, not a durable field; displayd={lines:?}; stderr={stderr}"
    );
}

/// ADR-363: OSK into packed PCManFM v2 on a keyboard-less seat.
/// Durable enable is `Fm::FolderViewListView`, not Filter.
/// No click. No Ctrl+L. Main shm after enable stays `484823fc…`.
#[test]
fn packed_pcmanfm_osk_without_seat_keyboard_hits_folderview_not_filter() {
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
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(runtime_dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("XDG_RUNTIME_DIR 0700");
    }
    let home_dir = tempfile::tempdir().expect("failed to create PCManFM HOME");
    let cfg = home_dir.path().join(".config/pcmanfm-qt/default");
    std::fs::create_dir_all(&cfg).expect("pcmanfm config dir");
    std::fs::write(
        cfg.join("settings.conf"),
        "[FolderView]\nShowFilter=true\n[Window]\nPathBarButtons=false\n",
    )
    .expect("write settings.conf");

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
        .env("XDG_CONFIG_HOME", home_dir.path().join(".config"))
        .env("QT_PLUGIN_PATH", pkg.join("plugins"))
        .env("QT_QPA_PLATFORM", "wayland")
        .env("XKB_CONFIG_ROOT", pkg.join("share/X11/xkb"))
        .env("QT_QPA_PLATFORMTHEME", "")
        .env("QT_LOGGING_TO_CONSOLE", "1")
        .env("QT_ASSUME_STDERR_HAS_CONSOLE", "1")
        .env(
            "QT_LOGGING_RULES",
            "qt.qpa.wayland.textinput.debug=true;qt.qpa.input.methods.debug=true",
        )
        .process_group(0)
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
    let mut saw_enable = false;
    let mut saw_commit = false;
    let mut saw_activated = false;
    let mut saw_kbd_focus = false;
    let mut saw_focus = false;
    let mut toplevel_surface = None;
    let mut hash_after = None;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline && !(saw_enable && ime_state.activate) {
        match log.recv_timeout(Duration::from_millis(200)) {
            Ok(line) => {
                if line.contains("new xdg_toplevel") {
                    saw_toplevel = true;
                }
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
                } else if line.contains("focus set to") {
                    saw_focus = true;
                }
                if line.contains("xdg activated") {
                    saw_activated = true;
                }
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                }
                if let Some(h) = toplevel_frame_hash(&line, &mut toplevel_surface) {
                    saw_frame = true;
                    hash_after = Some(h.to_string());
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }
    if !(saw_enable && ime_state.activate && saw_focus && !saw_kbd_focus && saw_activated) {
        reap_pcmanfm(&mut child);
        let stderr = stderr_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap_or_default();
        let _ = displayd.kill();
        let _ = displayd.wait();
        panic!(
            "PCManFM never enabled v2 for OSK; enable={saw_enable} activate={} kbd={saw_kbd_focus} xdg={saw_activated}; displayd={lines:?}; stderr={stderr}",
            ime_state.activate
        );
    }
    let hash_at_enable = hash_after.clone();

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

    let paint = Instant::now() + Duration::from_secs(2);
    while Instant::now() < paint {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("text-input-v2 commit_string") {
                    saw_commit = true;
                }
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
                }
                if let Some(h) = toplevel_frame_hash(&line, &mut toplevel_surface) {
                    hash_after = Some(h.to_string());
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }

    reap_pcmanfm(&mut child);
    let stderr = stderr_rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap_or_default();
    let _ = displayd.kill();
    let _ = displayd.wait();
    assert!(
        saw_toplevel && saw_frame && saw_commit && !saw_kbd_focus,
        "expected OSK commit_string on keyboard-less PCManFM; commit={saw_commit} kbd={saw_kbd_focus} at_enable={hash_at_enable:?} after={hash_after:?}; displayd={lines:?}; stderr={stderr}"
    );
    assert!(
        stderr.contains("FolderViewListView::inputMethodQuery"),
        "PCManFM v2 enable was not FolderViewListView; do not claim Filter typed; qt={stderr}"
    );
    assert!(
        !stderr.contains("discard commit_string"),
        "Qt discarded commit_string on keyboard-less PCManFM; qt={stderr}"
    );
    assert_eq!(
        hash_at_enable, hash_after,
        "PCManFM main shm changed after OSK; at_enable={hash_at_enable:?} after={hash_after:?}; displayd={lines:?}; stderr={stderr}"
    );
}

/// ADR-364 / ADR-365: one band click on a keyboard-less seat after
/// FolderView enable. `400 760` Filter, `400 40` PathEdit. Not a Y sweep.
fn packed_pcmanfm_band_click_osk(x: i32, y: i32, band: &str) {
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
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(runtime_dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("XDG_RUNTIME_DIR 0700");
    }
    let home_dir = tempfile::tempdir().expect("failed to create PCManFM HOME");
    let cfg = home_dir.path().join(".config/pcmanfm-qt/default");
    std::fs::create_dir_all(&cfg).expect("pcmanfm config dir");
    std::fs::write(
        cfg.join("settings.conf"),
        "[FolderView]\nShowFilter=true\n[Window]\nPathBarButtons=false\n",
    )
    .expect("write settings.conf");

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
        .env("XDG_CONFIG_HOME", home_dir.path().join(".config"))
        .env("QT_PLUGIN_PATH", pkg.join("plugins"))
        .env("QT_QPA_PLATFORM", "wayland")
        .env("XKB_CONFIG_ROOT", pkg.join("share/X11/xkb"))
        .env("QT_QPA_PLATFORMTHEME", "")
        .env("QT_LOGGING_TO_CONSOLE", "1")
        .env("QT_ASSUME_STDERR_HAS_CONSOLE", "1")
        .env(
            "QT_LOGGING_RULES",
            "qt.qpa.wayland.textinput.debug=true;qt.qpa.input.methods.debug=true",
        )
        .process_group(0)
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

    let mut saw_enable = false;
    let mut saw_activated = false;
    let mut saw_kbd_focus = false;
    let mut saw_focus = false;
    let mut saw_click = false;
    let mut saw_commit = false;
    let mut enable_count = 0u32;
    let mut toplevel_surface = None;
    let mut hash_after = None;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline && !(saw_enable && ime_state.activate) {
        match log.recv_timeout(Duration::from_millis(200)) {
            Ok(line) => {
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
                } else if line.contains("focus set to") {
                    saw_focus = true;
                }
                if line.contains("xdg activated") {
                    saw_activated = true;
                }
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                    enable_count += 1;
                }
                if let Some(h) = toplevel_frame_hash(&line, &mut toplevel_surface) {
                    hash_after = Some(h.to_string());
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }
    if !(saw_enable && ime_state.activate && saw_focus && !saw_kbd_focus && saw_activated) {
        reap_pcmanfm(&mut child);
        let stderr = stderr_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap_or_default();
        let _ = displayd.kill();
        let _ = displayd.wait();
        panic!(
            "pre-click {band}: PCManFM never enabled v2; enable={saw_enable} kbd={saw_kbd_focus}; displayd={lines:?}; stderr={stderr}"
        );
    }
    let hash_at_enable = hash_after.clone();

    if let Some(stdin) = displayd.stdin.as_mut() {
        writeln!(stdin, "inject-click {x} {y}").expect("inject-click");
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
                if line.contains("text-input-v2 enable") {
                    enable_count += 1;
                }
                if let Some(h) = toplevel_frame_hash(&line, &mut toplevel_surface) {
                    hash_after = Some(h.to_string());
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }
    if !saw_click || saw_kbd_focus {
        reap_pcmanfm(&mut child);
        let stderr = stderr_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap_or_default();
        let _ = displayd.kill();
        let _ = displayd.wait();
        panic!(
            "{band} click {x} {y}; click={saw_click} kbd={saw_kbd_focus}; displayd={lines:?}; stderr={stderr}"
        );
    }
    let hash_at_click = hash_after.clone();

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

    let paint = Instant::now() + Duration::from_secs(2);
    while Instant::now() < paint {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("text-input-v2 commit_string") {
                    saw_commit = true;
                }
                if line.contains("text-input-v2 enable") {
                    enable_count += 1;
                }
                if let Some(h) = toplevel_frame_hash(&line, &mut toplevel_surface) {
                    hash_after = Some(h.to_string());
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }

    reap_pcmanfm(&mut child);
    let stderr = stderr_rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap_or_default();
    let _ = displayd.kill();
    let _ = displayd.wait();
    assert!(
        saw_commit && !saw_kbd_focus,
        "expected OSK commit after {band} click; commit={saw_commit} enables={enable_count} kbd={saw_kbd_focus}; displayd={lines:?}; stderr={stderr}"
    );
    assert!(
        stderr.contains("FolderViewListView::inputMethodQuery"),
        "expected FolderView IM still after {band}; qt={stderr}"
    );
    assert!(
        !stderr.contains("QLineEdit::inputMethodQuery"),
        "{band} QLineEdit took IM after {x} {y}; do not claim typed without shm; qt={stderr}"
    );
    assert_eq!(
        hash_at_click, hash_after,
        "PCManFM shm changed after {band} click OSK; at_enable={hash_at_enable:?} at_click={hash_at_click:?} after={hash_after:?}; displayd={lines:?}; stderr={stderr}"
    );
}

/// ADR-364: one Filter-band click `400 760` on a keyboard-less seat
/// after FolderView enable. Not a Y sweep. Not Ctrl+L. Not Falkon.
#[test]
fn packed_pcmanfm_filter_click_without_seat_keyboard_still_folderview() {
    packed_pcmanfm_band_click_osk(400, 760, "Filter");
}

/// ADR-365: one PathEdit-band click `400 40` on a keyboard-less seat.
/// Not a Y sweep. Not Ctrl+L.
#[test]
fn packed_pcmanfm_pathedit_click_without_seat_keyboard_still_folderview() {
    packed_pcmanfm_band_click_osk(400, 40, "PathEdit");
}
