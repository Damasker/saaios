use crate::{
    validate_space_id, Entity, Event, EventPayload, Space, ValidationError, SCHEMA_VERSION,
};
use chrono::Utc;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use thiserror::Error;
use uuid::Uuid;

const SCHEMA_FILE: &str = "schema-version";
const SPACES_DIR: &str = "spaces";
const SPACE_FILE: &str = "space.json";
const EVENTS_DIR: &str = "events";
const ENTITIES_DIR: &str = "entities";

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid record: {0}")]
    Validation(#[from] ValidationError),
    #[error("unsupported store schema: {0}")]
    UnsupportedStoreSchema(String),
    #[error("space already exists: {0}")]
    SpaceExists(String),
    #[error("space not found: {0}")]
    SpaceNotFound(String),
    #[error("entity already exists: {0}")]
    EntityExists(Uuid),
    #[error("entity not found: {0}")]
    EntityNotFound(Uuid),
    #[error("entity revision must be {expected}, got {actual}")]
    RevisionConflict { expected: u64, actual: u64 },
    #[error("corrupt event log for {space_id}: {reason}")]
    CorruptLog { space_id: String, reason: String },
    #[error("injected failure after durable event")]
    InjectedAfterEvent,
}

#[derive(Debug)]
pub struct EntityStore {
    root: PathBuf,
    writer: Mutex<()>,
}

impl EntityStore {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, StoreError> {
        let root = root.as_ref().to_path_buf();
        create_private_dir(&root)?;
        create_private_dir(&root.join(SPACES_DIR))?;
        ensure_store_schema(&root)?;
        let store = Self {
            root,
            writer: Mutex::new(()),
        };
        store.recover_all()?;
        Ok(store)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn create_space(&self, space: Space) -> Result<Event, StoreError> {
        space.validate()?;
        let _writer = self.writer.lock().expect("entity store writer lock");
        let spaces = self.root.join(SPACES_DIR);
        let destination = spaces.join(&space.id);
        if destination.exists() {
            return Err(StoreError::SpaceExists(space.id));
        }

        let temporary = spaces.join(format!(".{}.{}.tmp", space.id, Uuid::new_v4()));
        create_private_dir(&temporary)?;
        create_private_dir(&temporary.join(EVENTS_DIR))?;
        create_private_dir(&temporary.join(ENTITIES_DIR))?;

        let event = Event {
            schema: SCHEMA_VERSION,
            id: Uuid::new_v4(),
            sequence: 1,
            space_id: space.id.clone(),
            timestamp: Utc::now(),
            payload: EventPayload::SpaceCreated {
                space: space.clone(),
            },
        };
        event.validate()?;
        write_new_json(&temporary.join(SPACE_FILE), &space)?;
        write_new_json(
            &temporary.join(EVENTS_DIR).join(event_filename(&event)),
            &event,
        )?;
        sync_directory(&temporary.join(EVENTS_DIR))?;
        sync_directory(&temporary.join(ENTITIES_DIR))?;
        sync_directory(&temporary)?;
        if let Err(error) = fs::rename(&temporary, &destination) {
            let _ = fs::remove_dir_all(&temporary);
            return Err(error.into());
        }
        sync_directory(&spaces)?;
        Ok(event)
    }

    pub fn list_spaces(&self) -> Result<Vec<Space>, StoreError> {
        let mut spaces = Vec::new();
        for directory in visible_directories(&self.root.join(SPACES_DIR))? {
            let space: Space = read_json(&directory.join(SPACE_FILE))?;
            space.validate()?;
            if directory.file_name().and_then(|name| name.to_str()) != Some(space.id.as_str()) {
                return Err(corrupt(
                    &space.id,
                    "space directory does not match space id",
                ));
            }
            spaces.push(space);
        }
        spaces.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(spaces)
    }

    pub fn create_entity(&self, entity: Entity) -> Result<Event, StoreError> {
        self.create_entity_inner(entity, false)
    }

