//! S02 headless integration test: spawns the real saai-displayd and
//! saai-demo-surface binaries as separate processes (never imports their
//! internals) and drives the same scenario verified manually during S02 --
//! normal lifecycle, frame-hash determinism, protocol-negative rejection,
//! and fault-injection resilience -- so CI exercises the actual vertical
//! slice, not just compilation.
//!
//! saai-demo-surface is a separate workspace package, not a dependency of
//! this one, so its binary path is derived from this crate's own
//! CARGO_BIN_EXE_saai-displayd rather than a CARGO_BIN_EXE_saai-demo-surface
//! that Cargo has no reason to set here; both land in the same target/
//! directory. This means the demo binary must already be built -- true for
//! `cargo test --workspace` (what CI runs), not guaranteed for
//! `cargo test -p saai-displayd` run in isolation.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

fn displayd_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_saai-displayd"))
}

fn demo_bin() -> PathBuf {
    displayd_bin().with_file_name(if cfg!(windows) {
        "saai-demo-surface.exe"
    } else {
        "saai-demo-surface"
    })
}

/// Spawns `cmd` (stdout/stderr already configured as piped by the caller)
/// and starts a background reader over its stdout into a channel, so the
/// caller can poll for specific lines without losing anything buffered
/// between separate short-lived reads on the same handle.
fn spawn_streaming(mut cmd: Command) -> (Child, mpsc::Receiver<String>) {
    let mut child = cmd.spawn().expect("failed to spawn process");
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

fn spawn_displayd(runtime_dir: &std::path::Path) -> (Child, mpsc::Receiver<String>) {
    let mut cmd = Command::new(displayd_bin());
    cmd.env("XDG_RUNTIME_DIR", runtime_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    spawn_streaming(cmd)
}

fn spawn_demo(
    runtime_dir: &std::path::Path,
    socket: &str,
    label: &str,
) -> (Child, mpsc::Receiver<String>) {
    let mut cmd = Command::new(demo_bin());
    cmd.env("XDG_RUNTIME_DIR", runtime_dir)
        .env("WAYLAND_DISPLAY", socket)
        .arg(label)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    spawn_streaming(cmd)
}

/// Drains whatever lines have arrived so far (non-blocking) plus anything
/// that arrives before `deadline`, stopping early once `predicate` matches.
fn collect_until(
    rx: &mpsc::Receiver<String>,
    deadline: Instant,
    predicate: impl Fn(&str) -> bool,
) -> Vec<String> {
    let mut lines = Vec::new();
    while let Ok(line) = rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        let matched = predicate(&line);
        lines.push(line);
        if matched {
            break;
        }
    }
    lines
}

fn run_demo(
    runtime_dir: &std::path::Path,
    socket: &str,
    label: &str,
    extra_arg: Option<&str>,
) -> (i32, String) {
    let mut cmd = Command::new(demo_bin());
    cmd.env("XDG_RUNTIME_DIR", runtime_dir)
        .env("WAYLAND_DISPLAY", socket)
        .arg(label)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(arg) = extra_arg {
        cmd.arg(arg);
    }
    let output = cmd.output().expect("failed to run saai-demo-surface");
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    (output.status.code().unwrap_or(-1), combined)
}

#[test]
fn headless_vertical_slice() {
    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    let (mut displayd, displayd_log) = spawn_displayd(runtime_dir.path());

    let deadline = Instant::now() + Duration::from_secs(10);
    let boot_lines = collect_until(&displayd_log, deadline, |l| l.contains("WAYLAND_DISPLAY="));
    let socket = boot_lines
        .iter()
        .find_map(|l| l.split("WAYLAND_DISPLAY=").nth(1))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| {
            panic!("displayd never printed its socket name; log so far: {boot_lines:?}")
        });

    // 1. Normal client lifecycle.
    let (code_a, log_a) = run_demo(runtime_dir.path(), &socket, "A", None);
    assert_eq!(code_a, 0, "client A did not exit cleanly:\n{log_a}");
    assert!(
        log_a.contains("committed 800x480 test pattern frame"),
        "client A never committed its frame:\n{log_a}"
    );

    // 2. Protocol-negative: a client attaching a buffer before any
    // configure must get a protocol error and disconnect cleanly, without
    // taking the compositor down.
    let (code_bad, log_bad) = run_demo(
        runtime_dir.path(),
        &socket,
        "BAD",
        Some("violate-configure"),
    );
    assert_eq!(
        code_bad, 0,
        "violating client should exit(0) after a clean protocol-error disconnect:\n{log_bad}"
    );
    assert!(
        log_bad.contains("connection ended after protocol violation"),
        "violating client did not observe a protocol error:\n{log_bad}"
    );

    assert!(
        displayd
            .try_wait()
            .expect("failed to poll displayd")
            .is_none(),
        "compositor must still be running after a malformed client"
    );

    // 3. Fault-injection resilience: a normal client still works on the
    // same compositor instance afterward.
    let (code_b, log_b) = run_demo(runtime_dir.path(), &socket, "B", None);
    assert_eq!(
        code_b, 0,
        "client B did not exit cleanly after the bad client:\n{log_b}"
    );
    assert!(
        log_b.contains("committed 800x480 test pattern frame"),
        "client B never committed its frame:\n{log_b}"
    );

    // 4. Determinism: pull both frame hashes from the compositor's own log
    // and confirm the identical test pattern hashed identically across two
    // independent client processes.
    let deadline2 = Instant::now() + Duration::from_secs(5);
    let mut rest = collect_until(&displayd_log, deadline2, |_| false);
    let mut all_lines = boot_lines;
    all_lines.append(&mut rest);

    if let Some(mut stdin) = displayd.stdin.take() {
        let _ = writeln!(stdin, "inject-key");
    }
    std::thread::sleep(Duration::from_millis(200));
    displayd.kill().ok();
    displayd.wait().ok();

    let hashes: Vec<&str> = all_lines
        .iter()
        .filter_map(|l| l.split("frame sha256=").nth(1))
        .collect();
    assert_eq!(
        hashes.len(),
        2,
        "expected exactly two hashed frames (A and B), compositor log:\n{all_lines:?}"
    );
    assert_eq!(
        hashes[0], hashes[1],
        "the same deterministic test pattern must hash identically across independent clients"
    );
}

