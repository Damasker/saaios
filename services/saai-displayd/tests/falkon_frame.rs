//! APP-06 / ADR-306: packaged aarch64 Falkon maps an xdg_toplevel on
//! host `saai-displayd` under `qemu-aarch64-static` and commits a
//! hashed shm frame. Not a panther `appd` install. WebEngine helper
//! spawn via binfmt is host-only and may fail; the Widgets chrome
//! frame is the hello-frame.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

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

    let _ = falkon_child.kill();
    let _ = falkon_child.wait();
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
