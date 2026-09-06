#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
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

fn workspace_binary(name: &str) -> PathBuf {
    let own_binary = PathBuf::from(env!("CARGO_BIN_EXE_saai-displayd"));
    let candidate = own_binary.with_file_name(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    if !candidate.is_file() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("displayd belongs to workspace/services");
        let status = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
            .args(["build", "--quiet", "--locked", "-p", name, "--bin", name])
            .current_dir(workspace)
            .status()
            .expect("build workspace client binary");
        assert!(status.success(), "could not build {name}");
    }
    assert!(
        candidate.is_file(),
        "{name} was not built next to {}",
        own_binary.display()
    );
    candidate
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

#[test]
fn shell_can_exit_and_restart_with_the_same_frame() {
    let runtime = tempfile::tempdir().unwrap();
    fs::set_permissions(runtime.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let shell_binary = workspace_binary("saai-shell");
    let socket = "wayland-saai-shell-integration";
    let report = runtime.path().join("shell-report.txt");
    let mut server = Command::new(env!("CARGO_BIN_EXE_saai-displayd"))
        .args(["--socket", socket, "--expected-successes", "2", "--report"])
        .arg(&report)
        .env("XDG_RUNTIME_DIR", runtime.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start compositor");
    wait_for_socket(&runtime.path().join(socket));

    for launch in 1..=2 {
        let output = Command::new(&shell_binary)
            .env("XDG_RUNTIME_DIR", runtime.path())
            .env("WAYLAND_DISPLAY", socket)
            .output()
            .expect("run shell process");
        assert!(
            output.status.success(),
            "shell launch {launch} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains(&format!(
                "SHELL_OK configured=true transport=wl_shm hash={}",
                saai_shell::frame_hash(&saai_shell::shell_frame())
            )),
            "shell launch {launch} did not report the deterministic frame"
        );
        if launch == 1 {
            assert!(
                server.try_wait().unwrap().is_none(),
                "compositor exited instead of accepting a restarted shell"
            );
        }
    }

    let started = Instant::now();
    let status = loop {
        if let Some(status) = server.try_wait().unwrap() {
            break status;
        }
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "compositor did not stop after restarted shell"
        );
        thread::sleep(Duration::from_millis(10));
    };
    assert!(status.success());

    let evidence = fs::read_to_string(report).unwrap();
    assert!(evidence.contains(
        "BACKEND wl_shm=true dmabuf_available=false fallback=Some(DmabufCapabilityUnavailable)"
    ));
    let expected_frame = format!(
        "FRAME hash={} width=64 height=48",
        saai_shell::frame_hash(&saai_shell::shell_frame())
    );
    assert_eq!(evidence.matches(&expected_frame).count(), 2);
    assert_eq!(evidence.matches("CLIENT_DISCONNECTED").count(), 2);
}
