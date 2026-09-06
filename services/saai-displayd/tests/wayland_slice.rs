#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

fn wait_for_socket(path: &Path) {
    let started = Instant::now();
    while !path.exists() {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "Wayland socket was not created"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn client(runtime: &Path, socket: &str, mode: Option<&str>) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_saai-demo-surface"));
    command
        .env("XDG_RUNTIME_DIR", runtime)
        .env("WAYLAND_DISPLAY", socket);
    if let Some(mode) = mode {
        command.arg(mode);
    }
    command.output().expect("run demo client")
}

#[test]
fn malformed_and_crashed_clients_do_not_break_the_compositor() {
    let runtime = tempfile::tempdir().unwrap();
    fs::set_permissions(runtime.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let socket = "wayland-saai-integration";
    let report = runtime.path().join("report.txt");
    let mut server = Command::new(env!("CARGO_BIN_EXE_saai-displayd"))
        .args(["--socket", socket, "--expected-successes", "1", "--report"])
        .arg(&report)
        .env("XDG_RUNTIME_DIR", runtime.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start compositor");
    wait_for_socket(&runtime.path().join(socket));

    for mode in ["--invalid-serial", "--invalid-buffer", "--invalid-role"] {
        let output = client(runtime.path(), socket, Some(mode));
        assert!(
            output.status.success(),
            "{mode} fixture failed unexpectedly: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            server.try_wait().unwrap().is_none(),
            "server exited after {mode}"
        );
    }

    let crashed = client(runtime.path(), socket, Some("--crash"));
    assert_eq!(crashed.status.code(), Some(23));
    assert!(
        server.try_wait().unwrap().is_none(),
        "server exited after crashed client"
    );

    let healthy = client(runtime.path(), socket, None);
    assert!(
        healthy.status.success(),
        "healthy client failed: {}",
        String::from_utf8_lossy(&healthy.stderr)
    );
    assert!(String::from_utf8_lossy(&healthy.stdout)
        .contains("CLIENT_OK configure=true input=true close=true"));

    let started = Instant::now();
    let status = loop {
        if let Some(status) = server.try_wait().unwrap() {
            break status;
        }
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "compositor did not stop after healthy client"
        );
        thread::sleep(Duration::from_millis(10));
    };
    assert!(status.success());
    let evidence = fs::read_to_string(report).unwrap();
    assert!(evidence.contains("CONFIGURE serial="));
    assert!(evidence
        .contains("FRAME hash=9b05ff34424f63e620a88baecc12950fe13e242ca1645ec9d8d57d939186f99d"));
    assert!(evidence.contains("INPUT key=30 focused=true"));
    assert!(evidence.contains("CLIENT_DISCONNECTED"));
}