    fn create_entity_inner(
        &self,
        entity: Entity,
        fail_after_event: bool,
    ) -> Result<Event, StoreError> {
        entity.validate()?;
        if entity.revision != 1 {
            return Err(StoreError::RevisionConflict {
                expected: 1,
                actual: entity.revision,
            });
        }
        let _writer = self.writer.lock().expect("entity store writer lock");
        let paths = self.space_paths(&entity.space_id)?;
        let projection = paths.entities.join(entity_filename(entity.id));
        if projection.exists() || replay_events(&paths)?.contains_key(&entity.id) {
            return Err(StoreError::EntityExists(entity.id));
        }
        let event = self.publish_entity_event(
            &paths,
            EventPayload::EntityCreated {
                entity: entity.clone(),
            },
        )?;
        if fail_after_event {
            return Err(StoreError::InjectedAfterEvent);
        }
        atomic_write_json(&projection, &entity)?;
        Ok(event)
    }

    pub fn update_entity(&self, entity: Entity) -> Result<Event, StoreError> {
        entity.validate()?;
        let _writer = self.writer.lock().expect("entity store writer lock");
        let paths = self.space_paths(&entity.space_id)?;
        let current = replay_events(&paths)?
            .remove(&entity.id)
            .ok_or(StoreError::EntityNotFound(entity.id))?;
        let expected = current.revision.saturating_add(1);
        if entity.revision != expected {
            return Err(StoreError::RevisionConflict {
                expected,
                actual: entity.revision,
            });
        }
        if entity.created_at != current.created_at {
            return Err(corrupt(
                &entity.space_id,
                "update changed immutable created_at",
            ));
        }
        let event = self.publish_entity_event(
            &paths,
            EventPayload::EntityUpdated {
                entity: entity.clone(),
            },
        )?;
        atomic_write_json(&paths.entities.join(entity_filename(entity.id)), &entity)?;
        Ok(event)
    }

    pub fn delete_entity(
        &self,
        space_id: &str,
        entity_id: Uuid,
        revision: u64,
    ) -> Result<Event, StoreError> {
        validate_space_id(space_id)?;
        let _writer = self.writer.lock().expect("entity store writer lock");
        let paths = self.space_paths(space_id)?;
        let current = replay_events(&paths)?
            .remove(&entity_id)
            .ok_or(StoreError::EntityNotFound(entity_id))?;
        let expected = current.revision.saturating_add(1);
        if revision != expected {
            return Err(StoreError::RevisionConflict {
                expected,
                actual: revision,
            });
        }
        let event = self.publish_entity_event(
            &paths,
            EventPayload::EntityDeleted {
                entity_id,
                revision,
            },
        )?;
        let projection = paths.entities.join(entity_filename(entity_id));
        match fs::remove_file(projection) {
            Ok(()) => sync_directory(&paths.entities)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        Ok(event)
    }

    pub fn list_entities(&self, space_id: &str) -> Result<Vec<Entity>, StoreError> {
        validate_space_id(space_id)?;
        let paths = self.space_paths(space_id)?;
        let mut entities = Vec::new();
        for path in json_files(&paths.entities)? {
            let entity: Entity = read_json(&path)?;
            entity.validate()?;
            if entity.space_id != space_id {
                return Err(corrupt(space_id, "projection crosses space boundary"));
            }
            entities.push(entity);
        }
        entities.sort_by_key(|entity| entity.id);
        Ok(entities)
    }

    pub fn recover_all(&self) -> Result<(), StoreError> {
        let _writer = self.writer.lock().expect("entity store writer lock");
        for directory in visible_directories(&self.root.join(SPACES_DIR))? {
            let id = directory
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| corrupt("unknown", "non-UTF-8 space directory"))?;
            validate_space_id(id)?;
            let paths = SpacePaths::from_directory(directory);
            recover_space(&paths)?;
        }
        Ok(())
    }

