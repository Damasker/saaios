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
    protocol::{
        wl_data_device, wl_data_device_manager, wl_data_offer, wl_data_source, wl_registry, wl_seat,
    },
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

struct ClipboardProbe {
    cancelled: bool,
    offers: usize,
}

macro_rules! empty_clipboard_dispatch {
    ($ty:ty) => {
        impl Dispatch<$ty, ()> for ClipboardProbe {
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

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for ClipboardProbe {
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

empty_clipboard_dispatch!(wl_data_device_manager::WlDataDeviceManager);
empty_clipboard_dispatch!(wl_seat::WlSeat);
empty_clipboard_dispatch!(wl_data_offer::WlDataOffer);

impl Dispatch<wl_data_source::WlDataSource, ()> for ClipboardProbe {
    fn event(
        state: &mut Self,
        _proxy: &wl_data_source::WlDataSource,
        event: wl_data_source::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if matches!(event, wl_data_source::Event::Cancelled) {
            state.cancelled = true;
        }
    }
}

impl Dispatch<wl_data_device::WlDataDevice, ()> for ClipboardProbe {
    fn event(
        state: &mut Self,
        _proxy: &wl_data_device::WlDataDevice,
        event: wl_data_device::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if matches!(event, wl_data_device::Event::DataOffer { .. }) {
            state.offers += 1;
        }
    }
}

fn connect_clipboard() -> (
    Connection,
    wayland_client::EventQueue<ClipboardProbe>,
    wayland_client::globals::GlobalList,
) {
    let conn = Connection::connect_to_env().expect("failed to connect to saai-displayd");
    let (globals, queue) =
        registry_queue_init::<ClipboardProbe>(&conn).expect("registry init failed");
    (conn, queue, globals)
}

#[test]
fn data_device_set_selection_is_cancelled_and_never_offered() {
    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    let (mut displayd, displayd_log) = spawn_displayd(runtime_dir.path());
    let socket_name = wait_for_socket(&displayd_log);

    unsafe {
        std::env::set_var("XDG_RUNTIME_DIR", runtime_dir.path());
        std::env::set_var("WAYLAND_DISPLAY", &socket_name);
    }

    let (_writer_conn, mut writer_queue, writer_globals) = connect_clipboard();
    let writer_qh = writer_queue.handle();
    let writer_manager = writer_globals
        .bind::<wl_data_device_manager::WlDataDeviceManager, _, _>(&writer_qh, 1..=3, ())
        .expect("writer data device manager");
    let writer_seat = writer_globals
        .bind::<wl_seat::WlSeat, _, _>(&writer_qh, 1..=9, ())
        .expect("writer seat");
    let writer_device = writer_manager.get_data_device(&writer_seat, &writer_qh, ());
    let source = writer_manager.create_data_source(&writer_qh, ());
    source.offer("text/plain".into());
    writer_device.set_selection(Some(&source), 0);

    let mut writer = ClipboardProbe {
        cancelled: false,
        offers: 0,
    };
    writer_queue
        .roundtrip(&mut writer)
        .expect("writer roundtrip");

    let (_reader_conn, mut reader_queue, reader_globals) = connect_clipboard();
    let reader_qh = reader_queue.handle();
    let reader_manager = reader_globals
        .bind::<wl_data_device_manager::WlDataDeviceManager, _, _>(&reader_qh, 1..=3, ())
        .expect("reader data device manager");
    let reader_seat = reader_globals
        .bind::<wl_seat::WlSeat, _, _>(&reader_qh, 1..=9, ())
        .expect("reader seat");
    let _reader_device = reader_manager.get_data_device(&reader_seat, &reader_qh, ());
    let mut reader = ClipboardProbe {
        cancelled: false,
        offers: 0,
    };
    for _ in 0..3 {
        reader_queue
            .roundtrip(&mut reader)
            .expect("reader roundtrip");
        writer_queue
            .roundtrip(&mut writer)
            .expect("writer follow-up");
    }

    let _ = displayd.kill();
    let _ = displayd.wait();

    assert!(
        writer.cancelled,
        "native SetSelection must be cancelled so x86 keyboard cannot open ungated clipboard"
    );
    assert_eq!(
        reader.offers, 0,
        "no wl_data_offer — Receive has nothing to read (ADR-023/294)"
    );
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
