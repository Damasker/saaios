#![cfg(unix)]

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use saai_appd::{LifecycleEvent, LifecycleEventKind, ResponseResult, ServerMessage};
use serde_json::{json, Value};
use tempfile::TempDir;

const APP_ID: &str = "org.saaios.example.ipc-demo";

struct DaemonGuard {
    child: Child,
}

impl Drop for DaemonGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

struct Client {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
}

impl Client {
    fn connect(socket: &Path) -> Self {
        let stream = UnixStream::connect(socket).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        Self {
            writer: stream.try_clone().unwrap(),
            reader: BufReader::new(stream),
        }
    }

    fn request(&mut self, request_id: &str, request: Value) -> ResponseResult {
        serde_json::to_writer(&mut self.writer, &request).unwrap();
        self.writer.write_all(b"\n").unwrap();
        self.writer.flush().unwrap();
        loop {
            match self.read_message() {
                ServerMessage::Response {
                    request_id: received,
                    ok,
                    result,
                    error,
                    ..
                } if received == request_id => {
                    assert!(ok, "request failed: {error:?}");
                    return result.expect("successful response must carry a result");
                }
                ServerMessage::Event { .. } | ServerMessage::Response { .. } => continue,
            }
        }
    }

    fn event(&mut self, expected: LifecycleEventKind) -> LifecycleEvent {
        loop {
            if let ServerMessage::Event { event, .. } = self.read_message() {
                if event.event == expected {
                    return event;
                }
            }
        }
    }

    fn read_message(&mut self) -> ServerMessage {
        let mut line = String::new();
        self.reader.read_line(&mut line).unwrap();
        assert!(!line.is_empty(), "daemon closed the IPC connection");
        serde_json::from_str(&line).unwrap()
    }
}

#[test]
fn daemon_and_demo_complete_install_launch_stop_remove_flow() {
    let temporary = TempDir::new().unwrap();
    let data_root = temporary.path().join("saaios");
    let runtime_dir = temporary.path().join("runtime");
    let socket = temporary.path().join("run/appd.sock");
    let package = make_package(temporary.path());
    fs::create_dir(&runtime_dir).unwrap();

    let child = Command::new(env!("CARGO_BIN_EXE_saai-appd"))
        .arg("--data-root")
        .arg(&data_root)
        .arg("--socket")
        .arg(&socket)
        .arg("--runtime-dir")
        .arg(&runtime_dir)
        .arg("--wayland-display")
        .arg("wayland-test")
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let _daemon = DaemonGuard { child };
    wait_for_socket(&socket);

    let mut controller = Client::connect(&socket);
    let mut observer = Client::connect(&socket);
    assert!(matches!(
        observer.request(
            "observer-ready",
            json!({"schema": 1, "request_id": "observer-ready", "command": "list"}),
        ),
        ResponseResult::List { .. }
    ));

    let installed = controller.request(
        "install",
        json!({
            "schema": 1,
            "request_id": "install",
            "command": "install",
            "package_path": package,
        }),
    );
    assert!(matches!(
        installed,
        ResponseResult::Installed { ref app } if app.id == APP_ID && app.state == "installed"
    ));
    assert_eq!(observer.event(LifecycleEventKind::Installed).app_id, APP_ID);

    let listed = controller.request(
        "list-one",
        json!({"schema": 1, "request_id": "list-one", "command": "list"}),
    );
    assert!(matches!(
        listed,
        ResponseResult::List { ref apps } if apps.len() == 1 && apps[0].id == APP_ID
    ));

    let launched = controller.request(
        "launch-one",
        json!({"schema": 1, "request_id": "launch-one", "command": "launch", "app_id": APP_ID}),
    );
    let first_pid = match launched {
        ResponseResult::Launched {
            pid,
            existing: false,
            ..
        } => pid,
        other => panic!("unexpected launch response: {other:?}"),
    };
    let running = observer.event(LifecycleEventKind::Running);
    assert_eq!(running.app_id, APP_ID);
    assert_eq!(running.pid, Some(first_pid));
    assert!(matches!(
        controller.request(
            "list-running",
            json!({"schema": 1, "request_id": "list-running", "command": "list"}),
        ),
        ResponseResult::List { ref apps }
            if apps.len() == 1 && apps[0].state == "running" && apps[0].pids == [first_pid]
    ));

    let launched_again = controller.request(
        "launch-existing",
        json!({"schema": 1, "request_id": "launch-existing", "command": "launch", "app_id": APP_ID}),
    );
    assert!(matches!(
        launched_again,
        ResponseResult::Launched { pid, existing: true, .. } if pid == first_pid
    ));

    let app_data = data_root.join("var/apps").join(APP_ID);
    fs::write(app_data.join("keep"), "persistent-data").unwrap();
    assert!(matches!(
        controller.request(
            "stop",
            json!({"schema": 1, "request_id": "stop", "command": "stop", "app_id": APP_ID}),
        ),
        ResponseResult::Stopped {
            was_running: true,
            ..
        }
    ));
    assert_eq!(observer.event(LifecycleEventKind::Stopped).app_id, APP_ID);

    assert!(matches!(
        controller.request(
            "remove",
            json!({"schema": 1, "request_id": "remove", "command": "remove", "app_id": APP_ID}),
        ),
        ResponseResult::Removed { removed: true, .. }
    ));
    assert_eq!(observer.event(LifecycleEventKind::Removed).app_id, APP_ID);
    assert_eq!(
        fs::read_to_string(app_data.join("keep")).unwrap(),
        "persistent-data"
    );

    assert!(matches!(
        controller.request(
            "list-empty",
            json!({"schema": 1, "request_id": "list-empty", "command": "list"}),
        ),
        ResponseResult::List { apps } if apps.is_empty()
    ));
}

fn make_package(parent: &Path) -> PathBuf {
    let root = parent.join("package");
    fs::create_dir_all(root.join("bin")).unwrap();
    fs::write(
        root.join("manifest.toml"),
        format!(
            "schema = 1\nid = \"{APP_ID}\"\nname = \"IPC demo\"\nexec = \"bin/demo\"\nversion = \"0.1.0\"\nui = \"wayland\"\nsingle_instance = true\ncapabilities = []\n"
        ),
    )
    .unwrap();
    let executable = root.join("bin/demo");
    fs::write(&executable, "#!/bin/sh\nsleep 30\n").unwrap();
    let mut permissions = fs::metadata(&executable).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(executable, permissions).unwrap();
    root
}

fn wait_for_socket(socket: &Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !socket.exists() {
        assert!(Instant::now() < deadline, "daemon socket was not created");
        thread::sleep(Duration::from_millis(10));
    }
}
