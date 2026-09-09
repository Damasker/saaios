#![cfg(unix)]

use saai_entity_protocol::{EntitydEvent, ResponseResult, ServerMessage};
use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

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
        match self.request_message(request) {
            ServerMessage::Response {
                request_id: received,
                ok: true,
                result: Some(result),
                ..
            } if received == request_id => *result,
            other => panic!("unexpected response: {other:?}"),
        }
    }

    fn request_message(&mut self, request: Value) -> ServerMessage {
        serde_json::to_writer(&mut self.writer, &request).unwrap();
        self.writer.write_all(b"\n").unwrap();
        self.writer.flush().unwrap();
        loop {
            match self.read_message() {
                response @ ServerMessage::Response { .. } => return response,
                ServerMessage::Event { .. } => continue,
            }
        }
    }

    fn event(&mut self) -> EntitydEvent {
        loop {
            if let ServerMessage::Event { event, .. } = self.read_message() {
                return event;
            }
        }
    }

    fn read_message(&mut self) -> ServerMessage {
        let mut line = String::new();
        self.reader.read_line(&mut line).unwrap();
        assert!(!line.is_empty(), "daemon closed IPC connection");
        serde_json::from_str(&line).unwrap()
    }
}

#[test]
fn daemon_scopes_crud_selection_events_and_restart_recovery() {
    let temp = TempDir::new().unwrap();
    let store = temp.path().join("entities");
    let legacy = temp.path().join("active-space");
    let socket = temp.path().join("run/entityd.sock");
    fs::write(&legacy, b"1\n").unwrap();

    let daemon = spawn_daemon(&store, &legacy, &socket);
    let mut home_client = Client::connect(&socket);
    let mut work_client = Client::connect(&socket);
    let mut observer = Client::connect(&socket);
    assert!(matches!(
        observer.request(
            "subscribe",
            json!({"schema":1,"request_id":"subscribe","command":"subscribe"}),
        ),
        ResponseResult::Subscribed
    ));

    assert!(matches!(
        home_client.request(
            "spaces",
            json!({"schema":1,"request_id":"spaces","command":"list_spaces"}),
        ),
        ResponseResult::Spaces { spaces } if spaces.len() == 4
    ));
    assert!(matches!(
        home_client.request(
            "selection",
            json!({"schema":1,"request_id":"selection","command":"get_selection"}),
        ),
        ResponseResult::Selection { selection } if selection.space_id == "work"
    ));

    let home_entity = created_entity(home_client.request(
        "create-home",
        json!({
            "schema":1,"request_id":"create-home","command":"create_entity",
            "space_id":"home","entity_type":"task.item","title":"Домашняя задача",
            "properties":{"done":false}
        }),
    ));
    assert!(matches!(
        observer.event(),
        EntitydEvent::EntityChanged { record } if record.space_id == "home"
    ));

    let work_entity = created_entity(work_client.request(
        "create-work",
        json!({
            "schema":1,"request_id":"create-work","command":"create_entity",
            "space_id":"work","entity_type":"task.item","title":"Рабочая задача"
        }),
    ));
    assert!(matches!(
        observer.event(),
        EntitydEvent::EntityChanged { record } if record.space_id == "work"
    ));

    let listed_home = home_client.request(
        "list-home",
        json!({"schema":1,"request_id":"list-home","command":"list_entities","space_id":"home"}),
    );
    assert!(matches!(
        listed_home,
        ResponseResult::Entities { space_id, entities }
            if space_id == "home" && entities.len() == 1
                && entities[0].id == home_entity.id && entities[0].id != work_entity.id
    ));

    let conflict = home_client.request_message(json!({
        "schema":1,"request_id":"conflict","command":"update_entity",
        "space_id":"home","entity_id":home_entity.id,"expected_revision":9,
        "entity_type":"task.item","title":"Не должно сохраниться"
    }));
    assert!(matches!(
        conflict,
        ServerMessage::Response { ok: false, error: Some(error), .. }
            if error.code == "revision_conflict"
    ));

    let updated = created_entity(home_client.request(
        "update-home",
        json!({
            "schema":1,"request_id":"update-home","command":"update_entity",
            "space_id":"home","entity_id":home_entity.id,"expected_revision":1,
            "entity_type":"task.item","title":"Домашняя задача готова",
            "properties":{"done":true}
        }),
    ));
    assert_eq!(updated.revision, 2);
    let _ = observer.event();

    assert!(matches!(
        home_client.request(
            "select-personal",
            json!({"schema":1,"request_id":"select-personal","command":"select_space","space_id":"personal"}),
        ),
        ResponseResult::Selection { selection } if selection.space_id == "personal"
    ));
    assert!(matches!(
        observer.event(),
        EntitydEvent::SelectionChanged { selection } if selection.space_id == "personal"
    ));

    assert!(matches!(
        work_client.request(
            "delete-work",
            json!({
                "schema":1,"request_id":"delete-work","command":"delete_entity",
                "space_id":"work","entity_id":work_entity.id,"expected_revision":1
            }),
        ),
        ResponseResult::Deleted { space_id, entity_id, revision, .. }
            if space_id == "work" && entity_id == work_entity.id && revision == 2
    ));
    let _ = observer.event();

    drop(observer);
    drop(work_client);
    drop(home_client);
    drop(daemon);

    let _restarted = spawn_daemon(&store, &legacy, &socket);
    let mut client = Client::connect(&socket);
    assert!(matches!(
        client.request(
            "selection-after-restart",
            json!({"schema":1,"request_id":"selection-after-restart","command":"get_selection"}),
        ),
        ResponseResult::Selection { selection } if selection.space_id == "personal"
    ));
    assert!(matches!(
        client.request(
            "home-after-restart",
            json!({"schema":1,"request_id":"home-after-restart","command":"list_entities","space_id":"home"}),
        ),
        ResponseResult::Entities { entities, .. }
            if entities.len() == 1 && entities[0].revision == 2
                && entities[0].title == "Домашняя задача готова"
    ));
    assert!(matches!(
        client.request(
            "work-after-restart",
            json!({"schema":1,"request_id":"work-after-restart","command":"list_entities","space_id":"work"}),
        ),
        ResponseResult::Entities { entities, .. } if entities.is_empty()
    ));
}

fn created_entity(result: ResponseResult) -> saai_entity_store::Entity {
    match result {
        ResponseResult::Entity { entity, .. } => entity,
        other => panic!("expected entity response, got {other:?}"),
    }
}

fn spawn_daemon(store: &Path, legacy: &Path, socket: &Path) -> DaemonGuard {
    let child = Command::new(env!("CARGO_BIN_EXE_saai-entityd"))
        .arg("--store-root")
        .arg(store)
        .arg("--legacy-active-space")
        .arg(legacy)
        .arg("--socket")
        .arg(socket)
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    wait_for_socket(socket);
    DaemonGuard { child }
}

fn wait_for_socket(socket: &Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while UnixStream::connect(socket).is_err() {
        assert!(Instant::now() < deadline, "daemon socket was not created");
        thread::sleep(Duration::from_millis(10));
    }
}
