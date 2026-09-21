//! APP-06 / ADR-306: packaged aarch64 Falkon maps an xdg_toplevel on
//! host `saai-displayd` under `qemu-aarch64-static` and commits a
//! hashed shm frame. ADR-333: windowed URL `QLineEdit` click enables
//! v2; IME `commit_string` in that window reaches the field. ADR-334:
//! OSK hi! sequence (commit+delete) in the same window. Not a
//! panther typed field. WebEngine helper spawn via binfmt is host-only
//! and may fail; the Widgets chrome frame is the hello-frame.
//! ADR-354: without wl_keyboard, xdg Activated does not enable URL v2.
//! ADR-355: one URL click 640 20 on that seat enables v2. Not typed.
//! ADR-358: that enable disables within 2 s (not PCManFM ADR-357).
//! ADR-384: OSK immediately after that URL enable on the same seat.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use saai_ui_core::{Keyboard, OskImeOp};
#[cfg(unix)]
use std::os::unix::process::CommandExt;

fn displayd_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_saai-displayd"))
}

fn falkon_package() -> PathBuf {
    std::env::var("FALKON_PACKAGE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../dist/panther/packages/org.saaios.demo.falkon")
        })
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

fn reap_falkon(child: &mut Child) {
    // qemu-aarch64 leaves QtWebEngineProcess children; kill the group
    // so the next falkon_frame test can still map a frame.
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
fn falkon_commits_an_shm_frame_on_host_displayd() {
    let pkg = falkon_package();
    let falkon = pkg.join("bin/falkon");
    let px = pkg.join("lib/libpxbackend-1.0.so");
    assert!(
        falkon.is_file(),
        "missing packed falkon at {} — run os/targets/panther/build-falkon-package.sh",
        falkon.display()
    );
    assert!(
        px.is_file(),
        "packed falkon missing libpxbackend-1.0.so at {}",
        px.display()
    );
    assert!(
        !pkg.join("plugins/platforms/libqwayland-egl.so").exists(),
        "wayland-egl must not ship; it blocks the shm hello-frame"
    );

    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    let home_dir = tempfile::tempdir().expect("failed to create Falkon HOME");
    let (mut displayd, log) = spawn_displayd(runtime_dir.path());
    let socket_name = wait_for_socket(&log);
    let path = std::env::var("PATH").unwrap_or_default();
    let pkg = pkg.canonicalize().expect("canonicalize falkon package");
    let falkon = pkg.join("bin/falkon");

    let mut falkon_child = Command::new("qemu-aarch64-static")
        .arg("-L")
        .arg(&pkg)
        .arg(&falkon)
        .current_dir(&pkg)
        .env_clear()
        .env("PATH", &path)
        .env("XDG_RUNTIME_DIR", runtime_dir.path())
        .env("WAYLAND_DISPLAY", &socket_name)
        .env("HOME", home_dir.path())
        .env("QT_PLUGIN_PATH", pkg.join("plugins"))
        .env("QT_QPA_PLATFORM", "wayland")
        .env("XKB_CONFIG_ROOT", pkg.join("share/X11/xkb"))
        .env(
            "QTWEBENGINEPROCESS_PATH",
            pkg.join("libexec/QtWebEngineProcess"),
        )
        .env(
            "QTWEBENGINE_RESOURCES_PATH",
            pkg.join("share/qt6/resources"),
        )
        .env(
            "QTWEBENGINE_LOCALES_PATH",
            pkg.join("share/qt6/translations/qtwebengine_locales"),
        )
        .env("QTWEBENGINE_DISABLE_SANDBOX", "1")
        .env(
            "QTWEBENGINE_CHROMIUM_FLAGS",
            "--no-sandbox --disable-gpu --disable-gpu-compositing --use-gl=disabled",
        )
        .env("LIBGL_ALWAYS_SOFTWARE", "1")
        .env("QT_QUICK_BACKEND", "software")
        .env("QSG_RENDER_LOOP", "basic")
        .process_group(0)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("qemu-aarch64-static failed to spawn falkon");
    let stderr_rx = {
        let stderr = falkon_child.stderr.take().expect("falkon stderr");
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
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(25);
    while Instant::now() < deadline && !(saw_toplevel && saw_frame) {
        match log.recv_timeout(Duration::from_millis(200)) {
            Ok(line) => {
                if line.contains("new xdg_toplevel") {
                    assert!(
                        line.contains("1280x800 fullscreen=false"),
                        "Falkon must map the x86 windowed size, not a phone panel: {line}"
                    );
                    saw_toplevel = true;
                }
                if line.contains("frame sha256=") {
                    saw_frame = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    reap_falkon(&mut falkon_child);
    let stderr = stderr_rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap_or_default();
    assert!(
        saw_toplevel,
        "Falkon never created an xdg_toplevel; displayd={lines:?}; stderr={stderr}"
    );
    assert!(
        saw_frame,
        "Falkon never committed a hashed shm frame; displayd={lines:?}; stderr={stderr}"
    );
    assert!(
        displayd
            .try_wait()
            .expect("failed to poll saai-displayd")
            .is_none(),
        "saai-displayd exited during the Falkon frame"
    );
    let _ = displayd.kill();
    let _ = displayd.wait();
}

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

fn interesting(line: &str) -> bool {
    line.contains("text-input-v2")
        || line.contains("text-input-v3")
        || line.contains("injected click")
        || line.contains("injected synthetic key")
        || line.contains("new xdg_toplevel")
        || line.contains("frame sha256=")
        || line.contains("xdg activated")
        || line.contains("focus set to")
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

fn activated_frame_hash<'a>(line: &'a str, surface: &Option<String>) -> Option<&'a str> {
    if !line.contains("frame sha256=") {
        return None;
    }
    let id = line
        .split("wl_surface@")
        .nth(1)?
        .split(|c: char| !c.is_ascii_digit())
        .next()?;
    match surface {
        Some(s) if s == id => {}
        _ => return None,
    }
    line.split("frame sha256=")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
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

/// ADR-333: windowed packed Falkon chrome (URL QLineEdit) on host qemu.
/// hideTabsWithOneTab so chrome is the navigation toolbar only.
/// Click 640,20 in that band. Not a Y sweep. Not a panther typed field.
#[test]
fn falkon_url_click_text_input_v2_protocol() {
    let pkg = falkon_package();
    let falkon = pkg.join("bin/falkon");
    assert!(
        falkon.is_file(),
        "missing packed falkon at {} — set FALKON_PACKAGE_DIR",
        falkon.display()
    );

    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(runtime_dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("XDG_RUNTIME_DIR 0700");
    }
    let home_dir = tempfile::tempdir().expect("failed to create Falkon HOME");
    let cfg = home_dir.path().join(".config/falkon/profiles/default");
    std::fs::create_dir_all(&cfg).expect("falkon profile dir");
    std::fs::write(
        cfg.join("settings.ini"),
        "[Browser-View-Settings]\n\
         showNavigationToolbar=true\n\
         showMenubar=false\n\
         showBookmarksToolbar=false\n\
         showStatusBar=false\n\
         [Browser-Tabs-Settings]\n\
         hideTabsWithOneTab=true\n",
    )
    .expect("write falkon settings.ini");
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
    let pkg = pkg.canonicalize().expect("canonicalize falkon package");
    let falkon = pkg.join("bin/falkon");

    let mut falkon_child = Command::new("qemu-aarch64-static")
        .arg("-L")
        .arg(&pkg)
        .arg(&falkon)
        .arg("--private-browsing")
        .arg("about:blank")
        .current_dir(&pkg)
        .env_clear()
        .env("PATH", &path)
        .env("XDG_RUNTIME_DIR", runtime_dir.path())
        .env("WAYLAND_DISPLAY", &socket_name)
        .env("HOME", home_dir.path())
        .env("XDG_CONFIG_HOME", home_dir.path().join(".config"))
        .env("QT_LOGGING_TO_CONSOLE", "1")
        .env("QT_PLUGIN_PATH", pkg.join("plugins"))
        .env("QT_QPA_PLATFORM", "wayland")
        .env("XKB_CONFIG_ROOT", pkg.join("share/X11/xkb"))
        .env(
            "QTWEBENGINEPROCESS_PATH",
            pkg.join("libexec/QtWebEngineProcess"),
        )
        .env(
            "QTWEBENGINE_RESOURCES_PATH",
            pkg.join("share/qt6/resources"),
        )
        .env(
            "QTWEBENGINE_LOCALES_PATH",
            pkg.join("share/qt6/translations/qtwebengine_locales"),
        )
        .env("QTWEBENGINE_DISABLE_SANDBOX", "1")
        .env(
            "QTWEBENGINE_CHROMIUM_FLAGS",
            "--no-sandbox --disable-gpu --disable-gpu-compositing --use-gl=disabled",
        )
        .env("LIBGL_ALWAYS_SOFTWARE", "1")
        .env("QT_QUICK_BACKEND", "software")
        .env("QSG_RENDER_LOOP", "basic")
        .env(
            "QT_LOGGING_RULES",
            "qt.qpa.wayland.textinput.debug=true;qt.qpa.input.methods.debug=true",
        )
        .process_group(0)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("qemu-aarch64-static failed to spawn falkon");
    let stderr_rx = {
        let stderr = falkon_child.stderr.take().expect("falkon stderr");
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
    let mut saw_disable = false;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(25);
    while Instant::now() < deadline && !(saw_toplevel && saw_frame) {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("new xdg_toplevel") {
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
        saw_toplevel && saw_frame,
        "Falkon never framed before URL click; displayd={:?}; get={saw_get} enable={saw_enable}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );

    if let Some(stdin) = displayd.stdin.as_mut() {
        writeln!(stdin, "inject-click 640 20").expect("inject-click");
        let _ = stdin.flush();
    }
    let mut saw_click = false;
    let mut saw_commit = false;
    let click_deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < click_deadline && !(saw_click && saw_enable) {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("injected click") {
                    saw_click = true;
                }
                if line.contains("text-input-v2 get") {
                    saw_get = true;
                }
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
        saw_click,
        "displayd never injected URL click; displayd={:?}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    assert!(
        saw_enable,
        "Falkon URL click 640,20 did not enable v2; get={saw_get} disable={saw_disable} activate={}; displayd={:?}; qt={}",
        ime_state.activate,
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>(),
        stderr_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap_or_default()
    );

    // Commit in the enable window. Waiting out the click settle lets Qt
    // disable first (IME active becomes None; commit_string is dropped).
    ime.commit_string(String::from("hi!"));
    ime.commit(0);
    queue
        .roundtrip(&mut ime_state)
        .expect("ime commit roundtrip");
    let commit_deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < commit_deadline && !saw_commit {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("text-input-v2 commit_string") {
                    saw_commit = true;
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
        saw_commit,
        "IME commit_string did not reach Falkon v2 after URL enable; disable={saw_disable}; displayd={:?}; qt={}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>(),
        stderr_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap_or_default()
    );

    reap_falkon(&mut falkon_child);
    let _ = displayd.kill();
    let _ = displayd.wait();
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

/// ADR-334: same URL enable window, OSK hi! sequence (commit + delete),
/// not a single commit_string. Not a painted LocationBar. Not panther.
#[test]
fn falkon_url_osk_hi_bang_reaches_v2() {
    let pkg = falkon_package();
    let falkon = pkg.join("bin/falkon");
    assert!(
        falkon.is_file(),
        "missing packed falkon at {} — set FALKON_PACKAGE_DIR",
        falkon.display()
    );

    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(runtime_dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("XDG_RUNTIME_DIR 0700");
    }
    let home_dir = tempfile::tempdir().expect("failed to create Falkon HOME");
    let cfg = home_dir.path().join(".config/falkon/profiles/default");
    std::fs::create_dir_all(&cfg).expect("falkon profile dir");
    std::fs::write(
        cfg.join("settings.ini"),
        "[Browser-View-Settings]\n\
         showNavigationToolbar=true\n\
         showMenubar=false\n\
         showBookmarksToolbar=false\n\
         showStatusBar=false\n\
         [Browser-Tabs-Settings]\n\
         hideTabsWithOneTab=true\n",
    )
    .expect("write falkon settings.ini");
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
    let pkg = pkg.canonicalize().expect("canonicalize falkon package");
    let falkon = pkg.join("bin/falkon");

    let mut falkon_child = Command::new("qemu-aarch64-static")
        .arg("-L")
        .arg(&pkg)
        .arg(&falkon)
        .arg("--private-browsing")
        .arg("about:blank")
        .current_dir(&pkg)
        .env_clear()
        .env("PATH", &path)
        .env("XDG_RUNTIME_DIR", runtime_dir.path())
        .env("WAYLAND_DISPLAY", &socket_name)
        .env("HOME", home_dir.path())
        .env("XDG_CONFIG_HOME", home_dir.path().join(".config"))
        .env("QT_LOGGING_TO_CONSOLE", "1")
        .env("QT_ASSUME_STDERR_HAS_CONSOLE", "1")
        .env(
            "QT_LOGGING_RULES",
            "qt.qpa.wayland.textinput.debug=true;qt.qpa.input.methods.debug=true",
        )
        .env("QT_PLUGIN_PATH", pkg.join("plugins"))
        .env("QT_QPA_PLATFORM", "wayland")
        .env("XKB_CONFIG_ROOT", pkg.join("share/X11/xkb"))
        .env(
            "QTWEBENGINEPROCESS_PATH",
            pkg.join("libexec/QtWebEngineProcess"),
        )
        .env(
            "QTWEBENGINE_RESOURCES_PATH",
            pkg.join("share/qt6/resources"),
        )
        .env(
            "QTWEBENGINE_LOCALES_PATH",
            pkg.join("share/qt6/translations/qtwebengine_locales"),
        )
        .env("QTWEBENGINE_DISABLE_SANDBOX", "1")
        .env(
            "QTWEBENGINE_CHROMIUM_FLAGS",
            "--no-sandbox --disable-gpu --disable-gpu-compositing --use-gl=disabled",
        )
        .env("LIBGL_ALWAYS_SOFTWARE", "1")
        .env("QT_QUICK_BACKEND", "software")
        .env("QSG_RENDER_LOOP", "basic")
        .process_group(0)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("qemu-aarch64-static failed to spawn falkon");
    let stderr_rx = {
        let stderr = falkon_child.stderr.take().expect("falkon stderr");
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
    let mut toplevel_surface = None;
    let mut hash_before = None::<String>;
    let mut hash_after = None::<String>;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(25);
    while Instant::now() < deadline && !(saw_toplevel && saw_frame) {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("new xdg_toplevel") {
                    saw_toplevel = true;
                }
                if line.contains("frame sha256=") {
                    saw_frame = true;
                }
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                }
                if line.contains("text-input-v2 disable") {
                    saw_disable = true;
                }
                if let Some(h) = toplevel_frame_hash(&line, &mut toplevel_surface) {
                    if hash_before.is_none() {
                        hash_before = Some(h.to_string());
                    }
                    hash_after = Some(h.to_string());
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
        "Falkon never framed before URL OSK; displayd={:?}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );

    if let Some(stdin) = displayd.stdin.as_mut() {
        writeln!(stdin, "inject-click 640 20").expect("inject-click");
        let _ = stdin.flush();
    }
    let mut click_count = 0u32;
    let click_deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < click_deadline && !(click_count >= 1 && saw_enable) {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("injected click") {
                    click_count += 1;
                }
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                }
                if line.contains("text-input-v2 disable") {
                    saw_disable = true;
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
    assert!(
        click_count >= 1 && saw_enable,
        "Falkon URL click did not enable v2; clicks={click_count} disable={saw_disable}; displayd={:?}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    let disable_deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < disable_deadline && !saw_disable {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("text-input-v2 disable") {
                    saw_disable = true;
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
    saw_enable = false;
    if let Some(stdin) = displayd.stdin.as_mut() {
        writeln!(stdin, "inject-click 640 20").expect("inject-click 2");
        let _ = stdin.flush();
    }
    let click2_deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < click2_deadline && !(click_count >= 2 && saw_enable) {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("injected click") {
                    click_count += 1;
                }
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                }
                if line.contains("text-input-v2 disable") {
                    saw_disable = true;
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
    assert!(
        click_count >= 2 && saw_enable,
        "Falkon URL second click did not re-enable v2; clicks={click_count} disable={saw_disable}; displayd={:?}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    let hash_at_osk = hash_after.clone();

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

    let mut commit_count = 0u32;
    let mut saw_delete = false;
    let osk_deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < osk_deadline && (commit_count < 4 || !saw_delete) {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("text-input-v2 commit_string") {
                    commit_count += 1;
                }
                if line.contains("text-input-v2 delete_surrounding") {
                    saw_delete = true;
                }
                if line.contains("text-input-v2 disable") {
                    saw_disable = true;
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

    let settle = Instant::now() + Duration::from_millis(700);
    while Instant::now() < settle {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
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

    reap_falkon(&mut falkon_child);
    let stderr = stderr_rx
        .recv_timeout(Duration::from_secs(1))
        .unwrap_or_default();
    let _ = displayd.kill();
    let _ = displayd.wait();
    assert!(
        commit_count >= 4 && saw_delete,
        "OSK hi! did not reach Falkon v2 (commit={commit_count} delete={saw_delete} disable={saw_disable}); at_osk={hash_at_osk:?} after={hash_after:?}; displayd={:?}; qt={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    assert_eq!(
        hash_at_osk, hash_after,
        "Falkon main shm changed after OSK; do not claim LocationBar paint; first={hash_before:?} at_osk={hash_at_osk:?} after={hash_after:?}; displayd={:?}; qt={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    assert!(
        stderr.contains("qt.qpa.input.methods"),
        "Falkon OSK spawn must carry QT_LOGGING_RULES; qt={stderr}"
    );
    assert!(
        !stderr.contains("discard commit_string"),
        "Qt discarded commit_string despite ADR-339 defer; qt={stderr}"
    );
}

/// ADR-342: after the same URL second-click enable, `inject-key` KEY_A
/// (wl_keyboard, not IME). Distinguishes widget key focus from silent
/// `focusObject() == null` on commit_string. Not a Y sweep. Not panther.
#[test]
fn falkon_url_inject_key_a_after_second_click() {
    let pkg = falkon_package();
    let falkon = pkg.join("bin/falkon");
    assert!(
        falkon.is_file(),
        "missing packed falkon at {} — set FALKON_PACKAGE_DIR",
        falkon.display()
    );

    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(runtime_dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("XDG_RUNTIME_DIR 0700");
    }
    let home_dir = tempfile::tempdir().expect("failed to create Falkon HOME");
    let cfg = home_dir.path().join(".config/falkon/profiles/default");
    std::fs::create_dir_all(&cfg).expect("falkon profile dir");
    std::fs::write(
        cfg.join("settings.ini"),
        "[Browser-View-Settings]\n\
         showNavigationToolbar=true\n\
         showMenubar=false\n\
         showBookmarksToolbar=false\n\
         showStatusBar=false\n\
         [Browser-Tabs-Settings]\n\
         hideTabsWithOneTab=true\n",
    )
    .expect("write falkon settings.ini");
    let (mut displayd, log) = spawn_displayd(runtime_dir.path());
    let socket_name = wait_for_socket(&log);

    let path = std::env::var("PATH").unwrap_or_default();
    let pkg = pkg.canonicalize().expect("canonicalize falkon package");
    let falkon = pkg.join("bin/falkon");

    let mut falkon_child = Command::new("qemu-aarch64-static")
        .arg("-L")
        .arg(&pkg)
        .arg(&falkon)
        .arg("--private-browsing")
        .arg("about:blank")
        .current_dir(&pkg)
        .env_clear()
        .env("PATH", &path)
        .env("XDG_RUNTIME_DIR", runtime_dir.path())
        .env("WAYLAND_DISPLAY", &socket_name)
        .env("HOME", home_dir.path())
        .env("XDG_CONFIG_HOME", home_dir.path().join(".config"))
        .env("QT_LOGGING_TO_CONSOLE", "1")
        .env("QT_ASSUME_STDERR_HAS_CONSOLE", "1")
        .env(
            "QT_LOGGING_RULES",
            "qt.qpa.wayland.textinput.debug=true;qt.qpa.input.methods.debug=true",
        )
        .env("QT_PLUGIN_PATH", pkg.join("plugins"))
        .env("QT_QPA_PLATFORM", "wayland")
        .env("XKB_CONFIG_ROOT", pkg.join("share/X11/xkb"))
        .env(
            "QTWEBENGINEPROCESS_PATH",
            pkg.join("libexec/QtWebEngineProcess"),
        )
        .env(
            "QTWEBENGINE_RESOURCES_PATH",
            pkg.join("share/qt6/resources"),
        )
        .env(
            "QTWEBENGINE_LOCALES_PATH",
            pkg.join("share/qt6/translations/qtwebengine_locales"),
        )
        .env("QTWEBENGINE_DISABLE_SANDBOX", "1")
        .env(
            "QTWEBENGINE_CHROMIUM_FLAGS",
            "--no-sandbox --disable-gpu --disable-gpu-compositing --use-gl=disabled",
        )
        .env("LIBGL_ALWAYS_SOFTWARE", "1")
        .env("QT_QUICK_BACKEND", "software")
        .env("QSG_RENDER_LOOP", "basic")
        .process_group(0)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("qemu-aarch64-static failed to spawn falkon");
    let stderr_rx = {
        let stderr = falkon_child.stderr.take().expect("falkon stderr");
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
    let mut toplevel_surface = None;
    let mut hash_before = None::<String>;
    let mut hash_after = None::<String>;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(25);
    while Instant::now() < deadline && !(saw_toplevel && saw_frame) {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("new xdg_toplevel") {
                    saw_toplevel = true;
                }
                if line.contains("frame sha256=") {
                    saw_frame = true;
                }
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                }
                if line.contains("text-input-v2 disable") {
                    saw_disable = true;
                }
                if let Some(h) = toplevel_frame_hash(&line, &mut toplevel_surface) {
                    if hash_before.is_none() {
                        hash_before = Some(h.to_string());
                    }
                    hash_after = Some(h.to_string());
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    assert!(
        saw_toplevel && saw_frame,
        "Falkon never framed before KEY_A; displayd={:?}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );

    if let Some(stdin) = displayd.stdin.as_mut() {
        writeln!(stdin, "inject-click 640 20").expect("inject-click");
        let _ = stdin.flush();
    }
    let mut click_count = 0u32;
    let click_deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < click_deadline && !(click_count >= 1 && saw_enable) {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("injected click") {
                    click_count += 1;
                }
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                }
                if line.contains("text-input-v2 disable") {
                    saw_disable = true;
                }
                if let Some(h) = toplevel_frame_hash(&line, &mut toplevel_surface) {
                    hash_after = Some(h.to_string());
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    assert!(
        click_count >= 1 && saw_enable,
        "Falkon URL click did not enable v2; clicks={click_count} disable={saw_disable}; displayd={:?}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    let disable_deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < disable_deadline && !saw_disable {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("text-input-v2 disable") {
                    saw_disable = true;
                }
                if let Some(h) = toplevel_frame_hash(&line, &mut toplevel_surface) {
                    hash_after = Some(h.to_string());
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    saw_enable = false;
    if let Some(stdin) = displayd.stdin.as_mut() {
        writeln!(stdin, "inject-click 640 20").expect("inject-click 2");
        let _ = stdin.flush();
    }
    let click2_deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < click2_deadline && !(click_count >= 2 && saw_enable) {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("injected click") {
                    click_count += 1;
                }
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                }
                if line.contains("text-input-v2 disable") {
                    saw_disable = true;
                }
                if let Some(h) = toplevel_frame_hash(&line, &mut toplevel_surface) {
                    hash_after = Some(h.to_string());
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    assert!(
        click_count >= 2 && saw_enable,
        "Falkon URL second click did not re-enable v2; clicks={click_count} disable={saw_disable}; displayd={:?}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    let hash_at_key = hash_after.clone();

    if let Some(stdin) = displayd.stdin.as_mut() {
        writeln!(stdin, "inject-key").expect("inject-key");
        let _ = stdin.flush();
    }
    let mut saw_key = false;
    let key_deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < key_deadline && !saw_key {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("injected synthetic key") {
                    saw_key = true;
                }
                if let Some(h) = toplevel_frame_hash(&line, &mut toplevel_surface) {
                    hash_after = Some(h.to_string());
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
                if line.contains("injected synthetic key") {
                    saw_key = true;
                }
                if let Some(h) = toplevel_frame_hash(&line, &mut toplevel_surface) {
                    hash_after = Some(h.to_string());
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    reap_falkon(&mut falkon_child);
    let stderr = stderr_rx
        .recv_timeout(Duration::from_secs(1))
        .unwrap_or_default();
    let _ = displayd.kill();
    let _ = displayd.wait();
    assert!(
        saw_key,
        "displayd never injected KEY_A; at_key={hash_at_key:?} after={hash_after:?}; displayd={:?}; qt={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    assert_eq!(
        hash_at_key, hash_after,
        "Falkon main shm changed after KEY_A; do not claim LocationBar paint; first={hash_before:?} at_key={hash_at_key:?} after={hash_after:?}; displayd={:?}; qt={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
}

/// ADR-354: packed Falkon on a keyboard-less seat binds v2 after xdg
/// Activated and does not enable. LocationBar is not auto-focused.
/// PCManFM chrome does enable (ADR-353). Not a panther field. Do not click.
#[test]
fn packed_falkon_without_seat_keyboard_does_not_enable_v2() {
    let pkg = falkon_package();
    let falkon = pkg.join("bin/falkon");
    assert!(
        falkon.is_file(),
        "missing packed falkon at {} — set FALKON_PACKAGE_DIR",
        falkon.display()
    );

    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(runtime_dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("XDG_RUNTIME_DIR 0700");
    }
    let home_dir = tempfile::tempdir().expect("failed to create Falkon HOME");
    let cfg = home_dir.path().join(".config/falkon/profiles/default");
    std::fs::create_dir_all(&cfg).expect("falkon profile dir");
    std::fs::write(
        cfg.join("settings.ini"),
        "[Browser-View-Settings]\n\
         showNavigationToolbar=true\n\
         showMenubar=false\n\
         showBookmarksToolbar=false\n\
         showStatusBar=false\n\
         [Browser-Tabs-Settings]\n\
         hideTabsWithOneTab=true\n",
    )
    .expect("write falkon settings.ini");
    let (mut displayd, log) =
        spawn_displayd_with(runtime_dir.path(), &[("SAAIOS_SEAT_NO_KEYBOARD", "1")]);
    let socket_name = wait_for_socket(&log);

    let path = std::env::var("PATH").unwrap_or_default();
    let pkg = pkg.canonicalize().expect("canonicalize falkon package");
    let falkon = pkg.join("bin/falkon");

    let mut falkon_child = Command::new("qemu-aarch64-static")
        .arg("-L")
        .arg(&pkg)
        .arg(&falkon)
        .arg("--private-browsing")
        .arg("about:blank")
        .current_dir(&pkg)
        .env_clear()
        .env("PATH", &path)
        .env("XDG_RUNTIME_DIR", runtime_dir.path())
        .env("WAYLAND_DISPLAY", &socket_name)
        .env("HOME", home_dir.path())
        .env("XDG_CONFIG_HOME", home_dir.path().join(".config"))
        .env("QT_PLUGIN_PATH", pkg.join("plugins"))
        .env("QT_QPA_PLATFORM", "wayland")
        .env("XKB_CONFIG_ROOT", pkg.join("share/X11/xkb"))
        .env(
            "QTWEBENGINEPROCESS_PATH",
            pkg.join("libexec/QtWebEngineProcess"),
        )
        .env(
            "QTWEBENGINE_RESOURCES_PATH",
            pkg.join("share/qt6/resources"),
        )
        .env(
            "QTWEBENGINE_LOCALES_PATH",
            pkg.join("share/qt6/translations/qtwebengine_locales"),
        )
        .env("QTWEBENGINE_DISABLE_SANDBOX", "1")
        .env(
            "QTWEBENGINE_CHROMIUM_FLAGS",
            "--no-sandbox --disable-gpu --disable-gpu-compositing --use-gl=disabled",
        )
        .env("LIBGL_ALWAYS_SOFTWARE", "1")
        .env("QT_QUICK_BACKEND", "software")
        .env("QSG_RENDER_LOOP", "basic")
        .process_group(0)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("qemu-aarch64-static failed to spawn falkon");
    let stderr_rx = {
        let stderr = falkon_child.stderr.take().expect("falkon stderr");
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
    let deadline = Instant::now() + Duration::from_secs(25);
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
    }

    reap_falkon(&mut falkon_child);
    let stderr = stderr_rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap_or_default();
    let _ = displayd.kill();
    let _ = displayd.wait();
    assert!(
        saw_toplevel && saw_frame && saw_activated && saw_focus && !saw_kbd_focus,
        "expected framed Falkon with xdg Activated and no wl_keyboard; kbd={saw_kbd_focus} focus={saw_focus} xdg={saw_activated}; displayd={:?}; stderr={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    assert!(
        saw_get,
        "Falkon never zwp_text_input_v2.get; displayd={:?}; stderr={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    assert!(
        !saw_enable,
        "Falkon enabled v2 without a LocationBar click; do not claim URL typed; displayd={:?}; stderr={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
}

/// ADR-355: one URL click `640 20` on a keyboard-less seat. Not a Y
/// sweep. Not a second click. Not OSK. LocationBar protocol only.
#[test]
fn packed_falkon_url_click_without_seat_keyboard_enables_v2() {
    let pkg = falkon_package();
    let falkon = pkg.join("bin/falkon");
    assert!(
        falkon.is_file(),
        "missing packed falkon at {} — set FALKON_PACKAGE_DIR",
        falkon.display()
    );

    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(runtime_dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("XDG_RUNTIME_DIR 0700");
    }
    let home_dir = tempfile::tempdir().expect("failed to create Falkon HOME");
    let cfg = home_dir.path().join(".config/falkon/profiles/default");
    std::fs::create_dir_all(&cfg).expect("falkon profile dir");
    std::fs::write(
        cfg.join("settings.ini"),
        "[Browser-View-Settings]\n\
         showNavigationToolbar=true\n\
         showMenubar=false\n\
         showBookmarksToolbar=false\n\
         showStatusBar=false\n\
         [Browser-Tabs-Settings]\n\
         hideTabsWithOneTab=true\n",
    )
    .expect("write falkon settings.ini");
    let (mut displayd, log) =
        spawn_displayd_with(runtime_dir.path(), &[("SAAIOS_SEAT_NO_KEYBOARD", "1")]);
    let socket_name = wait_for_socket(&log);

    let path = std::env::var("PATH").unwrap_or_default();
    let pkg = pkg.canonicalize().expect("canonicalize falkon package");
    let falkon = pkg.join("bin/falkon");

    let mut falkon_child = Command::new("qemu-aarch64-static")
        .arg("-L")
        .arg(&pkg)
        .arg(&falkon)
        .arg("--private-browsing")
        .arg("about:blank")
        .current_dir(&pkg)
        .env_clear()
        .env("PATH", &path)
        .env("XDG_RUNTIME_DIR", runtime_dir.path())
        .env("WAYLAND_DISPLAY", &socket_name)
        .env("HOME", home_dir.path())
        .env("XDG_CONFIG_HOME", home_dir.path().join(".config"))
        .env("QT_PLUGIN_PATH", pkg.join("plugins"))
        .env("QT_QPA_PLATFORM", "wayland")
        .env("XKB_CONFIG_ROOT", pkg.join("share/X11/xkb"))
        .env(
            "QTWEBENGINEPROCESS_PATH",
            pkg.join("libexec/QtWebEngineProcess"),
        )
        .env(
            "QTWEBENGINE_RESOURCES_PATH",
            pkg.join("share/qt6/resources"),
        )
        .env(
            "QTWEBENGINE_LOCALES_PATH",
            pkg.join("share/qt6/translations/qtwebengine_locales"),
        )
        .env("QTWEBENGINE_DISABLE_SANDBOX", "1")
        .env(
            "QTWEBENGINE_CHROMIUM_FLAGS",
            "--no-sandbox --disable-gpu --disable-gpu-compositing --use-gl=disabled",
        )
        .env("LIBGL_ALWAYS_SOFTWARE", "1")
        .env("QT_QUICK_BACKEND", "software")
        .env("QSG_RENDER_LOOP", "basic")
        .process_group(0)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("qemu-aarch64-static failed to spawn falkon");
    let stderr_rx = {
        let stderr = falkon_child.stderr.take().expect("falkon stderr");
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
    let mut saw_activated = false;
    let mut saw_kbd_focus = false;
    let mut saw_focus = false;
    let mut saw_click = false;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(25);
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
                if line.contains("text-input-v2 enable") {
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
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    if !(saw_toplevel && saw_frame && saw_activated && saw_focus && !saw_kbd_focus && !saw_enable) {
        reap_falkon(&mut falkon_child);
        let stderr = stderr_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap_or_default();
        let _ = displayd.kill();
        let _ = displayd.wait();
        panic!(
            "pre-click: expected framed Falkon Activated without enable; kbd={saw_kbd_focus} focus={saw_focus} xdg={saw_activated} enable={saw_enable}; displayd={:?}; stderr={stderr}",
            lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
        );
    }

    if let Some(stdin) = displayd.stdin.as_mut() {
        writeln!(stdin, "inject-click 640 20").expect("inject-click");
        let _ = stdin.flush();
    }
    let click_deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < click_deadline && !(saw_click && saw_enable) {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("injected click") {
                    saw_click = true;
                }
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
                }
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    reap_falkon(&mut falkon_child);
    let stderr = stderr_rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap_or_default();
    let _ = displayd.kill();
    let _ = displayd.wait();
    assert!(
        saw_click && !saw_kbd_focus,
        "URL click 640 20 on keyboard-less seat; kbd={saw_kbd_focus} click={saw_click}; displayd={:?}; stderr={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    assert!(
        saw_enable,
        "Falkon URL click 640 20 did not enable v2 without wl_keyboard; do not sweep Y; displayd={:?}; stderr={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
}

/// ADR-358: after that URL enable on a keyboard-less seat, 2 s quiet.
/// Falkon URL disable (ADR-337) vs PCManFM stay (ADR-357). One click.
/// Not a second click. Not OSK.
#[test]
fn packed_falkon_url_v2_disables_without_seat_keyboard() {
    let pkg = falkon_package();
    let falkon = pkg.join("bin/falkon");
    assert!(
        falkon.is_file(),
        "missing packed falkon at {} — set FALKON_PACKAGE_DIR",
        falkon.display()
    );

    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(runtime_dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("XDG_RUNTIME_DIR 0700");
    }
    let home_dir = tempfile::tempdir().expect("failed to create Falkon HOME");
    let cfg = home_dir.path().join(".config/falkon/profiles/default");
    std::fs::create_dir_all(&cfg).expect("falkon profile dir");
    std::fs::write(
        cfg.join("settings.ini"),
        "[Browser-View-Settings]\n\
         showNavigationToolbar=true\n\
         showMenubar=false\n\
         showBookmarksToolbar=false\n\
         showStatusBar=false\n\
         [Browser-Tabs-Settings]\n\
         hideTabsWithOneTab=true\n",
    )
    .expect("write falkon settings.ini");
    let (mut displayd, log) =
        spawn_displayd_with(runtime_dir.path(), &[("SAAIOS_SEAT_NO_KEYBOARD", "1")]);
    let socket_name = wait_for_socket(&log);

    let path = std::env::var("PATH").unwrap_or_default();
    let pkg = pkg.canonicalize().expect("canonicalize falkon package");
    let falkon = pkg.join("bin/falkon");

    let mut falkon_child = Command::new("qemu-aarch64-static")
        .arg("-L")
        .arg(&pkg)
        .arg(&falkon)
        .arg("--private-browsing")
        .arg("about:blank")
        .current_dir(&pkg)
        .env_clear()
        .env("PATH", &path)
        .env("XDG_RUNTIME_DIR", runtime_dir.path())
        .env("WAYLAND_DISPLAY", &socket_name)
        .env("HOME", home_dir.path())
        .env("XDG_CONFIG_HOME", home_dir.path().join(".config"))
        .env("QT_PLUGIN_PATH", pkg.join("plugins"))
        .env("QT_QPA_PLATFORM", "wayland")
        .env("XKB_CONFIG_ROOT", pkg.join("share/X11/xkb"))
        .env(
            "QTWEBENGINEPROCESS_PATH",
            pkg.join("libexec/QtWebEngineProcess"),
        )
        .env(
            "QTWEBENGINE_RESOURCES_PATH",
            pkg.join("share/qt6/resources"),
        )
        .env(
            "QTWEBENGINE_LOCALES_PATH",
            pkg.join("share/qt6/translations/qtwebengine_locales"),
        )
        .env("QTWEBENGINE_DISABLE_SANDBOX", "1")
        .env(
            "QTWEBENGINE_CHROMIUM_FLAGS",
            "--no-sandbox --disable-gpu --disable-gpu-compositing --use-gl=disabled",
        )
        .env("LIBGL_ALWAYS_SOFTWARE", "1")
        .env("QT_QUICK_BACKEND", "software")
        .env("QSG_RENDER_LOOP", "basic")
        .process_group(0)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("qemu-aarch64-static failed to spawn falkon");
    let stderr_rx = {
        let stderr = falkon_child.stderr.take().expect("falkon stderr");
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
    let mut saw_click = false;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(25);
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
                if line.contains("text-input-v2 enable") {
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
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    if !(saw_toplevel && saw_frame && saw_activated && saw_focus && !saw_kbd_focus && !saw_enable) {
        reap_falkon(&mut falkon_child);
        let stderr = stderr_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap_or_default();
        let _ = displayd.kill();
        let _ = displayd.wait();
        panic!(
            "pre-click: expected framed Falkon Activated without enable; kbd={saw_kbd_focus} focus={saw_focus} xdg={saw_activated} enable={saw_enable}; displayd={:?}; stderr={stderr}",
            lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
        );
    }

    if let Some(stdin) = displayd.stdin.as_mut() {
        writeln!(stdin, "inject-click 640 20").expect("inject-click");
        let _ = stdin.flush();
    }
    let click_deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < click_deadline && !(saw_click && saw_enable) {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("injected click") {
                    saw_click = true;
                }
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
                }
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
    }
    if !(saw_click && saw_enable && !saw_kbd_focus) {
        reap_falkon(&mut falkon_child);
        let stderr = stderr_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap_or_default();
        let _ = displayd.kill();
        let _ = displayd.wait();
        panic!(
            "URL click did not enable v2 without wl_keyboard; click={saw_click} enable={saw_enable} kbd={saw_kbd_focus}; displayd={:?}; stderr={stderr}",
            lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
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

    reap_falkon(&mut falkon_child);
    let stderr = stderr_rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap_or_default();
    let _ = displayd.kill();
    let _ = displayd.wait();
    assert!(
        saw_disable && !saw_kbd_focus,
        "Falkon URL v2 after click 640 20; expected disable within 2 s on keyboard-less seat (ADR-337 class, not PCManFM ADR-357); disable={saw_disable} kbd={saw_kbd_focus}; displayd={:?}; stderr={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
}

/// ADR-384: one URL click `640 20` on a keyboard-less seat, then OSK
/// immediately. v2 `commit_string` reaches the field. Toplevel shm
/// stays `6cd11128…`. Not a Y sweep. Not typed LocationBar.
#[test]
fn packed_falkon_url_osk_immediately_after_enable_without_seat_keyboard() {
    let pkg = falkon_package();
    let falkon = pkg.join("bin/falkon");
    assert!(
        falkon.is_file(),
        "missing packed falkon at {} — set FALKON_PACKAGE_DIR",
        falkon.display()
    );

    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(runtime_dir.path(), std::fs::Permissions::from_mode(0o700))
            .expect("XDG_RUNTIME_DIR 0700");
    }
    let home_dir = tempfile::tempdir().expect("failed to create Falkon HOME");
    let cfg = home_dir.path().join(".config/falkon/profiles/default");
    std::fs::create_dir_all(&cfg).expect("falkon profile dir");
    std::fs::write(
        cfg.join("settings.ini"),
        "[Browser-View-Settings]\n\
         showNavigationToolbar=true\n\
         showMenubar=false\n\
         showBookmarksToolbar=false\n\
         showStatusBar=false\n\
         [Browser-Tabs-Settings]\n\
         hideTabsWithOneTab=true\n",
    )
    .expect("write falkon settings.ini");
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
    let pkg = pkg.canonicalize().expect("canonicalize falkon package");
    let falkon = pkg.join("bin/falkon");

    let mut falkon_child = Command::new("qemu-aarch64-static")
        .arg("-L")
        .arg(&pkg)
        .arg(&falkon)
        .arg("--private-browsing")
        .arg("about:blank")
        .current_dir(&pkg)
        .env_clear()
        .env("PATH", &path)
        .env("XDG_RUNTIME_DIR", runtime_dir.path())
        .env("WAYLAND_DISPLAY", &socket_name)
        .env("HOME", home_dir.path())
        .env("XDG_CONFIG_HOME", home_dir.path().join(".config"))
        .env("QT_PLUGIN_PATH", pkg.join("plugins"))
        .env("QT_QPA_PLATFORM", "wayland")
        .env("XKB_CONFIG_ROOT", pkg.join("share/X11/xkb"))
        .env(
            "QTWEBENGINEPROCESS_PATH",
            pkg.join("libexec/QtWebEngineProcess"),
        )
        .env(
            "QTWEBENGINE_RESOURCES_PATH",
            pkg.join("share/qt6/resources"),
        )
        .env(
            "QTWEBENGINE_LOCALES_PATH",
            pkg.join("share/qt6/translations/qtwebengine_locales"),
        )
        .env("QTWEBENGINE_DISABLE_SANDBOX", "1")
        .env(
            "QTWEBENGINE_CHROMIUM_FLAGS",
            "--no-sandbox --disable-gpu --disable-gpu-compositing --use-gl=disabled",
        )
        .env("LIBGL_ALWAYS_SOFTWARE", "1")
        .env("QT_QUICK_BACKEND", "software")
        .env("QSG_RENDER_LOOP", "basic")
        .process_group(0)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("qemu-aarch64-static failed to spawn falkon");
    let stderr_rx = {
        let stderr = falkon_child.stderr.take().expect("falkon stderr");
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
    let mut saw_activated = false;
    let mut saw_kbd_focus = false;
    let mut saw_focus = false;
    let mut saw_click = false;
    let mut saw_commit = false;
    let mut activated: Option<String> = None;
    let mut hash_at_enable: Option<String> = None;
    let mut last_hash: Option<String> = None;
    let mut n_commits_after_osk = 0;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(25);
    while Instant::now() < deadline && !(saw_toplevel && saw_frame && saw_activated && saw_focus) {
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
                if let Some(id) = activated_surface_id(&line) {
                    saw_activated = true;
                    activated = Some(id.to_string());
                }
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                }
                if let Some(h) = activated_frame_hash(&line, &activated) {
                    saw_frame = true;
                    last_hash = Some(h.to_string());
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
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
                if let Some(h) = activated_frame_hash(&line, &activated) {
                    last_hash = Some(h.to_string());
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }
    if !(saw_toplevel
        && last_hash.is_some()
        && saw_activated
        && saw_focus
        && !saw_kbd_focus
        && !saw_enable)
    {
        reap_falkon(&mut falkon_child);
        let stderr = stderr_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap_or_default();
        let _ = displayd.kill();
        let _ = displayd.wait();
        panic!(
            "pre-click: expected framed Falkon Activated without enable; kbd={saw_kbd_focus} focus={saw_focus} xdg={saw_activated} enable={saw_enable} hash={last_hash:?}; displayd={:?}; stderr={stderr}",
            lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
        );
    }

    if let Some(stdin) = displayd.stdin.as_mut() {
        writeln!(stdin, "inject-click 640 20").expect("inject-click");
        let _ = stdin.flush();
    }
    let click_deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < click_deadline && !(saw_click && saw_enable && ime_state.activate) {
        match log.recv_timeout(Duration::from_millis(50)) {
            Ok(line) => {
                if line.contains("injected click") {
                    saw_click = true;
                }
                if line.contains("keyboard focus set") {
                    saw_kbd_focus = true;
                }
                if line.contains("text-input-v2 enable") {
                    saw_enable = true;
                }
                if let Some(h) = activated_frame_hash(&line, &activated) {
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
    if !(saw_click && saw_enable && !saw_kbd_focus) {
        reap_falkon(&mut falkon_child);
        let stderr = stderr_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap_or_default();
        let _ = displayd.kill();
        let _ = displayd.wait();
        panic!(
            "URL click 640 20 did not enable v2; click={saw_click} enable={saw_enable} kbd={saw_kbd_focus}; displayd={:?}; stderr={stderr}",
            lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
        );
    }
    let before = hash_at_enable
        .clone()
        .or(last_hash.clone())
        .expect("no shm on activated Falkon surface");

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
                if line.contains("text-input-v2 commit_string") {
                    saw_commit = true;
                }
                if line.contains("commit on surface") {
                    if activated_frame_hash(&line, &activated).is_some() {
                        n_commits_after_osk += 1;
                    }
                }
                if let Some(h) = activated_frame_hash(&line, &activated) {
                    last_hash = Some(h.to_string());
                }
                lines.push(line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        let _ = queue.roundtrip(&mut ime_state);
    }

    reap_falkon(&mut falkon_child);
    let stderr = stderr_rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap_or_default();
    let _ = displayd.kill();
    let _ = displayd.wait();
    let after = last_hash.as_deref().unwrap_or(before.as_str());
    assert!(
        saw_commit,
        "Falkon URL OSK immediately after enable; v2 commit_string missing; click={saw_click} enable={saw_enable} commits={n_commits_after_osk}; displayd={:?}; stderr={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    assert_eq!(
        n_commits_after_osk, 0,
        "Falkon URL OSK immediately after enable shm commit count; expected no toplevel redraw; commits={n_commits_after_osk} before={before} after={after}; displayd={:?}; stderr={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
    assert_eq!(
        after, before.as_str(),
        "Falkon URL OSK immediately after enable attached a new toplevel shm; before={before} after={after}; do not claim typed LocationBar; displayd={:?}; stderr={stderr}",
        lines.iter().filter(|l| interesting(l)).collect::<Vec<_>>()
    );
}