    fn publish_entity_event(
        &self,
        paths: &SpacePaths,
        payload: EventPayload,
    ) -> Result<Event, StoreError> {
        let events = load_events(paths)?;
        let sequence = events.last().map_or(1, |event| event.sequence + 1);
        let event = Event {
            schema: SCHEMA_VERSION,
            id: Uuid::new_v4(),
            sequence,
            space_id: paths.id.clone(),
            timestamp: Utc::now(),
            payload,
        };
        event.validate()?;
        publish_immutable_json(&paths.events, &event_filename(&event), &event)?;
        Ok(event)
    }

    fn space_paths(&self, space_id: &str) -> Result<SpacePaths, StoreError> {
        validate_space_id(space_id)?;
        let directory = self.root.join(SPACES_DIR).join(space_id);
        if !directory.is_dir() {
            return Err(StoreError::SpaceNotFound(space_id.into()));
        }
        Ok(SpacePaths::from_directory(directory))
    }
}

#[derive(Debug)]
struct SpacePaths {
    id: String,
    root: PathBuf,
    events: PathBuf,
    entities: PathBuf,
}

impl SpacePaths {
    fn from_directory(root: PathBuf) -> Self {
        let id = root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        Self {
            events: root.join(EVENTS_DIR),
            entities: root.join(ENTITIES_DIR),
            id,
            root,
        }
    }
}

fn recover_space(paths: &SpacePaths) -> Result<(), StoreError> {
    create_private_dir(&paths.events)?;
    create_private_dir(&paths.entities)?;
    let space: Space = read_json(&paths.root.join(SPACE_FILE))?;
    space.validate()?;
    if space.id != paths.id {
        return Err(corrupt(&paths.id, "space file crosses directory boundary"));
    }
    let events = load_events(paths)?;
    match events.first().map(|event| &event.payload) {
        Some(EventPayload::SpaceCreated { space: created }) if created == &space => {}
        _ => {
            return Err(corrupt(
                &paths.id,
                "first event is not matching space_created",
            ))
        }
    }
    let expected = replay_loaded_events(paths, &events)?;
    let mut expected_files = BTreeSet::new();
    for entity in expected.values() {
        let name = entity_filename(entity.id);
        expected_files.insert(name.clone());
        atomic_write_json(&paths.entities.join(name), entity)?;
    }
    for projection in json_files(&paths.entities)? {
        let name = projection
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if !expected_files.contains(name) {
            fs::remove_file(projection)?;
        }
    }
    sync_directory(&paths.entities)?;
    Ok(())
}

fn replay_events(paths: &SpacePaths) -> Result<BTreeMap<Uuid, Entity>, StoreError> {
    let events = load_events(paths)?;
    replay_loaded_events(paths, &events)
}

fn replay_loaded_events(
    paths: &SpacePaths,
    events: &[Event],
) -> Result<BTreeMap<Uuid, Entity>, StoreError> {
    let mut entities = BTreeMap::new();
    for event in events {
        match &event.payload {
            EventPayload::SpaceCreated { .. } => {}
            EventPayload::EntityCreated { entity } => {
                if entities.insert(entity.id, entity.clone()).is_some() {
                    return Err(corrupt(&paths.id, "duplicate entity_created"));
                }
            }
            EventPayload::EntityUpdated { entity } => {
                let Some(current) = entities.get(&entity.id) else {
                    return Err(corrupt(&paths.id, "entity_updated before create"));
                };
                if entity.revision != current.revision.saturating_add(1)
                    || entity.created_at != current.created_at
                {
                    return Err(corrupt(&paths.id, "invalid entity update revision"));
                }
                entities.insert(entity.id, entity.clone());
            }
            EventPayload::EntityDeleted {
                entity_id,
                revision,
            } => {
                let Some(current) = entities.remove(entity_id) else {
                    return Err(corrupt(&paths.id, "entity_deleted before create"));
                };
                if *revision != current.revision.saturating_add(1) {
                    return Err(corrupt(&paths.id, "invalid entity delete revision"));
                }
            }
        }
    }
    Ok(entities)
}

fn load_events(paths: &SpacePaths) -> Result<Vec<Event>, StoreError> {
    let mut events = Vec::new();
    for path in json_files(&paths.events)? {
        let event: Event =
            read_json(&path).map_err(|error| corrupt(&paths.id, error.to_string()))?;
        event
            .validate()
            .map_err(|error| corrupt(&paths.id, error.to_string()))?;
        let expected_sequence = events.len() as u64 + 1;
        if event.sequence != expected_sequence
            || path.file_name().and_then(|v| v.to_str()) != Some(&event_filename(&event))
        {
            return Err(corrupt(&paths.id, "event filename or sequence mismatch"));
        }
        events.push(event);
    }
    Ok(events)
}

fn ensure_store_schema(root: &Path) -> Result<(), StoreError> {
    let path = root.join(SCHEMA_FILE);
    match fs::read_to_string(&path) {
        Ok(value) if value.trim() == SCHEMA_VERSION.to_string() => Ok(()),
        Ok(value) => Err(StoreError::UnsupportedStoreSchema(value.trim().into())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            atomic_write_bytes(&path, format!("{SCHEMA_VERSION}\n").as_bytes())
        }
        Err(error) => Err(error.into()),
    }
}

fn event_filename(event: &Event) -> String {
    format!("{:020}-{}.json", event.sequence, event.id)
}

fn entity_filename(id: Uuid) -> String {
    format!("{id}.json")
}

fn publish_immutable_json<T: Serialize>(
    directory: &Path,
    name: &str,
    value: &T,
) -> Result<(), StoreError> {
    let destination = directory.join(name);
    if destination.exists() {
        return Err(std::io::Error::new(std::io::ErrorKind::AlreadyExists, name).into());
    }
    atomic_write_json(&destination, value)
}

fn atomic_write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), StoreError> {
    let mut bytes = serde_json::to_vec(value)?;
    bytes.push(b'\n');
    atomic_write_bytes(path, &bytes)
}

