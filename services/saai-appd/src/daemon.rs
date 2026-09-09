use std::fs;
use std::io;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use thiserror::Error;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{broadcast, Mutex};
use tokio::time::MissedTickBehavior;

use crate::{
    decode_request, encode_message, AppEventKind, AppState, AppStore, AppSummary, AppSupervisor,
    ClientRequest, InstalledApp, LaunchOutcome, LifecycleEvent, LifecycleEventKind, ProtocolError,
    ResponseResult, ServerMessage, StoreError, SupervisorError, MAX_WIRE_MESSAGE_BYTES,
};

const EVENT_CAPACITY: usize = 128;
const SUPERVISOR_POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, Clone)]
pub struct DaemonConfig {
    pub data_root: PathBuf,
    pub socket_path: PathBuf,
    pub runtime_dir: PathBuf,
    pub wayland_display: String,
}

#[derive(Debug, Error)]
pub enum AppdError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Supervisor(#[from] SupervisorError),
    #[error("another saai-appd is already listening at {0}")]
    AlreadyRunning(PathBuf),
    #[error("{operation} failed for {path}: {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

struct DaemonState {
    store: AppStore,
    supervisor: AppSupervisor,
}

struct SocketGuard {
    path: PathBuf,
}

impl Drop for SocketGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub async fn run_daemon(config: DaemonConfig) -> Result<(), AppdError> {
    let store = AppStore::new(&config.data_root);
    store.ensure_layout()?;
    store.scan()?;
    prepare_socket_path(&config.socket_path).await?;
    let listener = UnixListener::bind(&config.socket_path).map_err(|source| AppdError::Io {
        operation: "bind appd socket",
        path: config.socket_path.clone(),
        source,
    })?;
    let _socket_guard = SocketGuard {
        path: config.socket_path.clone(),
    };
    fs::set_permissions(&config.socket_path, fs::Permissions::from_mode(0o660)).map_err(
        |source| AppdError::Io {
            operation: "set appd socket permissions",
            path: config.socket_path.clone(),
            source,
        },
    )?;
    let supervisor = AppSupervisor::new(store.clone(), &config.runtime_dir, config.wayland_display);
    let state = Arc::new(Mutex::new(DaemonState { store, supervisor }));
    let (events, _) = broadcast::channel(EVENT_CAPACITY);
    let mut poll = tokio::time::interval(SUPERVISOR_POLL_INTERVAL);
    poll.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let (stream, _) = accepted.map_err(|source| AppdError::Io {
                    operation: "accept appd client",
                    path: config.socket_path.clone(),
                    source,
                })?;
                let client_state = Arc::clone(&state);
                let client_events = events.clone();
                tokio::spawn(async move {
                    if let Err(error) = serve_client(stream, client_state, client_events).await {
                        eprintln!("saai-appd: client disconnected with error: {error}");
                    }
                });
            }
            _ = poll.tick() => {
                let lifecycle = {
                    let mut state = state.lock().await;
                    poll_supervisor(&mut state.supervisor)?
                };
                for event in lifecycle {
                    let _ = events.send(event);
                }
            }
            signal = tokio::signal::ctrl_c() => {
                signal.map_err(|source| AppdError::Io {
                    operation: "listen for shutdown signal",
                    path: config.socket_path.clone(),
                    source,
                })?;
                return Ok(());
            }
        }
    }
}

async fn prepare_socket_path(path: &Path) -> Result<(), AppdError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| AppdError::Io {
            operation: "create appd runtime directory",
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(source) => {
            return Err(AppdError::Io {
                operation: "inspect appd socket",
                path: path.to_path_buf(),
                source,
            });
        }
    };
    if !metadata.file_type().is_socket() {
        return Err(AppdError::Io {
            operation: "refuse non-socket appd path",
            path: path.to_path_buf(),
            source: io::Error::new(io::ErrorKind::AlreadyExists, "path is not a Unix socket"),
        });
    }
    match UnixStream::connect(path).await {
        Ok(_) => Err(AppdError::AlreadyRunning(path.to_path_buf())),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::ConnectionRefused | io::ErrorKind::NotFound
            ) =>
        {
            fs::remove_file(path).map_err(|source| AppdError::Io {
                operation: "remove stale appd socket",
                path: path.to_path_buf(),
                source,
            })?;
            Ok(())
        }
        Err(source) => Err(AppdError::Io {
            operation: "probe existing appd socket",
            path: path.to_path_buf(),
            source,
        }),
    }
}

