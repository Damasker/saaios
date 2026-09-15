//! S30: posts a real `saaios.notification` through `saai-shell`'s portal
//! when the player wins -- this app's own first real use of a granted
//! capability (`notifications.post`), not just the `capabilities = []`
//! plumbing-proof ADR-072 started with. Same newline-delimited-JSON,
//! connect/write/read-one-line shape as `install.rs`'s own appd client,
//! just against the portal socket and protocol instead.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

use saai_portal_protocol::{encode_request, ClientRequest, PORTAL_WIRE_SCHEMA_V1};

const PORTAL_SOCKET: &str = "/run/saaios/portal.sock";

/// Best-effort: a denied capability, a portal that isn't listening, or a
/// downed `saai-entityd` should never crash or stall the game the player
/// just won -- there is no caller here that checks or cares about the
/// result, only that it was attempted.
pub fn post_win_notification() {
    let request = ClientRequest::PostNotification {
        schema: PORTAL_WIRE_SCHEMA_V1,
        request_id: "saai-mahjong:win".to_string(),
        title: "Маджонг: победа!".to_string(),
        body: "Все пары найдены".to_string(),
    };
    let Ok(encoded) = encode_request(&request) else {
        return;
    };
    let Ok(mut stream) = UnixStream::connect(PORTAL_SOCKET) else {
        return;
    };
    if stream.write_all(&encoded).is_err() {
        return;
    }
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    let _ = reader.read_line(&mut line);
}