/// S02 acceptance: "синтетический input доставляется только focused
/// client". Client A commits first and keeps polling for up to its own
/// deadline, giving a window to inject a key while it still holds focus;
/// client B commits afterward and must never see it.
#[test]
fn focused_input_delivery() {
    let runtime_dir = tempfile::tempdir().expect("failed to create XDG_RUNTIME_DIR");
    let (mut displayd, displayd_log) = spawn_displayd(runtime_dir.path());

    let boot_deadline = Instant::now() + Duration::from_secs(10);
    let boot_lines = collect_until(&displayd_log, boot_deadline, |l| {
        l.contains("WAYLAND_DISPLAY=")
    });
    let socket = boot_lines
        .iter()
        .find_map(|l| l.split("WAYLAND_DISPLAY=").nth(1))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| {
            panic!("displayd never printed its socket name; log so far: {boot_lines:?}")
        });

    let (mut client_a, log_a) = spawn_demo(runtime_dir.path(), &socket, "A");
    let a_deadline = Instant::now() + Duration::from_secs(10);
    let a_committed = collect_until(&log_a, a_deadline, |l| {
        l.contains("committed 800x480 test pattern frame")
    });
    assert!(
        a_committed
            .iter()
            .any(|l| l.contains("committed 800x480 test pattern frame")),
        "client A never committed while still in its input-wait window: {a_committed:?}"
    );

    // A is now focused and inside its own bounded wait for a key event;
    // inject one before that window closes.
    if let Some(mut stdin) = displayd.stdin.take() {
        writeln!(stdin, "inject-key").expect("failed to write inject-key to displayd stdin");
    }

    let a_status = client_a.wait().expect("failed to wait on client A");
    assert!(
        a_status.success(),
        "client A did not exit cleanly: {a_status:?}"
    );
    let a_rest = collect_until(&log_a, Instant::now() + Duration::from_secs(1), |_| false);
    let a_log: Vec<String> = a_committed.into_iter().chain(a_rest).collect();
    assert!(
        a_log
            .iter()
            .any(|l| l.contains("observed the injected key event")),
        "focused client A never observed the injected key: {a_log:?}"
    );

    // B commits after A already holds focus, so it must never see the key.
    let (code_b, log_b) = {
        let (mut child, log_rx) = spawn_demo(runtime_dir.path(), &socket, "B");
        let status = child.wait().expect("failed to wait on client B");
        let lines: Vec<String> =
            collect_until(&log_rx, Instant::now() + Duration::from_secs(1), |_| false);
        (status.code().unwrap_or(-1), lines.join("\n"))
    };
    assert_eq!(code_b, 0, "client B did not exit cleanly:\n{log_b}");
    assert!(
        !log_b.contains("received key event"),
        "unfocused client B must never receive the synthetic key:\n{log_b}"
    );

    displayd.kill().ok();
    displayd.wait().ok();
}
