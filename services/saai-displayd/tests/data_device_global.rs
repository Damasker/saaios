//! S08 Change 2 / ADR-021: `wl_data_device_manager` must be advertised.
//!
//! GTK4's `_gdk_wayland_display_open()` refuses to open a display at all
//! unless this global is present alongside `wl_compositor`/`wl_shm`
//! (confirmed against a real Alpine-built `gtk4-demo` binary in the
//! ADR-021 spike -- not part of this crate's own automated suite, since it
//! needs qemu-user-static and an Alpine sysroot neither CI nor this repo
//! provide). This test is the automated guard against silently losing that
//! global again: spawns the real `saai-displayd` binary, connects a plain
//! `wayland-client`, and asserts the global shows up in the registry --
//! cheap enough to run in every `cargo test`, unlike the full GTK4 spike.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use wayland_client::{
    globals::{registry_queue_init, GlobalListContents},
    protocol::wl_registry,
    Connection, Dispatch, QueueHandle,
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

struct ProbeState;

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

#[test]
fn advertises_wl_data_device_manager() {
    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    let (mut displayd, displayd_log) = spawn_displayd(runtime_dir.path());

    let deadline = Instant::now() + Duration::from_secs(10);
    let socket_name = loop {
        assert!(
            Instant::now() < deadline,
            "saai-displayd never announced a listening socket"
        );
        match displayd_log.recv_timeout(Duration::from_millis(200)) {
            Ok(line) => {
                if let Some(name) =
                    line.strip_prefix("saai-displayd: listening on WAYLAND_DISPLAY=")
                {
                    break name.to_string();
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                panic!("saai-displayd exited before announcing a socket")
            }
        }
    };

    // SAFETY: this test process does not touch these vars from another
    // thread concurrently -- Connection::connect_to_env() reads them once,
    // synchronously, right below.
    unsafe {
        std::env::set_var("XDG_RUNTIME_DIR", runtime_dir.path());
        std::env::set_var("WAYLAND_DISPLAY", &socket_name);
    }
    let conn = Connection::connect_to_env().expect("failed to connect to saai-displayd");
    let (globals, _queue) = registry_queue_init::<ProbeState>(&conn).expect("registry init failed");

    let has_data_device_manager = globals.contents().with_list(|list| {
        list.iter()
            .any(|global| global.interface == "wl_data_device_manager")
    });

    let _ = displayd.kill();
    let _ = displayd.wait();

    assert!(
        has_data_device_manager,
        "wl_data_device_manager not advertised -- GTK4 clients will refuse to open a display \
         (ADR-021, _gdk_wayland_display_open())"
    );
}
