#![cfg(unix)]

use saai_displayd::{expected_frame_hash, HeadlessReport};
use serde_json::Value;
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    os::unix::net::UnixStream,
    path::Path,
    process::{Child, Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

fn wait_for_socket(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !path.exists() {
        assert!(
            Instant::now() < deadline,
            "Wayland socket did not appear at {}",
            path.display()
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn assert_success(name: &str, output: &Output) {
    assert!(
        output.status.success(),
        "{name} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn wait_with_captured_stdout(mut child: Child, mut stdout: BufReader<impl Read>) -> Output {
    let status = child.wait().expect("wait for observer");
    let mut captured = Vec::new();
    stdout
        .read_to_end(&mut captured)
        .expect("read observer output");
    Output {
        status,
        stdout: captured,
        stderr: Vec::new(),
    }
}

fn assert_server_running(server: &mut Child, stage: &str) {
    if let Some(status) = server.try_wait().expect("query server status") {
        let mut stderr = String::new();
        server
            .stderr
            .as_mut()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        panic!("saai-displayd exited after {stage}: {status}\n{stderr}");
    }
}

#[test]
fn real_wayland_socket_survives_bad_client_and_routes_focus() {
    let runtime = tempfile::tempdir().unwrap();
    let socket_name = "wayland-saaios-integration";
    let socket_path = runtime.path().join(socket_name);
    let report_path = runtime.path().join("report.json");

    let mut server = Command::new(env!("CARGO_BIN_EXE_saai-displayd"))
        .args([
            "--socket",
            socket_name,
            "--report",
            report_path.to_str().unwrap(),
            "--timeout-ms",
            "5000",
        ])
        .env("XDG_RUNTIME_DIR", runtime.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start saai-displayd");
    wait_for_socket(&socket_path);

    let mut observer = Command::new(env!("CARGO_BIN_EXE_saai-demo-surface"))
        .arg("--observer")
        .env("XDG_RUNTIME_DIR", runtime.path())
        .env("WAYLAND_DISPLAY", socket_name)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start observer client");
    let mut observer_stdout = BufReader::new(observer.stdout.take().unwrap());
    let mut ready = String::new();
    observer_stdout
        .read_line(&mut ready)
        .expect("read observer readiness");
    assert_eq!(ready.trim(), "CLIENT_READY observer");

    let crashed = Command::new(env!("CARGO_BIN_EXE_saai-demo-surface"))
        .arg("--crash-before-buffer")
        .env("XDG_RUNTIME_DIR", runtime.path())
        .env("WAYLAND_DISPLAY", socket_name)
        .output()
        .expect("run crash fixture");
    assert_eq!(crashed.status.code(), Some(42));
    thread::sleep(Duration::from_millis(25));
    assert_server_running(&mut server, "client crash before buffer");

    let mut malformed = UnixStream::connect(&socket_path).expect("connect malformed client");
    malformed
        .write_all(&[0, 0, 0, 0, 0, 0, 0, 0])
        .expect("send malformed Wayland header");
    drop(malformed);
    thread::sleep(Duration::from_millis(25));
    assert_server_running(&mut server, "malformed wire client");

    let unconfigured = Command::new(env!("CARGO_BIN_EXE_saai-demo-surface"))
        .arg("--unconfigured-buffer")
        .env("XDG_RUNTIME_DIR", runtime.path())
        .env("WAYLAND_DISPLAY", socket_name)
        .output()
        .expect("run unconfigured-buffer client");
    assert_success("unconfigured-buffer client", &unconfigured);
    assert!(
        String::from_utf8_lossy(&unconfigured.stdout)
            .contains("unconfigured_buffer_disconnected=true"),
        "server must reject a buffer committed before configure ack"
    );
    assert_server_running(&mut server, "unconfigured-buffer client");

    let duplicate_role = Command::new(env!("CARGO_BIN_EXE_saai-demo-surface"))
        .arg("--duplicate-role")
        .env("XDG_RUNTIME_DIR", runtime.path())
        .env("WAYLAND_DISPLAY", socket_name)
        .output()
        .expect("run duplicate-role client");
    assert_success("duplicate-role client", &duplicate_role);
    assert!(
        String::from_utf8_lossy(&duplicate_role.stdout)
            .contains("duplicate_role_disconnected=true"),
        "server must reject a second xdg role for one wl_surface"
    );
    assert_server_running(&mut server, "duplicate-role client");

    let surface = Command::new(env!("CARGO_BIN_EXE_saai-demo-surface"))
        .arg("--reuse-role")
        .env("XDG_RUNTIME_DIR", runtime.path())
        .env("WAYLAND_DISPLAY", socket_name)
        .output()
        .expect("run surface client");
    assert_success("surface client", &surface);

    let observer_output = wait_with_captured_stdout(observer, observer_stdout);
    assert_success("observer client", &observer_output);

    let server_output = server.wait_with_output().expect("wait for saai-displayd");
    assert_success("saai-displayd", &server_output);

    let surface_stdout = String::from_utf8(surface.stdout).unwrap();
    let surface_result: Value = serde_json::from_str(
        surface_stdout
            .lines()
            .find_map(|line| line.strip_prefix("CLIENT_RESULT "))
            .expect("surface result"),
    )
    .unwrap();
    assert_eq!(surface_result["configured"], true);
    assert_eq!(surface_result["pointer_buttons"], 2);
    assert_eq!(surface_result["close_events"], 2);
    assert_eq!(surface_result["close_received"], true);

    let observer_stdout = String::from_utf8(observer_output.stdout).unwrap();
    let observer_result: Value = serde_json::from_str(
        observer_stdout
            .lines()
            .find_map(|line| line.strip_prefix("CLIENT_RESULT "))
            .expect("observer result"),
    )
    .unwrap();
    assert_eq!(observer_result["pointer_enters"], 0);
    assert_eq!(observer_result["pointer_buttons"], 0);

    let report: HeadlessReport =
        serde_json::from_slice(&fs::read(report_path).expect("read server report")).unwrap();
    assert!(report.configured);
    assert_eq!(report.frame_hash, expected_frame_hash());
    assert_eq!(report.input_events_sent, 1);
    assert!(report.close_sent);
    assert!(
        report.disconnected_clients >= 5,
        "crashed, surface, malformed, unconfigured, and duplicate-role clients should disconnect"
    );
}