fn write_new_json<T: Serialize>(path: &Path, value: &T) -> Result<(), StoreError> {
    let mut bytes = serde_json::to_vec(value)?;
    bytes.push(b'\n');
    write_new_bytes(path, &bytes)
}

fn atomic_write_bytes(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "path has no parent")
    })?;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "non-UTF-8 filename")
        })?;
    let temporary = parent.join(format!(".{name}.{}.tmp", Uuid::new_v4()));
    write_new_bytes(&temporary, bytes)?;
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    sync_directory(parent)?;
    Ok(())
}

fn write_new_bytes(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, StoreError> {
    let mut bytes = Vec::new();
    File::open(path)?.read_to_end(&mut bytes)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn json_files(directory: &Path) -> Result<Vec<PathBuf>, StoreError> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if name.starts_with('.') && name.ends_with(".tmp") {
            continue;
        }
        if entry.file_type()?.is_file() && name.ends_with(".json") {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn visible_directories(directory: &Path) -> Result<Vec<PathBuf>, StoreError> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        if entry.file_type()?.is_dir() {
            paths.push(entry.path());
        }
    }
    paths.sort();
    Ok(paths)
}

fn create_private_dir(path: &Path) -> Result<(), StoreError> {
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn sync_directory(path: &Path) -> Result<(), StoreError> {
    File::open(path)?.sync_all()?;
    Ok(())
}

fn corrupt(space_id: &str, reason: impl Into<String>) -> StoreError {
    StoreError::CorruptLog {
        space_id: space_id.into(),
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SpaceKind, BUILTIN_SPACE_IDS};
    use chrono::TimeZone;
    use serde_json::{json, Map};
    use tempfile::TempDir;

    fn timestamp() -> chrono::DateTime<Utc> {
        Utc.timestamp_opt(1_788_912_000, 0).unwrap()
    }

    fn space(id: &str) -> Space {
        Space {
            schema: SCHEMA_VERSION,
            id: id.into(),
            name: id.into(),
            kind: if id == "saaios" {
                SpaceKind::System
            } else {
                SpaceKind::User
            },
            created_at: timestamp(),
        }
    }

    fn entity(space_id: &str, id: u128) -> Entity {
        Entity {
            schema: SCHEMA_VERSION,
            id: Uuid::from_u128(id),
            space_id: space_id.into(),
            entity_type: "task.item".into(),
            title: format!("task-{id}"),
            properties: Map::from_iter([("done".into(), json!(false))]),
            revision: 1,
            created_at: timestamp(),
            updated_at: timestamp(),
        }
    }

    #[test]
    fn entities_are_scoped_by_space_directory() {
        let temp = TempDir::new().unwrap();
        let store = EntityStore::open(temp.path()).unwrap();
        store.create_space(space("home")).unwrap();
        store.create_space(space("work")).unwrap();
        store.create_entity(entity("home", 1)).unwrap();
        store.create_entity(entity("work", 2)).unwrap();

        let home = store.list_entities("home").unwrap();
        let work = store.list_entities("work").unwrap();
        assert_eq!(
            home.iter().map(|e| e.id).collect::<Vec<_>>(),
            [Uuid::from_u128(1)]
        );
        assert_eq!(
            work.iter().map(|e| e.id).collect::<Vec<_>>(),
            [Uuid::from_u128(2)]
        );
    }

    #[test]
    fn replay_repairs_missing_projection_after_durable_event() {
        let temp = TempDir::new().unwrap();
        let store = EntityStore::open(temp.path()).unwrap();
        store.create_space(space("home")).unwrap();
        let value = entity("home", 10);
        assert!(matches!(
            store.create_entity_inner(value.clone(), true),
            Err(StoreError::InjectedAfterEvent)
        ));
        assert!(store.list_entities("home").unwrap().is_empty());
        drop(store);

        let reopened = EntityStore::open(temp.path()).unwrap();
        assert_eq!(reopened.list_entities("home").unwrap(), [value]);
    }

    #[test]
    fn update_and_delete_keep_prior_events_immutable() {
        let temp = TempDir::new().unwrap();
        let store = EntityStore::open(temp.path()).unwrap();
        store.create_space(space("home")).unwrap();
        let original = entity("home", 20);
        store.create_entity(original.clone()).unwrap();
        let events = temp.path().join("spaces/home/events");
        let before = json_files(&events)
            .unwrap()
            .into_iter()
            .map(|path| (path.clone(), fs::read(path).unwrap()))
            .collect::<Vec<_>>();

        let mut updated = original.clone();
        updated.revision = 2;
        updated.title = "updated".into();
        updated.updated_at = Utc.timestamp_opt(1_788_912_001, 0).unwrap();
        store.update_entity(updated).unwrap();
        store.delete_entity("home", original.id, 3).unwrap();
        assert!(store.list_entities("home").unwrap().is_empty());
        for (path, contents) in before {
            assert_eq!(fs::read(path).unwrap(), contents);
        }
    }

    #[test]
    fn temporary_event_is_ignored_but_published_corruption_blocks_open() {
        let temp = TempDir::new().unwrap();
        let store = EntityStore::open(temp.path()).unwrap();
        store.create_space(space("home")).unwrap();
        let events = temp.path().join("spaces/home/events");
        fs::write(events.join(".interrupted.tmp"), b"{not json").unwrap();
        drop(store);
        EntityStore::open(temp.path()).unwrap();

        fs::write(events.join("00000000000000000002-bad.json"), b"{not json").unwrap();
        assert!(matches!(
            EntityStore::open(temp.path()),
            Err(StoreError::CorruptLog { .. })
        ));
    }

    #[test]
    fn builtins_can_be_created_and_sorted_independent_of_creation_order() {
        let temp = TempDir::new().unwrap();
        let store = EntityStore::open(temp.path()).unwrap();
        for id in BUILTIN_SPACE_IDS.into_iter().rev() {
            store.create_space(space(id)).unwrap();
        }
        assert_eq!(
            store
                .list_spaces()
                .unwrap()
                .into_iter()
                .map(|s| s.id)
                .collect::<Vec<_>>(),
            ["home", "personal", "saaios", "work"]
        );
    }
}
