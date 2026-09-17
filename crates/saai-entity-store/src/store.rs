use crate::{
    validate_space_id, Entity, Event, EventPayload, ObjectRef, Provenance, Relationship,
    RelationshipEvent, RelationshipEventPayload, RelationshipQuery, SelectionSource, Space,
    SpaceKind, SpaceSelection, ValidationError, BUILTIN_SPACE_IDS, SCHEMA_VERSION,
};
use chrono::Utc;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Map;
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
const SELECTION_FILE: &str = "selection.json";
const RELATIONSHIPS_DIR: &str = "relationships";
const RECORDS_DIR: &str = "records";
const BUILTIN_SPACE_NAMES: [&str; 4] = ["Дом", "Работа", "Личное", "SaaiOS"];

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
    #[error("entity revision is exhausted: {0}")]
    RevisionExhausted(Uuid),
    #[error("relationship already exists: {0}")]
    RelationshipExists(Uuid),
    #[error("relationship not found: {0}")]
    RelationshipNotFound(Uuid),
    #[error("relationship source or target does not exist")]
    MissingRelationshipEndpoint,
    #[error("corrupt event log for {space_id}: {reason}")]
    CorruptLog { space_id: String, reason: String },
    #[error("corrupt relationship log: {reason}")]
    CorruptRelationshipLog { reason: String },
    #[error("injected failure after durable event")]
    InjectedAfterEvent,
}

#[derive(Debug)]
pub struct EntityStore {
    root: PathBuf,
    writer: Mutex<()>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapResult {
    pub spaces_created: usize,
    pub selection: SpaceSelection,
    pub migrated_legacy: bool,
}

impl EntityStore {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, StoreError> {
        let root = root.as_ref().to_path_buf();
        create_private_dir(&root)?;
        create_private_dir(&root.join(SPACES_DIR))?;
        create_private_dir(&root.join(RELATIONSHIPS_DIR))?;
        create_private_dir(&root.join(RELATIONSHIPS_DIR).join(EVENTS_DIR))?;
        create_private_dir(&root.join(RELATIONSHIPS_DIR).join(RECORDS_DIR))?;
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

    pub fn bootstrap_builtin_spaces(
        &self,
        legacy_active_space: Option<&Path>,
    ) -> Result<BootstrapResult, StoreError> {
        let mut spaces_created = 0;
        let created_at = Utc::now();
        for (id, name) in BUILTIN_SPACE_IDS.into_iter().zip(BUILTIN_SPACE_NAMES) {
            let space = Space {
                schema: SCHEMA_VERSION,
                id: id.into(),
                name: name.into(),
                kind: if id == "saaios" {
                    SpaceKind::System
                } else {
                    SpaceKind::User
                },
                created_at,
            };
            match self.create_space(space) {
                Ok(_) => spaces_created += 1,
                Err(StoreError::SpaceExists(_)) => {}
                Err(error) => return Err(error),
            }
        }

        if let Some(selection) = self.selected_space()? {
            return Ok(BootstrapResult {
                spaces_created,
                selection,
                migrated_legacy: false,
            });
        }

        let legacy_index = legacy_active_space.and_then(read_legacy_index);
        let (space_id, source, migrated_legacy) = match legacy_index {
            Some(index) => (
                BUILTIN_SPACE_IDS[index].to_string(),
                SelectionSource::Legacy,
                true,
            ),
            None => ("home".into(), SelectionSource::Default, false),
        };
        let selection = SpaceSelection {
            schema: SCHEMA_VERSION,
            space_id,
            source,
            updated_at: Utc::now(),
        };
        self.write_selection(&selection)?;
        Ok(BootstrapResult {
            spaces_created,
            selection,
            migrated_legacy,
        })
    }

