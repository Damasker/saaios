#![cfg(unix)]

use saai_entity_protocol::{EntitydEvent, ResponseResult, ServerMessage};
use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
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

struct Client<S> {
    reader: BufReader<S>,
    writer: S,
}

impl Client<UnixStream> {
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
}

impl Client<TcpStream> {
    fn connect_tcp(addr: SocketAddr) -> Self {
        let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        Self {
            writer: stream.try_clone().unwrap(),
            reader: BufReader::new(stream),
        }
    }
}

impl<S: std::io::Read + Write> Client<S> {
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
    assert!(matches!(
        observer.event(),
        EntitydEvent::RelationshipChanged { .. }
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
    assert!(matches!(
        observer.event(),
        EntitydEvent::RelationshipChanged { .. }
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

#[test]
fn daemon_relationship_crud_events_and_semantic_membership() {
    let temp = TempDir::new().unwrap();
    let store = temp.path().join("entities");
    let legacy = temp.path().join("active-space");
    let socket = temp.path().join("run/entityd.sock");
    fs::write(&legacy, b"0\n").unwrap();

    let daemon = spawn_daemon(&store, &legacy, &socket);
    let mut client = Client::connect(&socket);
    let mut observer = Client::connect(&socket);
    assert!(matches!(
        observer.request(
            "subscribe",
            json!({"schema":1,"request_id":"subscribe","command":"subscribe"}),
        ),
        ResponseResult::Subscribed
    ));

    let entity = created_entity(client.request(
        "create-doc",
        json!({
            "schema":1,"request_id":"create-doc","command":"create_entity",
            "space_id":"work","entity_type":"document.file","title":"notes.pdf"
        }),
    ));
    assert!(matches!(
        observer.event(),
        EntitydEvent::EntityChanged { .. }
    ));
    assert!(matches!(
        observer.event(),
        EntitydEvent::RelationshipChanged { .. }
    ));

    assert!(matches!(
        client.request(
            "auto-work-members",
            json!({"schema":1,"request_id":"auto-work-members","command":"list_space_members","space_id":"work"}),
        ),
        ResponseResult::Entities { entities, .. }
            if entities.len() == 1 && entities[0].id == entity.id
    ));

    let home = created_relationship(client.request(
        "link-home",
        json!({
            "schema":1,"request_id":"link-home","command":"create_relationship",
            "source":{"kind":"entity","id":entity.id},
            "target":{"kind":"space","id":"home"},
            "relation_type":"saaios.in-space",
            "provenance":{"kind":"user"}
        }),
    ));
    assert!(matches!(
        observer.event(),
        EntitydEvent::RelationshipChanged { .. }
    ));

    assert!(matches!(
        client.request(
            "physical-home",
            json!({"schema":1,"request_id":"physical-home","command":"list_entities","space_id":"home"}),
        ),
        ResponseResult::Entities { entities, .. } if entities.is_empty()
    ));
    assert!(matches!(
        client.request(
            "visible-home",
            json!({"schema":1,"request_id":"visible-home","command":"list_space_members","space_id":"home"}),
        ),
        ResponseResult::Entities { entities, .. }
            if entities.len() == 1 && entities[0].id == entity.id
    ));
    assert!(matches!(
        client.request(
            "visible-work",
            json!({"schema":1,"request_id":"visible-work","command":"list_space_members","space_id":"work"}),
        ),
        ResponseResult::Entities { entities, .. }
            if entities.len() == 1 && entities[0].id == entity.id
    ));

    let listed = client.request(
        "list-home-links",
        json!({
            "schema":1,"request_id":"list-home-links","command":"list_relationships",
            "object":{"kind":"space","id":"home"},
            "direction":"to",
            "relation_type":"saaios.in-space"
        }),
    );
    assert!(matches!(
        listed,
        ResponseResult::Relationships { relationships }
            if relationships.len() == 1 && relationships[0].source == saai_entity_store::ObjectRef::entity(entity.id)
    ));

    let conflict = client.request_message(json!({
        "schema":1,"request_id":"rel-conflict","command":"update_relationship",
        "relationship_id":home.id,"expected_revision":9,
        "provenance":{"kind":"user"}
    }));
    assert!(matches!(
        conflict,
        ServerMessage::Response { ok: false, error: Some(error), .. }
            if error.code == "revision_conflict"
    ));

    let inferred = client.request_message(json!({
        "schema":1,"request_id":"bad-confidence","command":"create_relationship",
        "source":{"kind":"entity","id":entity.id},
        "target":{"kind":"space","id":"personal"},
        "relation_type":"saaios.in-space",
        "provenance":{"kind":"model","provider":"local"},
        "confidence":1.4
    }));
    assert!(matches!(
        inferred,
        ServerMessage::Response { ok: false, error: Some(error), .. }
            if error.code == "invalid_record"
    ));

    assert!(matches!(
        client.request(
            "delete-home-link",
            json!({
                "schema":1,"request_id":"delete-home-link","command":"delete_relationship",
                "relationship_id":home.id,"expected_revision":1
            }),
        ),
        ResponseResult::RelationshipDeleted { relationship_id, revision, .. }
            if relationship_id == home.id && revision == 2
    ));
    let _ = observer.event();

    drop(observer);
    drop(client);
    drop(daemon);

    let _restarted = spawn_daemon(&store, &legacy, &socket);
    let mut client = Client::connect(&socket);
    let listed = client.request(
        "after-restart",
        json!({
            "schema":1,"request_id":"after-restart","command":"list_relationships",
            "object":{"kind":"entity","id":entity.id}
        }),
    );
    assert!(matches!(
        listed,
        ResponseResult::Relationships { relationships }
            if relationships.len() == 1
                && relationships[0].target == saai_entity_store::ObjectRef::space("work")
                && relationships[0].provenance == saai_entity_store::Provenance::System
    ));
}

fn created_entity(result: ResponseResult) -> saai_entity_store::Entity {
    match result {
        ResponseResult::Entity { entity, .. } => entity,
        other => panic!("expected entity response, got {other:?}"),
    }
}

fn created_relationship(result: ResponseResult) -> saai_entity_store::Relationship {
    match result {
        ResponseResult::Relationship { relationship, .. } => relationship,
        other => panic!("expected relationship response, got {other:?}"),
    }
}

fn spawn_daemon(store: &Path, legacy: &Path, socket: &Path) -> DaemonGuard {
    spawn_daemon_with_tcp(store, legacy, socket, None)
}

fn spawn_daemon_with_tcp(
    store: &Path,
    legacy: &Path,
    socket: &Path,
    tcp: Option<SocketAddr>,
) -> DaemonGuard {
    let mut command = Command::new(env!("CARGO_BIN_EXE_saai-entityd"));
    command
        .arg("--store-root")
        .arg(store)
        .arg("--legacy-active-space")
        .arg(legacy)
        .arg("--socket")
        .arg(socket)
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    match tcp {
        Some(addr) => {
            command.arg("--tcp-bind").arg(addr.to_string());
        }
        None => {
            command.arg("--tcp-bind").arg("none");
        }
    }
    let child = command.spawn().unwrap();
    wait_for_socket(socket);
    if let Some(addr) = tcp {
        wait_for_tcp(addr);
    }
    DaemonGuard { child }
}

fn wait_for_socket(socket: &Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while UnixStream::connect(socket).is_err() {
        assert!(Instant::now() < deadline, "daemon socket was not created");
        thread::sleep(Duration::from_millis(10));
    }
}

fn wait_for_tcp(addr: SocketAddr) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while TcpStream::connect_timeout(&addr, Duration::from_millis(50)).is_err() {
        assert!(Instant::now() < deadline, "daemon TCP was not listening");
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn tcp_client_lists_the_same_space_entity_as_unix() {
    let temp = TempDir::new().unwrap();
    let store = temp.path().join("entities");
    let legacy = temp.path().join("active-space");
    let socket = temp.path().join("run/entityd.sock");
    fs::write(&legacy, b"1\n").unwrap();
    let probe = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = probe.local_addr().unwrap();
    drop(probe);

    let _daemon = spawn_daemon_with_tcp(&store, &legacy, &socket, Some(addr));
    let mut unix = Client::connect(&socket);
    let created = created_entity(unix.request(
        "create-home",
        json!({
            "schema":1,"request_id":"create-home","command":"create_entity",
            "space_id":"home","entity_type":"saaios.intent",
            "title":"shared-intent","properties":{}
        }),
    ));

    let mut tcp = Client::connect_tcp(addr);
    let listed = tcp.request(
        "list-home",
        json!({
            "schema":1,"request_id":"list-home","command":"list_entities",
            "space_id":"home"
        }),
    );
    match listed {
        ResponseResult::Entities { entities, space_id } => {
            assert_eq!(space_id, "home");
            assert!(
                entities
                    .iter()
                    .any(|entity| entity.id == created.id && entity.title == "shared-intent"),
                "TCP client must see the Unix-created Intent, got {entities:?}"
            );
        }
        other => panic!("expected entities, got {other:?}"),
    }
}
