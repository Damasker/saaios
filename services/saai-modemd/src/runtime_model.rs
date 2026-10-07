use anyhow::{anyhow, Result};

pub const RUNTIME_REQUEST_LEN: u16 = 12;
pub const MAX_RUNTIME_FRAME: usize = 65536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeQuery {
    SimStatus,
    RadioState,
    DataRegistration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeQuerySpec {
    pub query: RuntimeQuery,
    pub request_id: u16,
    pub token: u32,
    pub min_success_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeRequest {
    pub spec: RuntimeQuerySpec,
    pub bytes: [u8; RUNTIME_REQUEST_LEN as usize],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeFrameStatus {
    Incomplete,
    Malformed,
    Complete(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeFrame {
    pub channel: u8,
    pub message_id: u16,
    pub len: usize,
    /// Solicited frames carry a token; short unsolicited frames do not.
    pub token: Option<u32>,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeObservation {
    SimStatus {
        len: usize,
        error_raw: u8,
        card_state_raw: Option<u8>,
        universal_pin_raw: Option<u8>,
        applications: Option<u8>,
    },
    RadioState {
        len: usize,
        error_raw: u8,
        radio_state_raw: Option<u32>,
    },
    DataRegistration {
        len: usize,
        error_raw: u8,
        registration_raw: Option<u8>,
        reject_cause_raw: Option<u8>,
        radio_tech_raw: Option<u8>,
    },
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RuntimeFrameReader {
    pending: Vec<u8>,
}

impl RuntimeFrameReader {
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<RuntimeFrame>> {
        self.pending.extend_from_slice(bytes);
        if self.pending.len() > MAX_RUNTIME_FRAME {
            return Err(anyhow!("runtime frame buffer exceeded"));
        }

        let mut frames = Vec::new();
        loop {
            match runtime_frame_status(&self.pending) {
                RuntimeFrameStatus::Incomplete => return Ok(frames),
                RuntimeFrameStatus::Malformed => return Err(anyhow!("malformed runtime frame")),
                RuntimeFrameStatus::Complete(len) => {
                    let frame = parse_runtime_frame(&self.pending[..len])?;
                    self.pending.drain(..len);
                    frames.push(frame);
                }
            }
        }
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }
}

pub fn runtime_query_spec(query: RuntimeQuery) -> RuntimeQuerySpec {
    match query {
        RuntimeQuery::SimStatus => RuntimeQuerySpec {
            query,
            request_id: 0x0200,
            token: 1,
            min_success_len: 15,
        },
        RuntimeQuery::RadioState => RuntimeQuerySpec {
            query,
            request_id: 0x0801,
            token: 2,
            min_success_len: 16,
        },
        RuntimeQuery::DataRegistration => RuntimeQuerySpec {
            query,
            request_id: 0x0701,
            token: 3,
            min_success_len: 16,
        },
    }
}

pub fn runtime_query_from_name(name: &str) -> Option<RuntimeQuery> {
    match name {
        "sim-status" => Some(RuntimeQuery::SimStatus),
        "radio-state" => Some(RuntimeQuery::RadioState),
        "data-registration" => Some(RuntimeQuery::DataRegistration),
        _ => None,
    }
}

/// Whether a one-shot query may open the modem endpoint.
/// `Ready` still does not send by itself. The owner and the status lock
/// both mean someone else already holds the channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryAdmission {
    Ready,
    Refuse(&'static str),
}

pub fn query_admission(
    cp_state: Option<&str>,
    owner_running: bool,
    status_lock_busy: bool,
) -> QueryAdmission {
    if owner_running {
        QueryAdmission::Refuse("owner")
    } else if status_lock_busy {
        QueryAdmission::Refuse("lock")
    } else if cp_state.map(str::trim) != Some("ONLINE") {
        QueryAdmission::Refuse("cp")
    } else {
        QueryAdmission::Ready
    }
}

pub fn report_line(observation: &RuntimeObservation) -> String {
    match observation {
        RuntimeObservation::SimStatus {
            error_raw,
            applications,
            ..
        } => {
            let sim = match applications {
                Some(0) => " sim=absent",
                Some(1..=8) => " sim=present",
                _ => "",
            };
            format!("error_raw={error_raw}{sim}")
        }
        RuntimeObservation::RadioState {
            error_raw,
            radio_state_raw,
            ..
        } => {
            let radio = if *radio_state_raw == Some(10) {
                " radio=on"
            } else {
                ""
            };
            format!("error_raw={error_raw}{radio}")
        }
        RuntimeObservation::DataRegistration {
            error_raw,
            registration_raw,
            ..
        } => {
            let registration = match registration_raw {
                Some(raw) if *raw <= 5 => format!(" registration_raw={raw}"),
                _ => String::new(),
            };
            format!("error_raw={error_raw}{registration}")
        }
    }
}

pub fn first_matching_observation(
    bytes: &[u8],
    query: RuntimeQuery,
) -> Result<Option<RuntimeObservation>> {
    let mut reader = RuntimeFrameReader::default();
    let frames = reader.push(bytes)?;
    Ok(frames
        .into_iter()
        .find_map(|frame| parse_matching_response(query, &frame)))
}

pub fn runtime_query_name(query: RuntimeQuery) -> &'static str {
    match query {
        RuntimeQuery::SimStatus => "sim-status",
        RuntimeQuery::RadioState => "radio-state",
        RuntimeQuery::DataRegistration => "data-registration",
    }
}

pub fn build_runtime_request(query: RuntimeQuery) -> RuntimeRequest {
    let spec = runtime_query_spec(query);
    let request_id = spec.request_id;
    let token = spec.token;
    let mut bytes = [0u8; RUNTIME_REQUEST_LEN as usize];
    bytes[2..4].copy_from_slice(&request_id.to_le_bytes());
    bytes[4..6].copy_from_slice(&RUNTIME_REQUEST_LEN.to_le_bytes());
    bytes[6..10].copy_from_slice(&token.to_le_bytes());
    RuntimeRequest { spec, bytes }
}

pub fn runtime_frame_status(bytes: &[u8]) -> RuntimeFrameStatus {
    if bytes.len() < 6 {
        return RuntimeFrameStatus::Incomplete;
    }
    if bytes[0] > 2 {
        return RuntimeFrameStatus::Malformed;
    }
    let min_len = if bytes[0] == 2 { 8 } else { 12 };
    let len = le_u16(&bytes[4..6]) as usize;
    if len < min_len || len > MAX_RUNTIME_FRAME {
        return RuntimeFrameStatus::Malformed;
    }
    if bytes.len() < len {
        RuntimeFrameStatus::Incomplete
    } else {
        RuntimeFrameStatus::Complete(len)
    }
}

pub fn parse_runtime_frame(bytes: &[u8]) -> Result<RuntimeFrame> {
    let RuntimeFrameStatus::Complete(len) = runtime_frame_status(bytes) else {
        return Err(anyhow!("runtime frame is not complete"));
    };
    let channel = bytes[0];
    Ok(RuntimeFrame {
        channel,
        message_id: le_u16(&bytes[2..4]),
        len,
        token: if channel == 2 {
            None
        } else {
            Some(le_u32(&bytes[6..10]))
        },
        bytes: bytes[..len].to_vec(),
    })
}

pub fn parse_matching_response(
    query: RuntimeQuery,
    frame: &RuntimeFrame,
) -> Option<RuntimeObservation> {
    let spec = runtime_query_spec(query);
    if frame.channel != 1
        || frame.message_id != spec.request_id
        || frame.token != Some(spec.token)
    {
        return None;
    }
    let error_raw = *frame.bytes.get(10)?;
    Some(match query {
        RuntimeQuery::SimStatus => RuntimeObservation::SimStatus {
            len: frame.len,
            error_raw,
            card_state_raw: successful_field(error_raw, frame, 12, spec.min_success_len),
            universal_pin_raw: successful_field(error_raw, frame, 13, spec.min_success_len),
            applications: successful_field(error_raw, frame, 14, spec.min_success_len),
        },
        RuntimeQuery::RadioState => RuntimeObservation::RadioState {
            len: frame.len,
            error_raw,
            radio_state_raw: if error_raw == 0 && frame.len >= spec.min_success_len {
                Some(le_u32(&frame.bytes[12..16]))
            } else {
                None
            },
        },
        RuntimeQuery::DataRegistration => RuntimeObservation::DataRegistration {
            len: frame.len,
            error_raw,
            registration_raw: successful_field(error_raw, frame, 12, spec.min_success_len),
            reject_cause_raw: successful_field(error_raw, frame, 13, spec.min_success_len),
            radio_tech_raw: successful_field(error_raw, frame, 15, spec.min_success_len),
        },
    })
}

fn successful_field(
    error_raw: u8,
    frame: &RuntimeFrame,
    offset: usize,
    min_success_len: usize,
) -> Option<u8> {
    if error_raw == 0 && frame.len >= min_success_len {
        frame.bytes.get(offset).copied()
    } else {
        None
    }
}

fn le_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes(bytes.try_into().expect("slice has two bytes"))
}

fn le_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes.try_into().expect("slice has four bytes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_admission_refuses_a_busy_endpoint_and_an_offline_cp() {
        assert_eq!(
            query_admission(Some("ONLINE"), true, false),
            QueryAdmission::Refuse("owner")
        );
        assert_eq!(
            query_admission(Some("ONLINE"), false, true),
            QueryAdmission::Refuse("lock")
        );
        assert_eq!(
            query_admission(Some("OFFLINE"), false, false),
            QueryAdmission::Refuse("cp")
        );
        assert_eq!(query_admission(None, false, false), QueryAdmission::Refuse("cp"));
        assert_eq!(
            query_admission(Some("ONLINE"), false, false),
            QueryAdmission::Ready
        );
    }

    #[test]
    fn report_keeps_presence_words_and_drops_pin_and_unrelated_frames() {
        let mut bytes = vec![2, 0, 0x10, 0x02, 8, 0, 0, 0];
        bytes.extend(sim_response_frame(1, 3, 1));
        let observation = first_matching_observation(&bytes, RuntimeQuery::SimStatus)
            .unwrap()
            .unwrap();
        let line = report_line(&observation);
        assert_eq!(line, "error_raw=0 sim=present");
        assert!(!line.contains('3'));

        let radio = first_matching_observation(&radio_response_frame(10), RuntimeQuery::RadioState)
            .unwrap()
            .unwrap();
        assert_eq!(report_line(&radio), "error_raw=0 radio=on");
        let other = first_matching_observation(&radio_response_frame(1), RuntimeQuery::RadioState)
            .unwrap()
            .unwrap();
        assert_eq!(report_line(&other), "error_raw=0");

        let registration = first_matching_observation(
            &registration_response_frame(1, 7, 3),
            RuntimeQuery::DataRegistration,
        )
        .unwrap()
        .unwrap();
        let line = report_line(&registration);
        assert_eq!(line, "error_raw=0 registration_raw=1");
        assert!(!line.contains('7'));
    }

    #[test]
    fn request_builders_match_reviewed_diagnostic() {
        assert_eq!(
            build_runtime_request(RuntimeQuery::SimStatus).bytes,
            [0, 0, 0x00, 0x02, 12, 0, 1, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            build_runtime_request(RuntimeQuery::RadioState).bytes,
            [0, 0, 0x01, 0x08, 12, 0, 2, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            build_runtime_request(RuntimeQuery::DataRegistration).bytes,
            [0, 0, 0x01, 0x07, 12, 0, 3, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn frame_status_matches_diagnostic_fixtures() {
        let mut sim = [0u8; 16];
        sim[0] = 1;
        sim[2..4].copy_from_slice(&0x0200u16.to_le_bytes());
        sim[4..6].copy_from_slice(&15u16.to_le_bytes());
        sim[6..10].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(runtime_frame_status(&sim[..14]), RuntimeFrameStatus::Incomplete);
        assert_eq!(runtime_frame_status(&sim[..15]), RuntimeFrameStatus::Complete(15));

        sim[4..6].copy_from_slice(&11u16.to_le_bytes());
        assert_eq!(runtime_frame_status(&sim[..15]), RuntimeFrameStatus::Malformed);
        sim[0] = 2;
        sim[4..6].copy_from_slice(&8u16.to_le_bytes());
        assert_eq!(runtime_frame_status(&sim[..15]), RuntimeFrameStatus::Complete(8));
        sim[0] = 3;
        assert_eq!(runtime_frame_status(&sim[..15]), RuntimeFrameStatus::Malformed);
    }

    #[test]
    fn short_unsolicited_frame_has_no_token() {
        let indication = [2, 0, 0x10, 0x02, 8, 0, 0, 0];
        let frame = parse_runtime_frame(&indication).unwrap();
        assert_eq!(frame.channel, 2);
        assert_eq!(frame.message_id, 0x0210);
        assert_eq!(frame.len, 8);
        assert_eq!(frame.token, None);
        assert_eq!(parse_matching_response(RuntimeQuery::SimStatus, &frame), None);
    }

    #[test]
    fn truncated_and_malformed_frames_return_errors_without_panicking() {
        let indication = [2, 0, 0x10, 0x02, 8, 0, 0, 0];
        for len in 0..indication.len() {
            assert_eq!(runtime_frame_status(&indication[..len]), RuntimeFrameStatus::Incomplete);
            assert!(parse_runtime_frame(&indication[..len]).is_err());
        }

        let response = radio_response_frame(10);
        for len in 0..response.len() {
            assert_eq!(runtime_frame_status(&response[..len]), RuntimeFrameStatus::Incomplete);
            assert!(parse_runtime_frame(&response[..len]).is_err());
        }

        let mut malformed = response;
        malformed[4..6].copy_from_slice(&11u16.to_le_bytes());
        assert_eq!(runtime_frame_status(&malformed), RuntimeFrameStatus::Malformed);
        assert!(parse_runtime_frame(&malformed).is_err());
    }

    #[test]
    fn reader_preserves_short_unsolicited_before_solicited_response() {
        let indication = [2, 0, 0x10, 0x02, 8, 0, 0, 0];
        let radio = radio_response_frame(10);
        let mut reader = RuntimeFrameReader::default();
        assert!(reader.push(&indication[..7]).unwrap().is_empty());

        let mut remainder = indication[7..].to_vec();
        remainder.extend_from_slice(&radio);
        let frames = reader.push(&remainder).unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].token, None);
        assert_eq!(frames[1].token, Some(2));
        assert_eq!(reader.pending_len(), 0);
    }

    #[test]
    fn frame_reader_preserves_split_and_coalesced_frames() {
        let radio = radio_response_frame(10);
        let registration = registration_response_frame(0, 0, 0);
        let mut reader = RuntimeFrameReader::default();

        assert!(reader.push(&radio[..5]).unwrap().is_empty());
        assert_eq!(reader.pending_len(), 5);

        let mut rest = radio[5..].to_vec();
        rest.extend_from_slice(&registration);
        let frames = reader.push(&rest).unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].message_id, 0x0801);
        assert_eq!(frames[1].message_id, 0x0701);
        assert_eq!(reader.pending_len(), 0);
    }

    #[test]
    fn response_parser_drops_unrelated_frames_without_payload_exposure() {
        let frame = parse_runtime_frame(&radio_response_frame(10)).unwrap();
        assert_eq!(
            parse_matching_response(RuntimeQuery::RadioState, &frame),
            Some(RuntimeObservation::RadioState {
                len: 16,
                error_raw: 0,
                radio_state_raw: Some(10)
            })
        );
        assert_eq!(parse_matching_response(RuntimeQuery::SimStatus, &frame), None);
    }

    #[test]
    fn response_parser_extracts_only_reviewed_registration_fields() {
        let frame = parse_runtime_frame(&registration_response_frame(0, 0, 0)).unwrap();
        assert_eq!(
            parse_matching_response(RuntimeQuery::DataRegistration, &frame),
            Some(RuntimeObservation::DataRegistration {
                len: 16,
                error_raw: 0,
                registration_raw: Some(0),
                reject_cause_raw: Some(0),
                radio_tech_raw: Some(0)
            })
        );
    }

    #[test]
    fn response_parser_keeps_error_without_interpreting_fields() {
        let mut sim = sim_response_frame(1, 3, 1);
        sim[10] = 2;
        let frame = parse_runtime_frame(&sim).unwrap();
        assert_eq!(
            parse_matching_response(RuntimeQuery::SimStatus, &frame),
            Some(RuntimeObservation::SimStatus {
                len: 15,
                error_raw: 2,
                card_state_raw: None,
                universal_pin_raw: None,
                applications: None
            })
        );
    }

    fn sim_response_frame(card: u8, pin: u8, apps: u8) -> Vec<u8> {
        let mut frame = vec![0u8; 15];
        frame[0] = 1;
        frame[2..4].copy_from_slice(&0x0200u16.to_le_bytes());
        frame[4..6].copy_from_slice(&15u16.to_le_bytes());
        frame[6..10].copy_from_slice(&1u32.to_le_bytes());
        frame[12] = card;
        frame[13] = pin;
        frame[14] = apps;
        frame
    }

    fn radio_response_frame(state: u32) -> Vec<u8> {
        let mut frame = vec![0u8; 16];
        frame[0] = 1;
        frame[2..4].copy_from_slice(&0x0801u16.to_le_bytes());
        frame[4..6].copy_from_slice(&16u16.to_le_bytes());
        frame[6..10].copy_from_slice(&2u32.to_le_bytes());
        frame[12..16].copy_from_slice(&state.to_le_bytes());
        frame
    }

    fn registration_response_frame(registration: u8, reject: u8, tech: u8) -> Vec<u8> {
        let mut frame = vec![0u8; 16];
        frame[0] = 1;
        frame[2..4].copy_from_slice(&0x0701u16.to_le_bytes());
        frame[4..6].copy_from_slice(&16u16.to_le_bytes());
        frame[6..10].copy_from_slice(&3u32.to_le_bytes());
        frame[12] = registration;
        frame[13] = reject;
        frame[15] = tech;
        frame
    }
}