async fn serve_client(
    stream: UnixStream,
    state: Arc<Mutex<DaemonState>>,
    events: broadcast::Sender<LifecycleEvent>,
) -> Result<(), ProtocolError> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::with_capacity(8192, reader);
    let mut event_receiver = events.subscribe();

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
                let (response, emitted) = {
                    let mut state = state.lock().await;
                    match handle_request(&mut state, request) {
                        Ok(result) => result,
                        Err(error) => (
                            ServerMessage::error(request_id, "appd_error", error.to_string()),
                            Vec::new(),
                        ),
                    }
                };
                write_message(&mut writer, &response).await?;
                for event in emitted {
                    let _ = events.send(event);
                }
            }
            event = event_receiver.recv() => {
                match event {
                    Ok(event) => write_message(&mut writer, &ServerMessage::event(event)).await?,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => return Ok(()),
                }
            }
        }
    }
}

fn handle_request(
    state: &mut DaemonState,
    request: ClientRequest,
) -> Result<(ServerMessage, Vec<LifecycleEvent>), AppdError> {
    let request_id = request.request_id().to_owned();
    let mut events = Vec::new();
    let result = match request {
        ClientRequest::List { .. } => {
            let apps = state
                .store
                .scan()?
                .iter()
                .map(|app| summary(app, &state.supervisor))
                .collect();
            ResponseResult::List { apps }
        }
        ClientRequest::Install { package_path, .. } => {
            let installed = state.store.install(package_path)?;
            events.push(lifecycle(
                LifecycleEventKind::Installed,
                &installed.manifest.id,
                None,
                None,
            ));
            ResponseResult::Installed {
                app: summary(&installed, &state.supervisor),
            }
        }
        ClientRequest::Launch { app_id, .. } => {
            let outcome = state.supervisor.launch(&app_id)?;
            let (pid, existing) = match outcome {
                LaunchOutcome::Started { pid } => {
                    events.push(lifecycle(
                        LifecycleEventKind::Running,
                        &app_id,
                        Some(pid),
                        None,
                    ));
                    (pid, false)
                }
                LaunchOutcome::Existing { pid } => (pid, true),
            };
            ResponseResult::Launched {
                app_id,
                pid,
                existing,
            }
        }
        ClientRequest::Stop { app_id, .. } => {
            let was_running = state.supervisor.stop(&app_id)?;
            if was_running {
                events.push(lifecycle(LifecycleEventKind::Stopped, &app_id, None, None));
            }
            ResponseResult::Stopped {
                app_id,
                was_running,
            }
        }
        ClientRequest::Remove { app_id, .. } => {
            let was_running = state.supervisor.stop(&app_id)?;
            if was_running {
                events.push(lifecycle(LifecycleEventKind::Stopped, &app_id, None, None));
            }
            let removed = state.store.remove(&app_id)?;
            if removed {
                events.push(lifecycle(LifecycleEventKind::Removed, &app_id, None, None));
            }
            ResponseResult::Removed { app_id, removed }
        }
    };
    Ok((ServerMessage::success(request_id, result), events))
}

fn summary(app: &InstalledApp, supervisor: &AppSupervisor) -> AppSummary {
    let state = match supervisor.state(&app.manifest.id) {
        None => "installed",
        Some(AppState::Running) => "running",
        Some(AppState::Stopped) => "stopped",
        Some(AppState::CrashLimited) => "crash_limited",
    };
    AppSummary {
        id: app.manifest.id.clone(),
        name: app.manifest.name.clone(),
        version: app.manifest.version.to_string(),
        state: state.to_owned(),
        pids: supervisor.pids(&app.manifest.id),
    }
}

fn poll_supervisor(supervisor: &mut AppSupervisor) -> Result<Vec<LifecycleEvent>, AppdError> {
    supervisor
        .poll()?
        .into_iter()
        .map(|event| {
            let kind = match event.kind {
                AppEventKind::Running => LifecycleEventKind::Running,
                AppEventKind::Stopped => LifecycleEventKind::Stopped,
                AppEventKind::Crashed => LifecycleEventKind::Crashed,
                AppEventKind::CrashLimited => LifecycleEventKind::CrashLimited,
            };
            Ok(lifecycle(kind, &event.app_id, event.pid, event.exit_code))
        })
        .collect()
}

fn lifecycle(
    event: LifecycleEventKind,
    app_id: &str,
    pid: Option<u32>,
    exit_code: Option<i32>,
) -> LifecycleEvent {
    LifecycleEvent {
        event,
        app_id: app_id.to_owned(),
        pid,
        exit_code,
    }
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
    let encoded = encode_message(message)?;
    writer.write_all(&encoded).await?;
    Ok(())
}