    pub fn selected_space(&self) -> Result<Option<SpaceSelection>, StoreError> {
        let path = self.root.join(SELECTION_FILE);
        match read_json::<SpaceSelection>(&path) {
            Ok(selection) => {
                selection.validate()?;
                self.space_paths(&selection.space_id)?;
                Ok(Some(selection))
            }
            Err(StoreError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn select_space(&self, space_id: &str) -> Result<SpaceSelection, StoreError> {
        validate_space_id(space_id)?;
        self.space_paths(space_id)?;
        let selection = SpaceSelection {
            schema: SCHEMA_VERSION,
            space_id: space_id.into(),
            source: SelectionSource::User,
            updated_at: Utc::now(),
        };
        self.write_selection(&selection)?;
        Ok(selection)
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
        let expected = current
            .revision
            .checked_add(1)
            .ok_or(StoreError::RevisionExhausted(entity.id))?;
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
        let expected = current
            .revision
            .checked_add(1)
            .ok_or(StoreError::RevisionExhausted(entity_id))?;
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

    pub fn get_entity(
        &self,
        space_id: &str,
        entity_id: Uuid,
    ) -> Result<Option<Entity>, StoreError> {
        validate_space_id(space_id)?;
        let paths = self.space_paths(space_id)?;
        let path = paths.entities.join(entity_filename(entity_id));
        match read_json::<Entity>(&path) {
            Ok(entity) => {
                entity.validate()?;
                if entity.space_id != space_id || entity.id != entity_id {
                    return Err(corrupt(space_id, "projection identity mismatch"));
                }
                Ok(Some(entity))
            }
            Err(StoreError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn create_relationship(
        &self,
        relationship: Relationship,
    ) -> Result<RelationshipEvent, StoreError> {
        self.create_relationship_inner(relationship, false)
    }

    fn create_relationship_inner(
        &self,
        relationship: Relationship,
        fail_after_event: bool,
    ) -> Result<RelationshipEvent, StoreError> {
        relationship.validate()?;
        if relationship.revision != 1 {
            return Err(StoreError::RevisionConflict {
                expected: 1,
                actual: relationship.revision,
            });
        }
        let _writer = self.writer.lock().expect("entity store writer lock");
        self.ensure_endpoint_exists(&relationship.source)?;
        self.ensure_endpoint_exists(&relationship.target)?;
        let paths = self.relationship_paths();
        let live = replay_relationship_events(&paths)?;
        if live.contains_key(&relationship.id) {
            return Err(StoreError::RelationshipExists(relationship.id));
        }
        let event = self.publish_relationship_event(
            &paths,
            RelationshipEventPayload::RelationshipCreated {
                relationship: relationship.clone(),
            },
        )?;
        if fail_after_event {
            return Err(StoreError::InjectedAfterEvent);
        }
        atomic_write_json(
            &paths.records.join(relationship_filename(relationship.id)),
            &relationship,
        )?;
        Ok(event)
    }

    pub fn update_relationship(
        &self,
        relationship: Relationship,
    ) -> Result<RelationshipEvent, StoreError> {
        relationship.validate()?;
        let _writer = self.writer.lock().expect("entity store writer lock");
        let paths = self.relationship_paths();
        let current = replay_relationship_events(&paths)?
            .remove(&relationship.id)
            .ok_or(StoreError::RelationshipNotFound(relationship.id))?;
        let expected = current
            .revision
            .checked_add(1)
            .ok_or(StoreError::RevisionExhausted(relationship.id))?;
        if relationship.revision != expected {
            return Err(StoreError::RevisionConflict {
                expected,
                actual: relationship.revision,
            });
        }
        if relationship.created_at != current.created_at
            || relationship.source != current.source
            || relationship.target != current.target
            || relationship.relation_type != current.relation_type
        {
            return Err(corrupt_relationships(
                "update changed immutable relationship identity",
            ));
        }
        self.ensure_endpoint_exists(&relationship.source)?;
        self.ensure_endpoint_exists(&relationship.target)?;
        let event = self.publish_relationship_event(
            &paths,
            RelationshipEventPayload::RelationshipUpdated {
                relationship: relationship.clone(),
            },
        )?;
        atomic_write_json(
            &paths.records.join(relationship_filename(relationship.id)),
            &relationship,
        )?;
        Ok(event)
    }

    pub fn delete_relationship(
        &self,
        relationship_id: Uuid,
        revision: u64,
    ) -> Result<RelationshipEvent, StoreError> {
        let _writer = self.writer.lock().expect("entity store writer lock");
        let paths = self.relationship_paths();
        let current = replay_relationship_events(&paths)?
            .remove(&relationship_id)
            .ok_or(StoreError::RelationshipNotFound(relationship_id))?;
        let expected = current
            .revision
            .checked_add(1)
            .ok_or(StoreError::RevisionExhausted(relationship_id))?;
        if revision != expected {
            return Err(StoreError::RevisionConflict {
                expected,
                actual: revision,
            });
        }
        let event = self.publish_relationship_event(
            &paths,
            RelationshipEventPayload::RelationshipDeleted {
                relationship_id,
                revision,
            },
        )?;
        let projection = paths.records.join(relationship_filename(relationship_id));
        match fs::remove_file(projection) {
            Ok(()) => sync_directory(&paths.records)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        Ok(event)
    }

    pub fn get_relationship(
        &self,
        relationship_id: Uuid,
    ) -> Result<Option<Relationship>, StoreError> {
        let path = self
            .relationship_paths()
            .records
            .join(relationship_filename(relationship_id));
        match read_json::<Relationship>(&path) {
            Ok(relationship) => {
                relationship.validate()?;
                if relationship.id != relationship_id {
                    return Err(corrupt_relationships("projection identity mismatch"));
                }
                Ok(Some(relationship))
            }
            Err(StoreError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn list_relationships(
        &self,
        query: &RelationshipQuery,
    ) -> Result<Vec<Relationship>, StoreError> {
        let mut relationships = Vec::new();
        for path in json_files(&self.relationship_paths().records)? {
            let relationship: Relationship = read_json(&path)?;
            relationship.validate()?;
            if query.matches(&relationship) {
                relationships.push(relationship);
            }
        }
        relationships.sort_by_key(|relationship| relationship.id);
        Ok(relationships)
    }

    /// Physical `Entity.space_id` is a storage partition. Semantic
    /// membership is `saaios.in-space` and can name several Spaces.
    pub fn list_semantic_space_members(&self, space_id: &str) -> Result<Vec<Entity>, StoreError> {
        validate_space_id(space_id)?;
        self.space_paths(space_id)?;
        let query = RelationshipQuery {
            target: Some(ObjectRef::space(space_id)),
            relation_type: Some(crate::RELATION_IN_SPACE.into()),
            active_at: Some(Utc::now()),
            ..RelationshipQuery::default()
        };
        let mut members = Vec::new();
        let mut seen = BTreeSet::new();
        for relationship in self.list_relationships(&query)? {
            let ObjectRef::Entity { id } = relationship.source else {
                continue;
            };
            if !seen.insert(id) {
                continue;
            }
            if let Some(entity) = self.find_entity(id)? {
                members.push(entity);
            }
        }
        members.sort_by_key(|entity| entity.id);
        Ok(members)
    }

    /// What a Space screen should show: physical partition plus
    /// `saaios.in-space` members. `list_entities` stays the storage
    /// boundary; this union is display-only.
    pub fn list_visible_space_members(&self, space_id: &str) -> Result<Vec<Entity>, StoreError> {
        let mut entities = self.list_entities(space_id)?;
        let mut seen = entities
            .iter()
            .map(|entity| entity.id)
            .collect::<BTreeSet<_>>();
        for member in self.list_semantic_space_members(space_id)? {
            if seen.insert(member.id) {
                entities.push(member);
            }
        }
        Ok(entities)
    }

    /// Canonical semantic membership for the physical partition.
    /// Idempotent: a second call does not write another edge.
    pub fn ensure_in_space(
        &self,
        entity: &Entity,
    ) -> Result<Option<RelationshipEvent>, StoreError> {
        let query = RelationshipQuery {
            source: Some(ObjectRef::entity(entity.id)),
            target: Some(ObjectRef::space(&entity.space_id)),
            relation_type: Some(crate::RELATION_IN_SPACE.into()),
            active_at: Some(Utc::now()),
            ..RelationshipQuery::default()
        };
        if !self.list_relationships(&query)?.is_empty() {
            return Ok(None);
        }
        let now = Utc::now();
        let relationship = Relationship {
            schema: SCHEMA_VERSION,
            id: Uuid::new_v4(),
            source: ObjectRef::entity(entity.id),
            target: ObjectRef::space(&entity.space_id),
            relation_type: crate::RELATION_IN_SPACE.into(),
            provenance: Provenance::System,
            confidence: None,
            valid_from: None,
            valid_until: None,
            properties: Map::new(),
            revision: 1,
            created_at: now,
            updated_at: now,
        };
        Ok(Some(self.create_relationship(relationship)?))
    }

    pub fn find_entity(&self, entity_id: Uuid) -> Result<Option<Entity>, StoreError> {
        for space in self.list_spaces()? {
            if let Some(entity) = self.get_entity(&space.id, entity_id)? {
                return Ok(Some(entity));
            }
        }
        Ok(None)
    }

    fn ensure_endpoint_exists(&self, object: &ObjectRef) -> Result<(), StoreError> {
        match object {
            ObjectRef::Space { id } => {
                self.space_paths(id)?;
                Ok(())
            }
            ObjectRef::Entity { id } => {
                if self.find_entity(*id)?.is_none() {
                    Err(StoreError::MissingRelationshipEndpoint)
                } else {
                    Ok(())
                }
            }
        }
    }

    fn relationship_paths(&self) -> RelationshipPaths {
        RelationshipPaths::from_root(&self.root)
    }

    fn publish_relationship_event(
        &self,
        paths: &RelationshipPaths,
        payload: RelationshipEventPayload,
    ) -> Result<RelationshipEvent, StoreError> {
        let events = load_relationship_events(paths)?;
        let sequence = events.last().map_or(1, |event| event.sequence + 1);
        let event = RelationshipEvent {
            schema: SCHEMA_VERSION,
            id: Uuid::new_v4(),
            sequence,
            timestamp: Utc::now(),
            payload,
        };
        event.validate()?;
        publish_immutable_json(&paths.events, &relationship_event_filename(&event), &event)?;
        Ok(event)
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
        recover_relationships(&self.relationship_paths())?;
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

    fn write_selection(&self, selection: &SpaceSelection) -> Result<(), StoreError> {
        selection.validate()?;
        self.space_paths(&selection.space_id)?;
        let _writer = self.writer.lock().expect("entity store writer lock");
        atomic_write_json(&self.root.join(SELECTION_FILE), selection)
    }
}

fn read_legacy_index(path: &Path) -> Option<usize> {
    let value = fs::read_to_string(path).ok()?;
    let index = value.trim().parse::<usize>().ok()?;
    (index < BUILTIN_SPACE_IDS.len()).then_some(index)
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

#[derive(Debug)]
struct RelationshipPaths {
    events: PathBuf,
    records: PathBuf,
}

impl RelationshipPaths {
    fn from_root(root: &Path) -> Self {
        let directory = root.join(RELATIONSHIPS_DIR);
        Self {
            events: directory.join(EVENTS_DIR),
            records: directory.join(RECORDS_DIR),
        }
    }
}

fn recover_relationships(paths: &RelationshipPaths) -> Result<(), StoreError> {
    create_private_dir(&paths.events)?;
    create_private_dir(&paths.records)?;
    let expected = replay_relationship_events(paths)?;
    let mut expected_files = BTreeSet::new();
    for relationship in expected.values() {
        let name = relationship_filename(relationship.id);
        expected_files.insert(name.clone());
        atomic_write_json(&paths.records.join(&name), relationship)?;
    }
    for projection in json_files(&paths.records)? {
        let name = projection
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if !expected_files.contains(name) {
            fs::remove_file(projection)?;
        }
    }
    sync_directory(&paths.records)?;
    Ok(())
}

fn replay_relationship_events(
    paths: &RelationshipPaths,
) -> Result<BTreeMap<Uuid, Relationship>, StoreError> {
    let events = load_relationship_events(paths)?;
    let mut relationships = BTreeMap::new();
    for event in events {
        match event.payload {
            RelationshipEventPayload::RelationshipCreated { relationship } => {
                if relationships
                    .insert(relationship.id, relationship)
                    .is_some()
                {
                    return Err(corrupt_relationships("duplicate relationship_created"));
                }
            }
            RelationshipEventPayload::RelationshipUpdated { relationship } => {
                let Some(current) = relationships.get(&relationship.id) else {
                    return Err(corrupt_relationships("relationship_updated before create"));
                };
                if current.revision.checked_add(1) != Some(relationship.revision)
                    || relationship.created_at != current.created_at
                {
                    return Err(corrupt_relationships(
                        "invalid relationship update revision",
                    ));
                }
                relationships.insert(relationship.id, relationship);
            }
            RelationshipEventPayload::RelationshipDeleted {
                relationship_id,
                revision,
            } => {
                let Some(current) = relationships.remove(&relationship_id) else {
                    return Err(corrupt_relationships("relationship_deleted before create"));
                };
                if current.revision.checked_add(1) != Some(revision) {
                    return Err(corrupt_relationships(
                        "invalid relationship delete revision",
                    ));
                }
            }
        }
    }
    Ok(relationships)
}

fn load_relationship_events(
    paths: &RelationshipPaths,
) -> Result<Vec<RelationshipEvent>, StoreError> {
    let mut events = Vec::new();
    for path in json_files(&paths.events)? {
        let event: RelationshipEvent =
            read_json(&path).map_err(|error| corrupt_relationships(error.to_string()))?;
        event
            .validate()
            .map_err(|error| corrupt_relationships(error.to_string()))?;
        let expected_sequence = events.len() as u64 + 1;
        if event.sequence != expected_sequence
            || path.file_name().and_then(|value| value.to_str())
                != Some(&relationship_event_filename(&event))
        {
            return Err(corrupt_relationships(
                "relationship event filename or sequence mismatch",
            ));
        }
        events.push(event);
    }
    Ok(events)
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
                if current.revision.checked_add(1) != Some(entity.revision)
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
                if current.revision.checked_add(1) != Some(*revision) {
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

fn relationship_filename(id: Uuid) -> String {
    format!("{id}.json")
}

fn relationship_event_filename(event: &RelationshipEvent) -> String {
    format!("{:020}-{}.json", event.sequence, event.id)
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

fn corrupt_relationships(reason: impl Into<String>) -> StoreError {
    StoreError::CorruptRelationshipLog {
        reason: reason.into(),
    }
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

    #[test]
    fn every_legacy_index_maps_to_a_stable_builtin_id() {
        for (index, expected) in BUILTIN_SPACE_IDS.into_iter().enumerate() {
            let temp = TempDir::new().unwrap();
            let legacy = temp.path().join("legacy-active-space");
            fs::write(&legacy, format!("{index}\n")).unwrap();
            let store = EntityStore::open(temp.path().join("store")).unwrap();
            let result = store.bootstrap_builtin_spaces(Some(&legacy)).unwrap();

            assert_eq!(result.spaces_created, 4);
            assert_eq!(result.selection.space_id, expected);
            assert_eq!(result.selection.source, SelectionSource::Legacy);
            assert!(result.migrated_legacy);
            assert_eq!(fs::read_to_string(legacy).unwrap(), format!("{index}\n"));
        }
    }

    #[test]
    fn bootstrap_is_idempotent_and_does_not_reread_legacy_selection() {
        let temp = TempDir::new().unwrap();
        let legacy = temp.path().join("legacy-active-space");
        fs::write(&legacy, b"2\n").unwrap();
        let store = EntityStore::open(temp.path().join("store")).unwrap();
        let first = store.bootstrap_builtin_spaces(Some(&legacy)).unwrap();
        let selection_path = store.root().join(SELECTION_FILE);
        let first_bytes = fs::read(&selection_path).unwrap();

        fs::write(&legacy, b"1\n").unwrap();
        let second = store.bootstrap_builtin_spaces(Some(&legacy)).unwrap();
        assert_eq!(second.spaces_created, 0);
        assert!(!second.migrated_legacy);
        assert_eq!(second.selection, first.selection);
        assert_eq!(fs::read(selection_path).unwrap(), first_bytes);
        assert_eq!(fs::read_to_string(legacy).unwrap(), "1\n");
    }

    #[test]
    fn missing_or_invalid_legacy_selection_defaults_to_home() {
        for legacy_value in [None, Some("-1\n"), Some("4\n"), Some("junk\n")] {
            let temp = TempDir::new().unwrap();
            let legacy = temp.path().join("legacy-active-space");
            if let Some(value) = legacy_value {
                fs::write(&legacy, value).unwrap();
            }
            let store = EntityStore::open(temp.path().join("store")).unwrap();
            let result = store.bootstrap_builtin_spaces(Some(&legacy)).unwrap();
            assert_eq!(result.selection.space_id, "home");
            assert_eq!(result.selection.source, SelectionSource::Default);
            assert!(!result.migrated_legacy);
        }
    }

    #[test]
    fn explicit_selection_requires_an_existing_space() {
        let temp = TempDir::new().unwrap();
        let store = EntityStore::open(temp.path()).unwrap();
        store.bootstrap_builtin_spaces(None).unwrap();
        let selected = store.select_space("work").unwrap();
        assert_eq!(selected.space_id, "work");
        assert_eq!(selected.source, SelectionSource::User);
        assert_eq!(store.selected_space().unwrap(), Some(selected));
        assert!(matches!(
            store.select_space("missing"),
            Err(StoreError::SpaceNotFound(id)) if id == "missing"
        ));
    }

    fn relationship(source: Uuid, space_id: &str, id: u128) -> Relationship {
        Relationship {
            schema: SCHEMA_VERSION,
            id: Uuid::from_u128(id),
            source: ObjectRef::entity(source),
            target: ObjectRef::space(space_id),
            relation_type: crate::RELATION_IN_SPACE.into(),
            provenance: crate::Provenance::User,
            confidence: None,
            valid_from: None,
            valid_until: None,
            properties: Map::new(),
            revision: 1,
            created_at: timestamp(),
            updated_at: timestamp(),
        }
    }

    #[test]
    fn relationship_create_persists_and_survives_reopen() {
        let temp = TempDir::new().unwrap();
        let store = EntityStore::open(temp.path()).unwrap();
        store.bootstrap_builtin_spaces(None).unwrap();
        let doc = entity("work", 1);
        store.create_entity(doc.clone()).unwrap();
        store
            .create_relationship(relationship(doc.id, "home", 100))
            .unwrap();
        store
            .create_relationship(relationship(doc.id, "work", 101))
            .unwrap();
        store
            .create_relationship(relationship(doc.id, "saaios", 102))
            .unwrap();

        drop(store);
        let reopened = EntityStore::open(temp.path()).unwrap();
        let home = reopened.list_semantic_space_members("home").unwrap();
        let work = reopened.list_semantic_space_members("work").unwrap();
        let saaios = reopened.list_semantic_space_members("saaios").unwrap();
        assert_eq!(home.iter().map(|e| e.id).collect::<Vec<_>>(), [doc.id]);
        assert_eq!(work.iter().map(|e| e.id).collect::<Vec<_>>(), [doc.id]);
        assert_eq!(saaios.iter().map(|e| e.id).collect::<Vec<_>>(), [doc.id]);
        assert!(reopened
            .list_entities("home")
            .unwrap()
            .iter()
            .all(|entity| entity.id != doc.id));
        assert_eq!(reopened.list_entities("work").unwrap()[0].id, doc.id);
        assert_eq!(
            reopened.list_visible_space_members("home").unwrap()[0].id,
            doc.id
        );
        assert_eq!(
            reopened.list_visible_space_members("work").unwrap()[0].id,
            doc.id
        );
        assert!(reopened
            .list_visible_space_members("personal")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn ensure_in_space_is_idempotent_and_system_provenance() {
        let temp = TempDir::new().unwrap();
        let store = EntityStore::open(temp.path()).unwrap();
        store.bootstrap_builtin_spaces(None).unwrap();
        let doc = entity("work", 3);
        store.create_entity(doc.clone()).unwrap();
        let first = store.ensure_in_space(&doc).unwrap();
        assert!(first.is_some());
        assert!(store.ensure_in_space(&doc).unwrap().is_none());
        let members = store.list_semantic_space_members("work").unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].id, doc.id);
        let links = store
            .list_relationships(&RelationshipQuery {
                source: Some(ObjectRef::entity(doc.id)),
                relation_type: Some(crate::RELATION_IN_SPACE.into()),
                ..RelationshipQuery::default()
            })
            .unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].provenance, crate::Provenance::System);
        assert_eq!(links[0].target, ObjectRef::space("work"));
    }

    #[test]
    fn removing_one_space_link_does_not_delete_the_object() {
        let temp = TempDir::new().unwrap();
        let store = EntityStore::open(temp.path()).unwrap();
        store.bootstrap_builtin_spaces(None).unwrap();
        let doc = entity("work", 2);
        store.create_entity(doc.clone()).unwrap();
        store
            .create_relationship(relationship(doc.id, "home", 200))
            .unwrap();
        store
            .create_relationship(relationship(doc.id, "work", 201))
            .unwrap();
        store.delete_relationship(Uuid::from_u128(200), 2).unwrap();
        assert!(store
            .list_semantic_space_members("home")
            .unwrap()
            .is_empty());
        assert_eq!(
            store.list_semantic_space_members("work").unwrap()[0].id,
            doc.id
        );
        assert_eq!(
            store.get_entity("work", doc.id).unwrap().unwrap().id,
            doc.id
        );
    }

    #[test]
    fn relationship_update_increments_revision_and_rejects_stale_writes() {
        let temp = TempDir::new().unwrap();
        let store = EntityStore::open(temp.path()).unwrap();
        store.bootstrap_builtin_spaces(None).unwrap();
        let doc = entity("work", 3);
        store.create_entity(doc.clone()).unwrap();
        let mut rel = relationship(doc.id, "work", 300);
        store.create_relationship(rel.clone()).unwrap();
        rel.revision = 2;
        rel.updated_at = Utc.timestamp_opt(1_788_912_001, 0).unwrap();
        rel.properties.insert("user_confirmed".into(), json!(true));
        store.update_relationship(rel.clone()).unwrap();
        assert_eq!(store.get_relationship(rel.id).unwrap().unwrap().revision, 2);
        rel.revision = 2;
        assert!(matches!(
            store.update_relationship(rel),
            Err(StoreError::RevisionConflict {
                expected: 3,
                actual: 2
            })
        ));
    }

    #[test]
    fn relationship_delete_is_a_tombstone_and_keeps_prior_events() {
        let temp = TempDir::new().unwrap();
        let store = EntityStore::open(temp.path()).unwrap();
        store.bootstrap_builtin_spaces(None).unwrap();
        let doc = entity("work", 4);
        store.create_entity(doc.clone()).unwrap();
        let rel = relationship(doc.id, "work", 400);
        store.create_relationship(rel.clone()).unwrap();
        let events = temp.path().join("relationships/events");
        let before = json_files(&events)
            .unwrap()
            .into_iter()
            .map(|path| (path.clone(), fs::read(path).unwrap()))
            .collect::<Vec<_>>();
        store.delete_relationship(rel.id, 2).unwrap();
        assert!(store.get_relationship(rel.id).unwrap().is_none());
        assert!(!json_files(&events).unwrap().is_empty());
        for (path, contents) in before {
            assert_eq!(fs::read(path).unwrap(), contents);
        }
    }

    #[test]
    fn relationship_replay_repairs_missing_projection() {
        let temp = TempDir::new().unwrap();
        let store = EntityStore::open(temp.path()).unwrap();
        store.bootstrap_builtin_spaces(None).unwrap();
        let doc = entity("work", 5);
        store.create_entity(doc.clone()).unwrap();
        let rel = relationship(doc.id, "home", 500);
        assert!(matches!(
            store.create_relationship_inner(rel.clone(), true),
            Err(StoreError::InjectedAfterEvent)
        ));
        assert!(store
            .list_relationships(&RelationshipQuery::default())
            .unwrap()
            .is_empty());
        drop(store);
        let reopened = EntityStore::open(temp.path()).unwrap();
        assert_eq!(
            reopened
                .list_relationships(&RelationshipQuery::default())
                .unwrap(),
            [rel]
        );
    }

    #[test]
    fn missing_endpoint_and_stale_revision_are_rejected() {
        let temp = TempDir::new().unwrap();
        let store = EntityStore::open(temp.path()).unwrap();
        store.bootstrap_builtin_spaces(None).unwrap();
        assert!(matches!(
            store.create_relationship(relationship(Uuid::from_u128(99), "work", 600)),
            Err(StoreError::MissingRelationshipEndpoint)
        ));
        let doc = entity("work", 6);
        store.create_entity(doc.clone()).unwrap();
        store
            .create_relationship(relationship(doc.id, "work", 601))
            .unwrap();
        assert!(matches!(
            store.delete_relationship(Uuid::from_u128(601), 9),
            Err(StoreError::RevisionConflict {
                expected: 2,
                actual: 9
            })
        ));
    }

    #[test]
    fn model_provenance_survives_and_stays_distinct_from_user_facts() {
        let temp = TempDir::new().unwrap();
        let store = EntityStore::open(temp.path()).unwrap();
        store.bootstrap_builtin_spaces(None).unwrap();
        let person = entity("work", 7);
        let server = entity("work", 8);
        store.create_entity(person.clone()).unwrap();
        store.create_entity(server.clone()).unwrap();
        let mut inferred = Relationship {
            schema: SCHEMA_VERSION,
            id: Uuid::from_u128(700),
            source: ObjectRef::entity(person.id),
            target: ObjectRef::entity(server.id),
            relation_type: "saaios.related-to".into(),
            provenance: crate::Provenance::Model {
                provider: Some("local".into()),
                model: Some("notes".into()),
            },
            confidence: Some(0.68),
            valid_from: None,
            valid_until: None,
            properties: Map::from_iter([("user_confirmed".into(), json!(false))]),
            revision: 1,
            created_at: timestamp(),
            updated_at: timestamp(),
        };
        store.create_relationship(inferred.clone()).unwrap();
        let stored = store.get_relationship(inferred.id).unwrap().unwrap();
        assert!(stored.provenance.is_inferred());
        assert!(!stored.user_confirmed());
        inferred.revision = 2;
        inferred.updated_at = Utc.timestamp_opt(1_788_912_001, 0).unwrap();
        inferred
            .properties
            .insert("user_confirmed".into(), json!(true));
        store.update_relationship(inferred).unwrap();
        let confirmed = store
            .get_relationship(Uuid::from_u128(700))
            .unwrap()
            .unwrap();
        assert_eq!(confirmed.provenance.kind_token(), "model");
        assert!(confirmed.user_confirmed());
    }
}
