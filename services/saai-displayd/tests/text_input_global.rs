//! S08 Change 3 / ADR-022: `zwp_text_input_manager_v3` advertised, and a
//! client can bind + enable text input without the compositor crashing.
//!
//! Unlike `zwp_input_method_manager_v2` (deliberately NOT wired up -- see
//! ADR-022: its `GetInputMethod` handler unconditionally requires a
//! working keyboard, and a spike proved no keymap compiles on this device
//! at all, not even a fully self-contained one), text-input-v3's
//! client-facing half only touches per-seat user data, confirmed by
//! reading smithay's own request handler. This test is the automated
//! guard for that: spawns the real `saai-displayd` binary, binds
//! `zwp_text_input_manager_v3`, requests a `zwp_text_input_v3` for the
//! advertised seat, and calls `enable()` + `commit()` -- if the compositor
//! were to crash (the ADR-022 failure mode), the roundtrip below would
//! never return and the test would time out instead of passing.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use wayland_client::{
    globals::{registry_queue_init, GlobalListContents},
    protocol::{wl_registry, wl_seat},
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols::wp::text_input::zv3::client::{
    zwp_text_input_manager_v3::ZwpTextInputManagerV3, zwp_text_input_v3::ZwpTextInputV3,
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

struct ProbeState {
    seat: Option<wl_seat::WlSeat>,
    text_input_manager: Option<ZwpTextInputManagerV3>,
}

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

impl Dispatch<wl_seat::WlSeat, ()> for ProbeState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_seat::WlSeat,
        _event: wl_seat::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpTextInputManagerV3, ()> for ProbeState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpTextInputManagerV3,
        _event: <ZwpTextInputManagerV3 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpTextInputV3, ()> for ProbeState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpTextInputV3,
        _event: <ZwpTextInputV3 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

#[test]
fn text_input_v3_binds_and_enables_without_crashing_compositor() {
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
    let (globals, mut queue) =
        registry_queue_init::<ProbeState>(&conn).expect("registry init failed");
    let qh = queue.handle();

    let mut state = ProbeState {
        seat: None,
        text_input_manager: None,
    };
    state.seat = Some(
        globals
            .bind(&qh, 1..=9, ())
            .expect("wl_seat not advertised"),
    );
    state.text_input_manager = Some(
        globals
            .bind(&qh, 1..=1, ())
            .expect("zwp_text_input_manager_v3 not advertised -- ADR-022 regression"),
    );

    let text_input = state.text_input_manager.as_ref().unwrap().get_text_input(
        state.seat.as_ref().unwrap(),
        &qh,
        (),
    );
    text_input.enable();
    text_input.commit();

    // If the compositor had crashed handling any of the above (the exact
    // ADR-022 failure mode for input-method-v2), this roundtrip would
    // never return -- it would hang until the outer test harness's own
    // timeout, not fail cleanly. A real reply confirms the process is
    // still alive and processed the requests.
    queue
        .roundtrip(&mut state)
        .expect("compositor did not survive enable()/commit() on a text-input-v3 object");

    assert!(
        displayd
            .try_wait()
            .expect("failed to poll saai-displayd")
            .is_none(),
        "saai-displayd exited unexpectedly during the text-input-v3 roundtrip"
    );

    let _ = displayd.kill();
    let _ = displayd.wait();
}
