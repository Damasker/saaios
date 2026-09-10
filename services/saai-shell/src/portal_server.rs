//! ADR-020 section 8 / S07 Change 7: `saai-shell` is the trust boundary
//! sandboxed apps go through for `clipboard.read`/`clipboard.write` and
//! `portal.open_file` -- the same role it already holds for session
//! lock/focus (ADR-015). This is deliberately scoped as an entry point,
//! not a finished portal system (see the S07 sprint doc's Scope): the
//! clipboard round trip is real end to end, `open_file` is wired through
//! the same request/response/capability-check path but always answers
//! `not_implemented` -- there is no file-picker UI yet.
//!
//! The socket lives under `/run/saaios/`, which `saai-appd`'s sandbox
//! (`services/saai-appd/src/sandbox.rs`) never masks, so a sandboxed app
//! reaches it exactly the way it already reaches `saai-appd`'s and
//! `saai-displayd`'s sockets -- no new sandbox path needed for this.
//!
//! Authorization has nothing to do with which socket path an app can see,
//! though: any process on the system can open a Unix socket at a known
//! path. The actual check is `SO_PEERCRED` (via `nix`'s `getsockopt(..,
//! PeerCredentials)` -- std's own `UnixStream::peer_cred()` is still
//! unstable on this toolchain, see the Cargo.toml comment) at accept time,
//! resolving the connecting process's pid to an app_id via
//! `Shell`'s own cache of `saai-appd`'s last `list()` response, then
//! checking that app_id's `granted_capabilities` -- a request from a pid
//! that isn't a currently known running app (an arbitrary shell command,
//! for instance) is refused before any capability check even runs.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

use nix::sys::socket::{getsockopt, sockopt::PeerCredentials};
use saai_portal_protocol::{
    decode_request, encode_message, ClientRequest, ResponseResult, ServerMessage,
    MAX_CLIPBOARD_TEXT_BYTES, MAX_WIRE_MESSAGE_BYTES,
};

/// Matches `Capability::as_str()` in saai-appd (ADR-020's vocabulary).
/// `saai-shell` only speaks `saai-appd`'s wire protocol, not its Rust
/// types, so these six names are duplicated here as plain strings rather
/// than pulling in `saai-appd` as a library dependency for this alone.
const CAP_CLIPBOARD_READ: &str = "clipboard.read";
const CAP_CLIPBOARD_WRITE: &str = "clipboard.write";
const CAP_PORTAL_OPEN_FILE: &str = "portal.open_file";

pub struct PortalServer {
    listener: UnixListener,
    socket_path: PathBuf,
    connections: Vec<Connection>,
}

struct Connection {
    stream: UnixStream,
    /// Resolved once at accept time via `SO_PEERCRED` -- the portal's only
    /// notion of "who is asking". `None` when the credential lookup itself
    /// failed, kept (not dropped) so the request still gets a clean
    /// `unknown_peer` error instead of the connection silently vanishing.
    peer_pid: Option<u32>,
    read_buffer: Vec<u8>,
    write_buffer: Vec<u8>,
}

impl PortalServer {
    pub fn bind(socket_path: impl Into<PathBuf>) -> io::Result<Self> {
        let socket_path = socket_path.into();
        if let Some(parent) = socket_path.parent() {
            fs::create_dir_all(parent)?;
        }
        // Best-effort: a stale socket file left by a previous saai-shell
        // process. Unlike saai-appd, saai-shell has no "am I already
        // running" check of its own to reuse here -- there is exactly one
        // shell for the whole device, started once by native-init.c.
        let _ = fs::remove_file(&socket_path);
        let listener = UnixListener::bind(&socket_path)?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            listener,
            socket_path,
            connections: Vec::new(),
        })
    }

    /// Accepts pending connections and services every open one against the
    /// current `apps_by_pid`/`apps_grants` snapshot and the shared
    /// in-memory `clipboard` (no persistence -- cleared on `saai-shell`
    /// restart, same as a real desktop clipboard would be on logout).
    pub fn poll(
        &mut self,
        apps_by_pid: &BTreeMap<u32, String>,
        apps_grants: &BTreeMap<String, Vec<String>>,
        clipboard: &mut Option<String>,
    ) {
        self.accept_pending();
        self.connections
            .retain_mut(|connection| connection.service(apps_by_pid, apps_grants, clipboard));
    }

    fn accept_pending(&mut self) {
        loop {
            match self.listener.accept() {
                Ok((stream, _addr)) => {
                    if let Err(error) = stream.set_nonblocking(true) {
                        eprintln!("saai-shell: portal connection setup failed: {error}");
                        continue;
                    }
                    let peer_pid = getsockopt(&stream, PeerCredentials)
                        .ok()
                        .map(|credentials| credentials.pid() as u32);
                    self.connections.push(Connection {
                        stream,
                        peer_pid,
                        read_buffer: Vec::new(),
                        write_buffer: Vec::new(),
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => {
                    eprintln!("saai-shell: portal accept failed: {error}");
                    break;
                }
            }
        }
    }
}

impl Drop for PortalServer {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket_path);
    }
}

