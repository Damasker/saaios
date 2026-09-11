//! Minimal async client for `saai-entityd`'s newline-delimited JSON wire
//! protocol (`saai-entity-protocol`). `saai-shell`'s `entityd_client.rs`
//! already has one of these, but it's built around a non-blocking
//! poll loop tuned for a Wayland event loop that must never block.
//! `saai-taskd` has no such constraint -- it's a plain tokio process
//! that only ever wants one request in flight at a time -- so this is
//! a smaller, straightforwardly sequential request/response client
//! instead of reusing that one.

use saai_entity_protocol::{
    encode_request, ClientRequest, Entity, EntitydEvent, ProtocolError, ResponseResult,
    ServerMessage, ENTITYD_WIRE_SCHEMA_V1,
};
use serde_json::{Map, Value};
use std::collections::VecDeque;
use std::path::Path;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::UnixStream;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error("entityd connection failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("malformed entityd message: {0}")]
    Json(#[from] serde_json::Error),
    #[error("entityd rejected request {request_id}: {code}: {message}")]
    Wire {
        request_id: String,
        code: String,
        message: String,
    },
    #[error("entityd closed the connection")]
    Closed,
    #[error("unexpected entityd response shape: {0}")]
    UnexpectedResult(String),
}

pub struct EntitydConn {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
    next_request: u64,
    pending_events: VecDeque<EntitydEvent>,
}

impl EntitydConn {
    pub async fn connect(path: &Path) -> Result<Self, ClientError> {
        let stream = UnixStream::connect(path).await?;
        let (read_half, write_half) = stream.into_split();
        Ok(Self {
            reader: BufReader::new(read_half),
            writer: write_half,
            next_request: 1,
            pending_events: VecDeque::new(),
        })
    }

    fn request_id(&mut self) -> String {
        let id = format!("taskd:{}", self.next_request);
        self.next_request = self.next_request.wrapping_add(1).max(1);
        id
    }

    async fn read_message(&mut self) -> Result<ServerMessage, ClientError> {
        let mut line = String::new();
        let bytes = self.reader.read_line(&mut line).await?;
        if bytes == 0 {
            return Err(ClientError::Closed);
        }
        Ok(serde_json::from_str(line.trim_end())?)
    }

    /// Sends one request and waits for its matching response, buffering
    /// any `Subscribe`-driven `Event` frames that arrive interleaved on
    /// the same connection for `next_event`/`take_events` to hand out
    /// afterwards. Safe only because this client never has more than
    /// one request in flight -- there's nothing here to mismatch
    /// against.
    async fn call(&mut self, request: ClientRequest) -> Result<ResponseResult, ClientError> {
        let request_id = request.request_id().to_owned();
        let encoded = encode_request(&request)?;
        self.writer.write_all(&encoded).await?;
        loop {
            match self.read_message().await? {
                ServerMessage::Event { event, .. } => self.pending_events.push_back(event),
                ServerMessage::Response {
                    request_id: reply_id,
                    ok,
                    result,
                    error,
                    ..
                } if reply_id == request_id => {
                    if ok {
                        return Ok(*result.ok_or_else(|| {
                            ClientError::UnexpectedResult("ok response with no result".into())
                        })?);
                    }
                    let error = error.ok_or_else(|| {
                        ClientError::UnexpectedResult("error response with no error".into())
                    })?;
                    return Err(ClientError::Wire {
                        request_id: reply_id,
                        code: error.code,
                        message: error.message,
                    });
                }
                // A response to some earlier, already-abandoned request
                // id can't happen in this sequential client -- ignored
                // rather than treated as fatal, matching entityd's own
                // tolerance for out-of-order frames.
                ServerMessage::Response { .. } => continue,
            }
        }
    }

    /// Blocks until the next `Subscribe`-driven event, draining any
    /// already buffered by a previous `call()` first.
    pub async fn next_event(&mut self) -> Result<EntitydEvent, ClientError> {
        if let Some(event) = self.pending_events.pop_front() {
            return Ok(event);
        }
        loop {
            match self.read_message().await? {
                ServerMessage::Event { event, .. } => return Ok(event),
                ServerMessage::Response { .. } => continue,
            }
        }
    }

    pub async fn subscribe(&mut self) -> Result<(), ClientError> {
        let request_id = self.request_id();
        self.call(ClientRequest::Subscribe {
            schema: ENTITYD_WIRE_SCHEMA_V1,
            request_id,
        })
        .await?;
        Ok(())
    }

    pub async fn list_entities(&mut self, space_id: &str) -> Result<Vec<Entity>, ClientError> {
        let request_id = self.request_id();
        match self
            .call(ClientRequest::ListEntities {
                schema: ENTITYD_WIRE_SCHEMA_V1,
                request_id,
                space_id: space_id.to_string(),
            })
            .await?
        {
            ResponseResult::Entities { entities, .. } => Ok(entities),
            other => Err(ClientError::UnexpectedResult(format!("{other:?}"))),
        }
    }

    pub async fn create_entity(
        &mut self,
        space_id: &str,
        entity_type: &str,
        title: &str,
        properties: Map<String, Value>,
    ) -> Result<Entity, ClientError> {
        let request_id = self.request_id();
        match self
            .call(ClientRequest::CreateEntity {
                schema: ENTITYD_WIRE_SCHEMA_V1,
                request_id,
                space_id: space_id.to_string(),
                entity_type: entity_type.to_string(),
                title: title.to_string(),
                properties,
            })
            .await?
        {
            ResponseResult::Entity { entity, .. } => Ok(entity),
            other => Err(ClientError::UnexpectedResult(format!("{other:?}"))),
        }
    }

    pub async fn update_entity(
        &mut self,
        entity: &Entity,
        properties: Map<String, Value>,
    ) -> Result<Entity, ClientError> {
        let request_id = self.request_id();
        match self
            .call(ClientRequest::UpdateEntity {
                schema: ENTITYD_WIRE_SCHEMA_V1,
                request_id,
                space_id: entity.space_id.clone(),
                entity_id: entity.id,
                expected_revision: entity.revision,
                entity_type: entity.entity_type.clone(),
                title: entity.title.clone(),
                properties,
            })
            .await?
        {
            ResponseResult::Entity { entity, .. } => Ok(entity),
            other => Err(ClientError::UnexpectedResult(format!("{other:?}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader as StdBufReader, Write};
    use std::os::unix::net::{UnixListener, UnixStream as StdUnixStream};
    use std::thread;

    /// Runs `server` against one accepted connection on a fresh socket,
    /// then runs `client_task` against a real `EntitydConn` to that same
    /// socket -- the same "real Unix socket, fake peer" shape
    /// `entityd_client.rs`'s own test already uses, adapted for this
    /// client's async/tokio side.
    async fn with_fake_entityd<S, C, T>(server: S, client_task: C) -> T
    where
        S: FnOnce(StdUnixStream) + Send + 'static,
        C: FnOnce(EntitydConn) -> std::pin::Pin<Box<dyn std::future::Future<Output = T>>>,
    {
        let temp = tempfile::tempdir().unwrap();
        let socket = temp.path().join("entityd.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            server(stream);
        });
        let conn = EntitydConn::connect(&socket).await.unwrap();
        let result = client_task(conn).await;
        handle.join().unwrap();
        result
    }

    fn respond(stream: &StdUnixStream, message: &ServerMessage) {
        let mut encoded = serde_json::to_vec(message).unwrap();
        encoded.push(b'\n');
        (&*stream).write_all(&encoded).unwrap();
    }

    fn read_request(reader: &mut impl BufRead) -> ClientRequest {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    }

    #[tokio::test]
    async fn subscribe_succeeds_on_a_plain_ok_response() {
        with_fake_entityd(
            |stream| {
                let mut reader = StdBufReader::new(stream.try_clone().unwrap());
                let request = read_request(&mut reader);
                assert!(matches!(request, ClientRequest::Subscribe { .. }));
                respond(
                    &stream,
                    &ServerMessage::success(request.request_id(), ResponseResult::Subscribed),
                );
            },
            |mut conn| {
                Box::pin(async move {
                    conn.subscribe().await.unwrap();
                })
            },
        )
        .await;
    }

    #[tokio::test]
    async fn wire_error_surfaces_as_a_typed_error_not_a_panic() {
        with_fake_entityd(
            |stream| {
                let mut reader = StdBufReader::new(stream.try_clone().unwrap());
                let request = read_request(&mut reader);
                respond(
                    &stream,
                    &ServerMessage::error(request.request_id(), "revision_conflict", "stale"),
                );
            },
            |mut conn| {
                Box::pin(async move {
                    let error = conn.list_entities("home").await.unwrap_err();
                    match error {
                        ClientError::Wire { code, .. } => assert_eq!(code, "revision_conflict"),
                        other => panic!("expected a Wire error, got {other:?}"),
                    }
                })
            },
        )
        .await;
    }

    #[tokio::test]
    async fn an_interleaved_subscribe_event_does_not_derail_the_pending_call() {
        with_fake_entityd(
            |stream| {
                let mut reader = StdBufReader::new(stream.try_clone().unwrap());
                let request = read_request(&mut reader);
                // A live EntityChanged event arrives before this
                // request's own response -- exactly what a real
                // subscribed connection can interleave.
                respond(
                    &stream,
                    &ServerMessage::event(EntitydEvent::SelectionChanged {
                        selection: saai_entity_protocol::SpaceSelection {
                            schema: 1,
                            space_id: "home".into(),
                            source: saai_entity_store::SelectionSource::User,
                            updated_at: chrono::Utc::now(),
                        },
                    }),
                );
                respond(
                    &stream,
                    &ServerMessage::success(
                        request.request_id(),
                        ResponseResult::Entities {
                            space_id: "home".into(),
                            entities: vec![],
                        },
                    ),
                );
            },
            |mut conn| {
                Box::pin(async move {
                    let entities = conn.list_entities("home").await.unwrap();
                    assert!(entities.is_empty());
                    // The interleaved event wasn't dropped -- it's
                    // sitting in the buffer for the caller to consume.
                    assert!(matches!(
                        conn.next_event().await.unwrap(),
                        EntitydEvent::SelectionChanged { .. }
                    ));
                })
            },
        )
        .await;
    }
}
