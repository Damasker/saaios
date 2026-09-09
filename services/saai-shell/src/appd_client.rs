use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use saai_app_protocol::{
    encode_request, ClientRequest, ServerMessage, APPD_WIRE_SCHEMA_V1, MAX_WIRE_MESSAGE_BYTES,
};

const RETRY_DELAY: Duration = Duration::from_millis(500);

pub struct AppdClient {
    socket_path: PathBuf,
    stream: Option<UnixStream>,
    read_buffer: Vec<u8>,
    write_buffer: Vec<u8>,
    next_request: u64,
    retry_at: Instant,
}

impl AppdClient {
    pub fn new(socket_path: impl Into<PathBuf>) -> Self {
        Self {
            socket_path: socket_path.into(),
            stream: None,
            read_buffer: Vec::new(),
            write_buffer: Vec::new(),
            next_request: 1,
            retry_at: Instant::now(),
        }
    }

    pub fn list(&mut self) {
        let request_id = self.request_id();
        self.queue(ClientRequest::List {
            schema: APPD_WIRE_SCHEMA_V1,
            request_id,
        });
    }

    pub fn install(&mut self, package_path: impl Into<PathBuf>) {
        let request_id = self.request_id();
        self.queue(ClientRequest::Install {
            schema: APPD_WIRE_SCHEMA_V1,
            request_id,
            package_path: package_path.into(),
        });
    }

    pub fn launch(&mut self, app_id: impl Into<String>) {
        let request_id = self.request_id();
        self.queue(ClientRequest::Launch {
            schema: APPD_WIRE_SCHEMA_V1,
            request_id,
            app_id: app_id.into(),
        });
    }

    pub fn poll(&mut self) -> Vec<ServerMessage> {
        if self.stream.is_none() && Instant::now() >= self.retry_at {
            self.connect();
        }
        if self.stream.is_none() {
            return Vec::new();
        }

        if let Err(error) = self.flush() {
            self.disconnect(error);
            return Vec::new();
        }
        match self.read_messages() {
            Ok(messages) => messages,
            Err(error) => {
                self.disconnect(error);
                Vec::new()
            }
        }
    }

    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    fn request_id(&mut self) -> String {
        let request_id = format!("shell:{}", self.next_request);
        self.next_request = self.next_request.wrapping_add(1).max(1);
        request_id
    }

    fn queue(&mut self, request: ClientRequest) {
        match encode_request(&request) {
            Ok(encoded) => self.write_buffer.extend(encoded),
            Err(error) => eprintln!("saai-shell: failed to encode appd request: {error}"),
        }
    }

    fn connect(&mut self) {
        match UnixStream::connect(&self.socket_path) {
            Ok(stream) => {
                if let Err(error) = stream.set_nonblocking(true) {
                    self.disconnect(error);
                    return;
                }
                self.stream = Some(stream);
                self.read_buffer.clear();
                println!("saai-shell: connected to saai-appd");
                // The registry is authoritative after a reconnect. Commands
                // are safe to reconcile from this fresh snapshot.
                self.list();
            }
            Err(error) => self.disconnect(error),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        while !self.write_buffer.is_empty() {
            let Some(stream) = self.stream.as_mut() else {
                return Ok(());
            };
            match stream.write(&self.write_buffer) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "appd socket closed",
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

    fn read_messages(&mut self) -> io::Result<Vec<ServerMessage>> {
        let mut chunk = [0_u8; 4096];
        while let Some(stream) = self.stream.as_mut() {
            match stream.read(&mut chunk) {
                Ok(0) if self.read_buffer.is_empty() => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "appd socket closed",
                    ))
                }
                Ok(0) => break,
                Ok(read) => {
                    self.read_buffer.extend_from_slice(&chunk[..read]);
                    if self.read_buffer.len() > MAX_WIRE_MESSAGE_BYTES {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "appd response exceeds wire limit",
                        ));
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => return Err(error),
            }
        }

        let mut messages = Vec::new();
        while let Some(newline) = self.read_buffer.iter().position(|byte| *byte == b'\n') {
            let mut frame: Vec<u8> = self.read_buffer.drain(..=newline).collect();
            frame.pop();
            if frame.last() == Some(&b'\r') {
                frame.pop();
            }
            let message = serde_json::from_slice(&frame)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            messages.push(message);
        }
        Ok(messages)
    }

    fn disconnect(&mut self, error: io::Error) {
        if self.stream.take().is_some() {
            eprintln!("saai-shell: saai-appd disconnected: {error}");
        }
        self.read_buffer.clear();
        // A partially written command cannot safely be resumed on another
        // connection. Registry refresh on reconnect reconciles the UI.
        self.write_buffer.clear();
        self.retry_at = Instant::now() + RETRY_DELAY;
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;
    use std::thread;
    use std::time::{Duration, Instant};

    use saai_app_protocol::{ClientRequest, ResponseResult, ServerMessage};

    use super::AppdClient;

    #[test]
    fn reconnect_lists_and_delivers_typed_response() {
        let temporary = tempfile::tempdir().unwrap();
        let socket = temporary.path().join("appd.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let request: ClientRequest = serde_json::from_str(&line).unwrap();
            assert!(matches!(request, ClientRequest::List { .. }));
            let request_id = request.request_id().to_owned();
            let response =
                ServerMessage::success(request_id, ResponseResult::List { apps: Vec::new() });
            serde_json::to_writer(&stream, &response).unwrap();
            writeln!(&stream).unwrap();
        });

        let mut client = AppdClient::new(&socket);
        let deadline = Instant::now() + Duration::from_secs(2);
        let message = loop {
            if let Some(message) = client.poll().into_iter().next() {
                break message;
            }
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(10));
        };
        assert!(client.is_connected());
        assert!(matches!(
            message,
            ServerMessage::Response {
                result: Some(ResponseResult::List { apps }),
                ..
            } if apps.is_empty()
        ));
        server.join().unwrap();
    }
}
