//! APP-02 / ADR-305: a real GTK4 client (GDK Wayland + GSK cairo) maps
//! an xdg_toplevel on host `saai-displayd` and commits a hashed shm
//! frame. This is the toolkit half of ADR-266/286, not a panther
//! frame. Alpine 4.14.4 musl still waits the next displayd experiment.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

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

#[test]
fn gtk4_commits_an_shm_frame_on_host_displayd() {
    assert!(
        gtk4_import_available(),
        "host GTK4 probe needs python3 + gir1.2-gtk-4.0 (Gtk 4.0)"
    );
    let probe = gtk4_probe();
    assert!(probe.is_file(), "missing GTK4 probe at {}", probe.display());

    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    let (mut displayd, log) = spawn_displayd(runtime_dir.path());
    let socket_name = wait_for_socket(&log);

    let mut gtk = Command::new("python3")
        .arg(&probe)
        .env("XDG_RUNTIME_DIR", runtime_dir.path())
        .env("WAYLAND_DISPLAY", &socket_name)
        .env("GDK_BACKEND", "wayland")
        .env("GSK_RENDERER", "cairo")
        .env("GTK_A11Y", "none")
        .env("NO_AT_BRIDGE", "1")
        .env_remove("DISPLAY")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn gtk4_hello.py");

    let mut saw_toplevel = false;
    let mut saw_frame = false;
    let mut lines = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline && !(saw_toplevel && saw_frame) {
        match log.recv_timeout(Duration::from_millis(200)) {
            Ok(line) => {
                assert!(
                    !line.contains("1776831"),
                    "ADR-025 uninitialized height leaked into displayd log: {line}"
                );
                if line.contains("new xdg_toplevel") {
                    assert!(
                        line.contains("1280x800 fullscreen=false"),
                        "GTK4 must map the x86 windowed size, not a phone panel: {line}"
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

    let gtk_deadline = Instant::now() + Duration::from_secs(8);
    let status = loop {
        if let Some(status) = gtk.try_wait().expect("failed to poll gtk4_hello.py") {
            break status;
        }
        assert!(
            Instant::now() < gtk_deadline,
            "GTK4 probe hung; displayd={lines:?}"
        );
        std::thread::sleep(Duration::from_millis(50));
    };
    let stderr = {
        let mut buf = String::new();
        if let Some(mut err) = gtk.stderr.take() {
            let _ = std::io::Read::read_to_string(&mut err, &mut buf);
        }
        buf
    };
    assert!(
        status.success(),
        "GTK4 probe exited {status:?}; stderr={stderr}; displayd={lines:?}"
    );
    assert!(
        saw_toplevel,
        "GTK4 never created an xdg_toplevel; displayd={lines:?}; stderr={stderr}"
    );
    assert!(
        saw_frame,
        "GTK4 never committed a hashed shm frame; displayd={lines:?}; stderr={stderr}"
    );
    assert!(
        displayd
            .try_wait()
            .expect("failed to poll saai-displayd")
            .is_none(),
        "saai-displayd exited during the GTK4 frame"
    );
    let _ = displayd.kill();
    let _ = displayd.wait();
}
