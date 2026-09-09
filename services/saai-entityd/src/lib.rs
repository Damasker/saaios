#![cfg(unix)]

use chrono::Utc;
use saai_entity_protocol::{
    decode_request, encode_message, ClientRequest, EntitydEvent, ProtocolError, ResponseResult,
    ServerMessage, MAX_WIRE_MESSAGE_BYTES,
};
use saai_entity_store::{Entity, EntityStore, StoreError, SCHEMA_VERSION};
use std::fs;
use std::io;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{broadcast, Mutex};
use uuid::Uuid;

const EVENT_CAPACITY: usize = 128;

#[derive(Debug, Clone)]
pub struct DaemonConfig {
    pub store_root: PathBuf,
    pub legacy_active_space: PathBuf,
    pub socket_path: PathBuf,
}

#[derive(Debug, Error)]
pub enum EntitydError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("another saai-entityd is already listening at {0}")]
    AlreadyRunning(PathBuf),
    #[error("{operation} failed for {path}: {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

struct SocketGuard {
    path: PathBuf,
}

impl Drop for SocketGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub async fn run_daemon(config: DaemonConfig) -> Result<(), EntitydError> {
    let store = EntityStore::open(&config.store_root)?;
    store.bootstrap_builtin_spaces(Some(&config.legacy_active_space))?;
    prepare_socket_path(&config.socket_path).await?;
    let listener = UnixListener::bind(&config.socket_path).map_err(|source| EntitydError::Io {
        operation: "bind entityd socket",
        path: config.socket_path.clone(),
        source,
    })?;
    let _socket_guard = SocketGuard {
        path: config.socket_path.clone(),
    };
    fs::set_permissions(&config.socket_path, fs::Permissions::from_mode(0o660)).map_err(
        |source| EntitydError::Io {
            operation: "set entityd socket permissions",
            path: config.socket_path.clone(),
            source,
        },
    )?;
    let store = Arc::new(Mutex::new(store));
    let (events, _) = broadcast::channel(EVENT_CAPACITY);

    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let (stream, _) = accepted.map_err(|source| EntitydError::Io {
                    operation: "accept entityd client",
                    path: config.socket_path.clone(),
                    source,
                })?;
                let client_store = Arc::clone(&store);
                let client_events = events.clone();
                tokio::spawn(async move {
                    if let Err(error) = serve_client(stream, client_store, client_events).await {
                        eprintln!("saai-entityd: client disconnected: {error}");
                    }
                });
            }
            signal = tokio::signal::ctrl_c() => {
                signal.map_err(|source| EntitydError::Io {
                    operation: "listen for shutdown signal",
                    path: config.socket_path.clone(),
                    source,
                })?;
                return Ok(());
            }
        }
    }
}

async fn prepare_socket_path(path: &Path) -> Result<(), EntitydError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| EntitydError::Io {
            operation: "create entityd runtime directory",
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(source) => {
            return Err(EntitydError::Io {
                operation: "inspect entityd socket",
                path: path.to_path_buf(),
                source,
            })
        }
    };
    if !metadata.file_type().is_socket() {
        return Err(EntitydError::Io {
            operation: "refuse non-socket entityd path",
            path: path.to_path_buf(),
            source: io::Error::new(io::ErrorKind::AlreadyExists, "path is not a Unix socket"),
        });
    }
    match UnixStream::connect(path).await {
        Ok(_) => Err(EntitydError::AlreadyRunning(path.to_path_buf())),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::ConnectionRefused | io::ErrorKind::NotFound
            ) =>
        {
            fs::remove_file(path).map_err(|source| EntitydError::Io {
                operation: "remove stale entityd socket",
                path: path.to_path_buf(),
                source,
            })?;
            Ok(())
        }
        Err(source) => Err(EntitydError::Io {
            operation: "probe existing entityd socket",
            path: path.to_path_buf(),
            source,
        }),
    }
}

async fn serve_client(
    stream: UnixStream,
    store: Arc<Mutex<EntityStore>>,
    events: broadcast::Sender<EntitydEvent>,
) -> Result<(), ProtocolError> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::with_capacity(8192, reader);
    let mut event_receiver = events.subscribe();
    let mut subscribed = false;

    loop {
        tokio::select! {
            frame = read_frame(&mut reader) => {
                let Some(frame) = frame? else {
                    return Ok(());
                };
                let request = match decode_request(&frame) {
                    Ok(request) => request,
                    Err(error) => {
                        write_message(
                            &mut writer,
                            &ServerMessage::error("invalid", "invalid_request", error.to_string()),
                        ).await?;
                        continue;
                    }
                };
                let request_id = request.request_id().to_owned();
                if matches!(&request, ClientRequest::Subscribe { .. }) {
                    subscribed = true;
                }
                let (response, emitted) = {
                    let store = store.lock().await;
                    match handle_request(&store, request) {
                        Ok(result) => result,
                        Err(error) => (
                            store_error_message(request_id, &error),
                            None,
                        ),
                    }
                };
                write_message(&mut writer, &response).await?;
                if let Some(event) = emitted {
                    let _ = events.send(event);
                }
            }
            event = event_receiver.recv() => {
                match event {
                    Ok(event) if subscribed => {
                        write_message(&mut writer, &ServerMessage::event(event)).await?
                    }
                    Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => return Ok(()),
                }
            }
        }
    }
}