impl Connection {
    /// Returns false when the connection should be dropped (closed by the
    /// peer, or broke the wire protocol in a way that isn't recoverable).
    fn service(
        &mut self,
        apps_by_pid: &BTreeMap<u32, String>,
        apps_grants: &BTreeMap<String, Vec<String>>,
        clipboard: &mut Option<String>,
    ) -> bool {
        if self.flush().is_err() {
            return false;
        }
        if self.read_available().is_err() {
            return false;
        }
        while let Some(newline) = self.read_buffer.iter().position(|byte| *byte == b'\n') {
            let mut frame: Vec<u8> = self.read_buffer.drain(..=newline).collect();
            frame.pop();
            if frame.last() == Some(&b'\r') {
                frame.pop();
            }
            let response = self.handle_frame(&frame, apps_by_pid, apps_grants, clipboard);
            match encode_message(&response) {
                Ok(encoded) => self.write_buffer.extend(encoded),
                Err(error) => eprintln!("saai-shell: failed to encode portal response: {error}"),
            }
        }
        self.flush().is_ok()
    }

    fn handle_frame(
        &self,
        frame: &[u8],
        apps_by_pid: &BTreeMap<u32, String>,
        apps_grants: &BTreeMap<String, Vec<String>>,
        clipboard: &mut Option<String>,
    ) -> ServerMessage {
        let request = match decode_request(frame) {
            Ok(request) => request,
            Err(error) => {
                return ServerMessage::error("invalid", "invalid_request", error.to_string())
            }
        };
        let request_id = request.request_id().to_owned();

        let Some(app_id) = self.peer_pid.and_then(|pid| apps_by_pid.get(&pid)) else {
            return ServerMessage::error(
                request_id,
                "unknown_peer",
                "connecting process is not a currently running installed app",
            );
        };
        let granted = apps_grants.get(app_id).map(Vec::as_slice).unwrap_or(&[]);

        let required = match &request {
            ClientRequest::ClipboardRead { .. } => CAP_CLIPBOARD_READ,
            ClientRequest::ClipboardWrite { .. } => CAP_CLIPBOARD_WRITE,
            ClientRequest::OpenFile { .. } => CAP_PORTAL_OPEN_FILE,
        };
        if !granted.iter().any(|capability| capability == required) {
            return ServerMessage::error(
                request_id,
                "capability_denied",
                format!("{app_id} has not been granted {required}"),
            );
        }

        match request {
            ClientRequest::ClipboardRead { .. } => ServerMessage::success(
                request_id,
                ResponseResult::ClipboardText {
                    text: clipboard.clone().unwrap_or_default(),
                },
            ),
            ClientRequest::ClipboardWrite { text, .. } => {
                if text.len() > MAX_CLIPBOARD_TEXT_BYTES {
                    return ServerMessage::error(
                        request_id,
                        "clipboard_too_large",
                        format!(
                            "text exceeds the {MAX_CLIPBOARD_TEXT_BYTES}-byte clipboard limit"
                        ),
                    );
                }
                *clipboard = Some(text);
                ServerMessage::success(request_id, ResponseResult::ClipboardWritten)
            }
            ClientRequest::OpenFile { .. } => ServerMessage::error(
                request_id,
                "not_implemented",
                "portal.open_file is a recorded entry point only -- the file picker flow is not built yet",
            ),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        while !self.write_buffer.is_empty() {
            match self.stream.write(&self.write_buffer) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "portal client closed",
                    ))
                }
                Ok(written) => {
                    self.write_buffer.drain(..written);
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    fn read_available(&mut self) -> io::Result<()> {
        let mut chunk = [0_u8; 4096];
        loop {
            match self.stream.read(&mut chunk) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "portal client closed",
                    ))
                }
                Ok(read) => {
                    self.read_buffer.extend_from_slice(&chunk[..read]);
                    if self.read_buffer.len() > MAX_WIRE_MESSAGE_BYTES {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "portal request exceeds wire limit",
                        ));
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                Err(error) => return Err(error),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;
    use std::time::{Duration, Instant};

    use saai_portal_protocol::{
        decode_request as decode_portal_request, encode_request, ClientRequest, ResponseResult,
        ServerMessage, PORTAL_WIRE_SCHEMA_V1,
    };

    use super::PortalServer;

