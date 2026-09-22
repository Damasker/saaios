//! APP-02 / ADR-320: Alpine musl GTK 4.14.4 (`gtk4-demo`) maps an
//! xdg_toplevel on host `saai-displayd` under `qemu-aarch64-static`
//! and commits a hashed shm frame. This is the same 4.14.4 binary
//! that asked for height 2337935 on panther when bounds were `(0,0)`
//! (ADR-310). Host displayd now sends window bounds (ADR-311). Not a
//! panther flash.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

fn displayd_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_saai-displayd"))
}

fn gtk4_probe() -> PathBuf {
    std::env::var("GTK4_ALPINE_PROBE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/gtk4-probe"))
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
fn alpine_gtk414_commits_an_shm_frame_when_bounds_are_the_window() {
    let probe = gtk4_probe();
    let demo = probe.join("bin/gtk4-demo");
    let loader = probe.join("lib/ld-musl-aarch64.so.1");
    let xkb = probe.join("share/X11/xkb");
    assert!(
        demo.is_file(),
        "missing Alpine gtk4-demo at {} — pack /tmp/gtk4-probe from alpine-gtk4-sysroot",
        demo.display()
    );
    assert!(
        loader.is_file(),
        "missing musl loader at {}",
        loader.display()
    );
    assert!(xkb.is_dir(), "missing XKB_CONFIG_ROOT at {}", xkb.display());

    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    let (mut displayd, log) = spawn_displayd(runtime_dir.path());
    let socket_name = wait_for_socket(&log);
    let probe = probe.canonicalize().expect("canonicalize gtk4 probe");
    let demo = probe.join("bin/gtk4-demo");
    let path = std::env::var("PATH").unwrap_or_default();

    let mut gtk = Command::new("qemu-aarch64-static")
        .arg("-L")
        .arg(&probe)
        .arg(&demo)
        .arg("--run=dialog")
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
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline && !(saw_toplevel && saw_frame) {
        match log.recv_timeout(Duration::from_millis(200)) {
            Ok(line) => {
                assert!(
                    !line.contains("2337935") && !line.contains("1776831"),
                    "GTK 4.14.4 still asked for the ADR-310 garbage height: {line}"
                );
                if line.contains("new xdg_toplevel") {
                    assert!(
                        line.contains("1280x800 fullscreen=false"),
                        "GTK 4.14.4 must map the x86 windowed size, not a phone panel: {line}"
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

    let _ = gtk.kill();
    let _ = gtk.wait();
    let stderr = stderr_rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap_or_default();
    assert!(
        saw_toplevel,
        "Alpine GTK 4.14.4 never created an xdg_toplevel; displayd={lines:?}; stderr={stderr}"
    );
    assert!(
        saw_frame,
        "Alpine GTK 4.14.4 never committed a hashed shm frame; displayd={lines:?}; stderr={stderr}"
    );
    assert!(
        displayd
            .try_wait()
            .expect("failed to poll saai-displayd")
            .is_none(),
        "saai-displayd exited during the GTK 4.14.4 frame"
    );
    let _ = displayd.kill();
    let _ = displayd.wait();
}