fn handle_request(
    store: &EntityStore,
    request: ClientRequest,
) -> Result<(ServerMessage, Option<EntitydEvent>), StoreError> {
    let request_id = request.request_id().to_owned();
    let mut emitted = None;
    let result = match request {
        ClientRequest::ListSpaces { .. } => ResponseResult::Spaces {
            spaces: store.list_spaces()?,
        },
        ClientRequest::GetSelection { .. } => ResponseResult::Selection {
            selection: store.selected_space()?.ok_or_else(|| {
                StoreError::UnsupportedStoreSchema("selection is not initialized".into())
            })?,
        },
        ClientRequest::SelectSpace { space_id, .. } => {
            let selection = store.select_space(&space_id)?;
            emitted = Some(EntitydEvent::SelectionChanged {
                selection: selection.clone(),
            });
            ResponseResult::Selection { selection }
        }
        ClientRequest::ListEntities { space_id, .. } => ResponseResult::Entities {
            entities: store.list_entities(&space_id)?,
            space_id,
        },
        ClientRequest::CreateEntity {
            space_id,
            entity_type,
            title,
            properties,
            ..
        } => {
            let now = Utc::now();
            let entity = Entity {
                schema: SCHEMA_VERSION,
                id: Uuid::new_v4(),
                space_id,
                entity_type,
                title,
                properties,
                revision: 1,
                created_at: now,
                updated_at: now,
            };
            let event = store.create_entity(entity.clone())?;
            emitted = Some(EntitydEvent::EntityChanged {
                record: event.clone(),
            });
            ResponseResult::Entity { entity, event }
        }
        ClientRequest::UpdateEntity {
            space_id,
            entity_id,
            expected_revision,
            entity_type,
            title,
            properties,
            ..
        } => {
            let current = store
                .get_entity(&space_id, entity_id)?
                .ok_or(StoreError::EntityNotFound(entity_id))?;
            if current.revision != expected_revision {
                return Err(StoreError::RevisionConflict {
                    expected: current.revision,
                    actual: expected_revision,
                });
            }
            let entity = Entity {
                schema: SCHEMA_VERSION,
                id: entity_id,
                space_id,
                entity_type,
                title,
                properties,
                revision: current
                    .revision
                    .checked_add(1)
                    .ok_or(StoreError::RevisionExhausted(entity_id))?,
                created_at: current.created_at,
                updated_at: Utc::now(),
            };
            let event = store.update_entity(entity.clone())?;
            emitted = Some(EntitydEvent::EntityChanged {
                record: event.clone(),
            });
            ResponseResult::Entity { entity, event }
        }
        ClientRequest::DeleteEntity {
            space_id,
            entity_id,
            expected_revision,
            ..
        } => {
            let current = store
                .get_entity(&space_id, entity_id)?
                .ok_or(StoreError::EntityNotFound(entity_id))?;
            if current.revision != expected_revision {
                return Err(StoreError::RevisionConflict {
                    expected: current.revision,
                    actual: expected_revision,
                });
            }
            let revision = current
                .revision
                .checked_add(1)
                .ok_or(StoreError::RevisionExhausted(entity_id))?;
            let event = store.delete_entity(&space_id, entity_id, revision)?;
            emitted = Some(EntitydEvent::EntityChanged {
                record: event.clone(),
            });
            ResponseResult::Deleted {
                space_id,
                entity_id,
                revision,
                event,
            }
        }
        ClientRequest::Subscribe { .. } => ResponseResult::Subscribed,
    };
    Ok((ServerMessage::success(request_id, result), emitted))
}

fn store_error_message(request_id: String, error: &StoreError) -> ServerMessage {
    let code = match error {
        StoreError::SpaceNotFound(_) | StoreError::EntityNotFound(_) => "not_found",
        StoreError::RevisionConflict { .. } => "revision_conflict",
        StoreError::Validation(_) => "invalid_record",
        StoreError::CorruptLog { .. } => "store_corrupt",
        _ => "entityd_error",
    };
    ServerMessage::error(request_id, code, error.to_string())
}

async fn read_frame<R: AsyncBufRead + Unpin>(reader: &mut R) -> io::Result<Option<Vec<u8>>> {
    let mut frame = Vec::new();
    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            return if frame.is_empty() {
                Ok(None)
            } else {
                Ok(Some(frame))
            };
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let consumed = newline.map_or(available.len(), |position| position + 1);
        let content = newline.unwrap_or(available.len());
        if frame.len() + content > MAX_WIRE_MESSAGE_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "wire message exceeds size limit",
            ));
        }
        frame.extend_from_slice(&available[..content]);
        reader.consume(consumed);
        if newline.is_some() {
            if frame.last() == Some(&b'\r') {
                frame.pop();
            }
            return Ok(Some(frame));
        }
    }
}

async fn write_message<W: AsyncWrite + Unpin>(
    writer: &mut W,
    message: &ServerMessage,
) -> Result<(), ProtocolError> {
    writer.write_all(&encode_message(message)?).await?;
    Ok(())
}