    fn roundtrip(
        server: &mut PortalServer,
        apps_by_pid: &BTreeMap<u32, String>,
        apps_grants: &BTreeMap<String, Vec<String>>,
        clipboard: &mut Option<String>,
        client: &mut UnixStream,
        request: &ClientRequest,
    ) -> ServerMessage {
        client.write_all(&encode_request(request).unwrap()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            server.poll(apps_by_pid, apps_grants, clipboard);
            client.set_nonblocking(true).unwrap();
            let mut reader = BufReader::new(&*client);
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => panic!("portal socket closed before answering"),
                Ok(_) if !line.is_empty() => return serde_json::from_str(&line).unwrap(),
                _ => {
                    assert!(Instant::now() < deadline, "portal server never answered");
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
        }
    }

    #[test]
    fn unknown_peer_is_refused_before_any_capability_check() {
        let temporary = tempfile::tempdir().unwrap();
        let socket = temporary.path().join("portal.sock");
        let mut server = PortalServer::bind(&socket).unwrap();
        let mut client = UnixStream::connect(&socket).unwrap();

        // No app in apps_by_pid can match this test process's own pid.
        let response = roundtrip(
            &mut server,
            &BTreeMap::new(),
            &BTreeMap::new(),
            &mut None,
            &mut client,
            &ClientRequest::ClipboardRead {
                schema: PORTAL_WIRE_SCHEMA_V1,
                request_id: "t1".into(),
            },
        );
        assert!(matches!(
            response,
            ServerMessage::Response { ok: false, error: Some(error), .. }
                if error.code == "unknown_peer"
        ));
    }

    #[test]
    fn known_app_without_grant_is_denied() {
        let temporary = tempfile::tempdir().unwrap();
        let socket = temporary.path().join("portal.sock");
        let mut server = PortalServer::bind(&socket).unwrap();
        let mut client = UnixStream::connect(&socket).unwrap();

        let mut apps_by_pid = BTreeMap::new();
        apps_by_pid.insert(std::process::id(), "org.saaios.demo-surface".to_string());
        let apps_grants = BTreeMap::new(); // no capabilities on file at all

        let response = roundtrip(
            &mut server,
            &apps_by_pid,
            &apps_grants,
            &mut None,
            &mut client,
            &ClientRequest::ClipboardRead {
                schema: PORTAL_WIRE_SCHEMA_V1,
                request_id: "t2".into(),
            },
        );
        assert!(matches!(
            response,
            ServerMessage::Response { ok: false, error: Some(error), .. }
                if error.code == "capability_denied"
        ));
    }

    #[test]
    fn authorized_clipboard_write_then_read_round_trips_real_text() {
        let temporary = tempfile::tempdir().unwrap();
        let socket = temporary.path().join("portal.sock");
        let mut server = PortalServer::bind(&socket).unwrap();
        let mut client = UnixStream::connect(&socket).unwrap();

        let mut apps_by_pid = BTreeMap::new();
        apps_by_pid.insert(std::process::id(), "org.saaios.demo-surface".to_string());
        let mut apps_grants = BTreeMap::new();
        apps_grants.insert(
            "org.saaios.demo-surface".to_string(),
            vec!["clipboard.read".to_string(), "clipboard.write".to_string()],
        );
        let mut clipboard = None;

        let write_response = roundtrip(
            &mut server,
            &apps_by_pid,
            &apps_grants,
            &mut clipboard,
            &mut client,
            &ClientRequest::ClipboardWrite {
                schema: PORTAL_WIRE_SCHEMA_V1,
                request_id: "t3".into(),
                text: "hello from the sandbox".into(),
            },
        );
        assert!(matches!(
            write_response,
            ServerMessage::Response {
                ok: true,
                result: Some(ResponseResult::ClipboardWritten),
                ..
            }
        ));

        let read_response = roundtrip(
            &mut server,
            &apps_by_pid,
            &apps_grants,
            &mut clipboard,
            &mut client,
            &ClientRequest::ClipboardRead {
                schema: PORTAL_WIRE_SCHEMA_V1,
                request_id: "t4".into(),
            },
        );
        assert!(matches!(
            read_response,
            ServerMessage::Response {
                ok: true,
                result: Some(ResponseResult::ClipboardText { text }),
                ..
            } if text == "hello from the sandbox"
        ));
    }

    #[test]
    fn open_file_is_a_typed_entry_point_not_a_working_flow() {
        let temporary = tempfile::tempdir().unwrap();
        let socket = temporary.path().join("portal.sock");
        let mut server = PortalServer::bind(&socket).unwrap();
        let mut client = UnixStream::connect(&socket).unwrap();

        let mut apps_by_pid = BTreeMap::new();
        apps_by_pid.insert(std::process::id(), "org.saaios.demo-surface".to_string());
        let mut apps_grants = BTreeMap::new();
        apps_grants.insert(
            "org.saaios.demo-surface".to_string(),
            vec!["portal.open_file".to_string()],
        );

        let response = roundtrip(
            &mut server,
            &apps_by_pid,
            &apps_grants,
            &mut None,
            &mut client,
            &ClientRequest::OpenFile {
                schema: PORTAL_WIRE_SCHEMA_V1,
                request_id: "t5".into(),
            },
        );
        assert!(matches!(
            response,
            ServerMessage::Response { ok: false, error: Some(error), .. }
                if error.code == "not_implemented"
        ));
        // Sanity: the fixture round-trips through the real decoder too, not
        // just this test's own client-side construction.
        let encoded = encode_request(&ClientRequest::OpenFile {
            schema: PORTAL_WIRE_SCHEMA_V1,
            request_id: "t6".into(),
        })
        .unwrap();
        assert!(decode_portal_request(&encoded[..encoded.len() - 1]).is_ok());
    }
}
