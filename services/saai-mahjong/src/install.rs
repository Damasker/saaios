//! `saai-mahjong --install [package-dir] [appd-socket]`: the same
//! binary that plays the game also knows how to install itself (and,
//! generically, any other package directory) through `saai-appd`'s
//! real `Install` RPC. Folded into this crate instead of a separate
//! `saai-install-app` binary purely to save image size on the fixed
//! 8MB `init_boot` partition (ADR: "install a third-party app" spike)
//! -- two copies of `saai-app-protocol`/`serde_json` statically
//! linked into two small binaries cost more than one `--install`
//! branch in this one. Newline-delimited JSON, the same wire format
//! `saai-shell`'s own `appd_client` already uses -- this is a second,
//! independent client of the same socket, not a special-cased path.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use saai_app_protocol::{
    decode_request, encode_request, ClientRequest, ServerMessage, APPD_WIRE_SCHEMA_V1,
};

const DEFAULT_PACKAGE_DIR: &str = "/saaios/packages/org.saaios.mahjong";
const DEFAULT_SOCKET: &str = "/run/saaios/appd.sock";

pub fn run(mut args: impl Iterator<Item = String>) -> bool {
    let package_path = args
        .next()
        .unwrap_or_else(|| DEFAULT_PACKAGE_DIR.to_string());
    let socket_path = args.next().unwrap_or_else(|| DEFAULT_SOCKET.to_string());

    let request = ClientRequest::Install {
        schema: APPD_WIRE_SCHEMA_V1,
        request_id: "saai-mahjong:install".to_string(),
        package_path: PathBuf::from(&package_path),
    };
    let encoded = match encode_request(&request) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("saai-mahjong --install: failed to encode request: {error}");
            return false;
        }
    };
    // Round-trip through decode_request so a malformed request_id/
    // schema is caught locally rather than after a real socket
    // round-trip.
    if let Err(error) = decode_request(&encoded[..encoded.len() - 1]) {
        eprintln!("saai-mahjong --install: request failed local validation: {error}");
        return false;
    }

    let mut stream = match UnixStream::connect(&socket_path) {
        Ok(stream) => stream,
        Err(error) => {
            eprintln!("saai-mahjong --install: connect {socket_path}: {error}");
            return false;
        }
    };
    if let Err(error) = stream.write_all(&encoded) {
        eprintln!("saai-mahjong --install: write request: {error}");
        return false;
    }

    let mut reader = BufReader::new(match stream.try_clone() {
        Ok(clone) => clone,
        Err(error) => {
            eprintln!("saai-mahjong --install: clone socket: {error}");
            return false;
        }
    });
    let mut line = String::new();
    if let Err(error) = reader.read_line(&mut line) {
        eprintln!("saai-mahjong --install: read response: {error}");
        return false;
    }

    match serde_json::from_str::<ServerMessage>(&line) {
        Ok(ServerMessage::Response {
            ok: true, result, ..
        }) => {
            println!("saai-mahjong --install: OK: {result:?}");
            true
        }
        Ok(ServerMessage::Response {
            ok: false,
            error: Some(error),
            ..
        }) => {
            eprintln!("saai-mahjong --install: appd refused: {error:?}");
            false
        }
        Ok(other) => {
            eprintln!("saai-mahjong --install: unexpected response: {other:?}");
            false
        }
        Err(error) => {
            eprintln!("saai-mahjong --install: malformed response {line:?}: {error}");
            false
        }
    }
}
