use anyhow::{anyhow, Result};

pub const TOC_ENTRY_SIZE: usize = 32;
pub const SIT_START: u32 = 0x0000_a100;
pub const SIT_START_ACK: u32 = 0x0000_c100;
pub const SIT_BIN: u32 = 0x0000_a10b;
pub const SIT_BIN_ACK: u32 = 0x0000_c10b;
pub const SIT_CRC: u32 = 0x0000_a301;
pub const SIT_CRC_ACK: u32 = 0x0000_c300;
pub const SIT_DONE: u32 = 0x0000_a10d;
pub const SIT_DONE_ACK: u32 = 0x0000_c10d;
pub const SIT_READY: u32 = 0x0000_a00b;
pub const SIT_READY_ACK: u32 = 0x0000_c00b;
pub const SIT_FIN: u32 = 0x0000_a400;
pub const SIT_FIN_ACK: u32 = 0x0000_c400;
pub const SIT_CHUNK: u32 = 0x7e8;
pub const EXYNOS_HEADER_SIZE: u32 = 12;
pub const SIT_HEADER_SIZE: u32 = 12;
pub const LEGACY_RAW_TXQ_SIZE: u32 = 0x1fd000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TocEntry {
    pub name: String,
    pub boot_offset: u32,
    pub memory_offset: u32,
    pub size: u32,
    pub crc: u32,
    pub index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageCrcPolicy {
    Required,
    NotRequired,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagePlan {
    pub name: String,
    pub index: u32,
    pub size: u32,
    pub crc: u32,
    pub crc_policy: StageCrcPolicy,
    pub start: u32,
    pub start_ack: u32,
    pub done: u32,
    pub done_ack: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SitBinPacket {
    pub command: u16,
    pub length: u16,
    pub total: u32,
    pub offset: u32,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootStep {
    LoadBoot { size: u32 },
    StartBootloader,
    Ready { request: u32, ack: u32 },
    TocStart { request: u32, ack: u32 },
    TocBin { total: u32, ack: u32 },
    TocDone { request: u32, ack: u32 },
    Stage(StagePlan),
    Fin { request: u32, ack: u32 },
    CompleteNormalBoot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadSource {
    FirmwareImage,
    NvNormalCopy,
    NvProtectedCopy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutorAction {
    LoadBootImage {
        size: u32,
    },
    StartBootloader,
    WriteWordExpectAck {
        request: u32,
        ack: u32,
    },
    SendTocBin {
        total: u32,
        ack: u32,
    },
    TransferStage {
        name: String,
        index: u32,
        size: u32,
        crc: u32,
        crc_policy: StageCrcPolicy,
        payload: PayloadSource,
        start: u32,
        start_ack: u32,
        bin_ack: u32,
        crc_request: Option<u32>,
        crc_ack: Option<u32>,
        done: u32,
        done_ack: u32,
    },
    CompleteNormalBoot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootPlan {
    pub steps: Vec<BootStep>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootFailureClass {
    Timeout,
    AckMismatch,
    BadConfig,
    Io,
    UnexpectedState,
    Layout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootFailureDisposition {
    StopBeforeHardware,
    FreshBootRequired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootFailure {
    pub step_index: usize,
    pub class: BootFailureClass,
    pub disposition: BootFailureDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AckMismatch {
    pub expected: u32,
    pub got: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootStepOutcome {
    Success,
    Timeout,
    AckMismatch(AckMismatch),
    BadConfig,
    Io,
    UnexpectedState,
    Layout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootProgress {
    plan: BootPlan,
    next_step: usize,
    failure: Option<BootFailure>,
}

impl BootProgress {
    pub fn new(plan: BootPlan) -> Self {
        Self {
            plan,
            next_step: 0,
            failure: None,
        }
    }

    pub fn current_step(&self) -> Option<&BootStep> {
        if self.failure.is_some() {
            return None;
        }
        self.plan.steps.get(self.next_step)
    }

    pub fn next_step_index(&self) -> usize {
        self.next_step
    }

    pub fn failure(&self) -> Option<&BootFailure> {
        self.failure.as_ref()
    }

    pub fn is_complete(&self) -> bool {
        self.failure.is_none() && self.next_step == self.plan.steps.len()
    }

    pub fn record_success(&mut self) -> Result<()> {
        if self.failure.is_some() {
            return Err(anyhow!("boot plan is failed; refusing to advance"));
        }
        if self.next_step >= self.plan.steps.len() {
            return Err(anyhow!("boot plan is already complete"));
        }
        self.next_step += 1;
        Ok(())
    }

    pub fn record_failure(&mut self, class: BootFailureClass) -> Result<BootFailure> {
        if let Some(failure) = &self.failure {
            return Err(anyhow!(
                "boot plan already failed at step {} ({:?})",
                failure.step_index,
                failure.class
            ));
        }
        if self.next_step >= self.plan.steps.len() {
            return Err(anyhow!("boot plan is already complete"));
        }
        let failure = BootFailure {
            step_index: self.next_step,
            class,
            disposition: failure_disposition(self.next_step),
        };
        self.failure = Some(failure.clone());
        Ok(failure)
    }

    pub fn record_outcome(&mut self, outcome: BootStepOutcome) -> Result<Option<BootFailure>> {
        match outcome {
            BootStepOutcome::Success => {
                self.record_success()?;
                Ok(None)
            }
            BootStepOutcome::Timeout => self.record_failure(BootFailureClass::Timeout).map(Some),
            BootStepOutcome::AckMismatch(_) => {
                self.record_failure(BootFailureClass::AckMismatch).map(Some)
            }
            BootStepOutcome::BadConfig => {
                self.record_failure(BootFailureClass::BadConfig).map(Some)
            }
            BootStepOutcome::Io => self.record_failure(BootFailureClass::Io).map(Some),
            BootStepOutcome::UnexpectedState => self
                .record_failure(BootFailureClass::UnexpectedState)
                .map(Some),
            BootStepOutcome::Layout => self.record_failure(BootFailureClass::Layout).map(Some),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Ack4Reader {
    pending: Vec<u8>,
}

impl Ack4Reader {
    pub fn push(&mut self, bytes: &[u8]) -> Vec<u32> {
        self.pending.extend_from_slice(bytes);
        let mut out = Vec::new();
        while self.pending.len() >= 4 {
            let word = le_u32(&self.pending[..4]);
            self.pending.drain(..4);
            out.push(word);
        }
        out
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }
}

pub fn parse_toc(image: &[u8]) -> Result<Vec<TocEntry>> {
    if image.len() < TOC_ENTRY_SIZE {
        return Err(anyhow!("image too small for TOC"));
    }
    let first = parse_toc_entry(&image[..TOC_ENTRY_SIZE])?;
    let count = first.index as usize;
    if count < 2 || count > 16 {
        return Err(anyhow!("invalid TOC count {}", first.index));
    }
    let toc_len = count
        .checked_mul(TOC_ENTRY_SIZE)
        .ok_or_else(|| anyhow!("TOC length overflow"))?;
    if image.len() < toc_len {
        return Err(anyhow!("image shorter than declared TOC"));
    }
    let mut entries = Vec::with_capacity(count);
    for chunk in image[..toc_len].chunks_exact(TOC_ENTRY_SIZE) {
        entries.push(parse_toc_entry(chunk)?);
    }
    Ok(entries)
}

pub fn find_stage<'a>(toc: &'a [TocEntry], name: &str) -> Option<&'a TocEntry> {
    toc.iter().find(|entry| entry.name == name)
}

pub fn validate_reviewed_panther_layout(toc: &[TocEntry], image_len: usize) -> Result<()> {
    require_stage(toc, image_len, "BOOT", 1, Some(0x16800))?;
    require_stage(toc, image_len, "MAIN", 2, None)?;
    require_stage(toc, image_len, "VSS", 3, None)?;
    require_stage(toc, image_len, "APM", 4, None)?;
    require_stage(toc, image_len, "NV_NORM", 5, Some(0x80000))?;
    require_stage(toc, image_len, "NV_PROT", 6, Some(0x80000))?;
    Ok(())
}

pub fn crc_policy(name: &str, index: u32) -> StageCrcPolicy {
    match (name, index) {
        ("MAIN", 2) => StageCrcPolicy::Required,
        ("VSS", 3) | ("APM", 4) | ("NV_NORM", 5) | ("NV_PROT", 6) => StageCrcPolicy::NotRequired,
        _ => StageCrcPolicy::Unsupported,
    }
}

pub fn stage_plan(entry: &TocEntry) -> Result<StagePlan> {
    let crc_policy = crc_policy(&entry.name, entry.index);
    if crc_policy == StageCrcPolicy::Unsupported {
        return Err(anyhow!(
            "unsupported stage {} index {}",
            entry.name,
            entry.index
        ));
    }
    Ok(StagePlan {
        name: entry.name.clone(),
        index: entry.index,
        size: entry.size,
        crc: entry.crc,
        crc_policy,
        start: sit_cmd(SIT_START, entry.index),
        start_ack: sit_cmd(SIT_START_ACK, entry.index),
        done: sit_cmd(SIT_DONE, entry.index),
        done_ack: sit_cmd(SIT_DONE_ACK, entry.index),
    })
}

pub fn reviewed_boot_plan(toc: &[TocEntry], image_len: usize) -> Result<BootPlan> {
    validate_reviewed_panther_layout(toc, image_len)?;
    let boot = find_stage(toc, "BOOT").ok_or_else(|| anyhow!("missing BOOT"))?;
    let mut steps = vec![
        BootStep::LoadBoot { size: boot.size },
        BootStep::StartBootloader,
        BootStep::Ready {
            request: SIT_READY,
            ack: SIT_READY_ACK,
        },
        BootStep::TocStart {
            request: sit_cmd(SIT_START, 1),
            ack: sit_cmd(SIT_START_ACK, 1),
        },
        BootStep::TocBin {
            total: 0x410,
            ack: sit_cmd(SIT_BIN_ACK, 1),
        },
        BootStep::TocDone {
            request: sit_cmd(SIT_DONE, 1),
            ack: sit_cmd(SIT_DONE_ACK, 1),
        },
    ];
    for name in ["MAIN", "VSS", "APM", "NV_NORM", "NV_PROT"] {
        let entry = find_stage(toc, name).ok_or_else(|| anyhow!("missing stage {name}"))?;
        steps.push(BootStep::Stage(stage_plan(entry)?));
    }
    steps.push(BootStep::Fin {
        request: SIT_FIN,
        ack: SIT_FIN_ACK,
    });
    steps.push(BootStep::CompleteNormalBoot);
    Ok(BootPlan { steps })
}

pub fn executor_actions(step: &BootStep) -> Result<Vec<ExecutorAction>> {
    match step {
        BootStep::LoadBoot { size } => Ok(vec![ExecutorAction::LoadBootImage { size: *size }]),
        BootStep::StartBootloader => Ok(vec![ExecutorAction::StartBootloader]),
        BootStep::Ready { request, ack }
        | BootStep::TocStart { request, ack }
        | BootStep::TocDone { request, ack }
        | BootStep::Fin { request, ack } => Ok(vec![ExecutorAction::WriteWordExpectAck {
            request: *request,
            ack: *ack,
        }]),
        BootStep::TocBin { total, ack } => Ok(vec![ExecutorAction::SendTocBin {
            total: *total,
            ack: *ack,
        }]),
        BootStep::Stage(stage) => Ok(vec![stage_transfer_action(stage)?]),
        BootStep::CompleteNormalBoot => Ok(vec![ExecutorAction::CompleteNormalBoot]),
    }
}

pub fn plan_executor_actions(plan: &BootPlan) -> Result<Vec<ExecutorAction>> {
    let mut actions = Vec::new();
    for step in &plan.steps {
        actions.extend(executor_actions(step)?);
    }
    validate_executor_actions(&actions)?;
    Ok(actions)
}

pub fn validate_executor_actions(actions: &[ExecutorAction]) -> Result<()> {
    if !matches!(actions.first(), Some(ExecutorAction::LoadBootImage { .. })) {
        return Err(anyhow!("boot executor plan must begin with BOOT load"));
    }
    if !matches!(actions.get(1), Some(ExecutorAction::StartBootloader)) {
        return Err(anyhow!("boot executor plan must start bootloader second"));
    }
    if !matches!(actions.last(), Some(ExecutorAction::CompleteNormalBoot)) {
        return Err(anyhow!(
            "boot executor plan must end with normal boot completion"
        ));
    }

    let mut saw_main_crc = false;
    for action in actions {
        if let ExecutorAction::TransferStage {
            name,
            index,
            crc_policy: action_crc_policy,
            payload,
            crc_request,
            crc_ack,
            ..
        } = action
        {
            let expected_payload = payload_source_for_stage(name, *index)?;
            if *payload != expected_payload {
                return Err(anyhow!(
                    "stage {name} index {index} has wrong payload source"
                ));
            }
            let expected_policy = crc_policy(name, *index);
            if expected_policy == StageCrcPolicy::Unsupported {
                return Err(anyhow!(
                    "stage {name} index {index} has unsupported CRC policy"
                ));
            }
            if *action_crc_policy != expected_policy {
                return Err(anyhow!("stage {name} index {index} has wrong CRC policy"));
            }
            match action_crc_policy {
                StageCrcPolicy::Required => {
                    if *crc_request != Some(sit_cmd(SIT_CRC, *index))
                        || *crc_ack != Some(sit_cmd(SIT_CRC_ACK, *index))
                    {
                        return Err(anyhow!("stage {name} index {index} has wrong CRC action"));
                    }
                    saw_main_crc = name == "MAIN" && *index == 2;
                }
                StageCrcPolicy::NotRequired => {
                    if crc_request.is_some() || crc_ack.is_some() {
                        return Err(anyhow!("stage {name} index {index} must not request CRC"));
                    }
                }
                StageCrcPolicy::Unsupported => {
                    return Err(anyhow!("unsupported CRC policy for {name}"));
                }
            }
        }
    }
    if !saw_main_crc {
        return Err(anyhow!("boot executor plan is missing MAIN CRC action"));
    }
    Ok(())
}

pub fn expect_ack(expected: u32, got: u32) -> std::result::Result<(), AckMismatch> {
    if expected == got {
        Ok(())
    } else {
        Err(AckMismatch { expected, got })
    }
}

pub fn ack_step_outcome(expected: u32, got: u32) -> BootStepOutcome {
    match expect_ack(expected, got) {
        Ok(()) => BootStepOutcome::Success,
        Err(mismatch) => BootStepOutcome::AckMismatch(mismatch),
    }
}

pub fn failure_disposition(step_index: usize) -> BootFailureDisposition {
    if step_index <= 1 {
        BootFailureDisposition::StopBeforeHardware
    } else {
        BootFailureDisposition::FreshBootRequired
    }
}

fn stage_transfer_action(stage: &StagePlan) -> Result<ExecutorAction> {
    let payload = payload_source_for_stage(&stage.name, stage.index)?;
    let (crc_request, crc_ack) = match stage.crc_policy {
        StageCrcPolicy::Required => (
            Some(sit_cmd(SIT_CRC, stage.index)),
            Some(sit_cmd(SIT_CRC_ACK, stage.index)),
        ),
        StageCrcPolicy::NotRequired => (None, None),
        StageCrcPolicy::Unsupported => {
            return Err(anyhow!("unsupported CRC policy for {}", stage.name));
        }
    };
    Ok(ExecutorAction::TransferStage {
        name: stage.name.clone(),
        index: stage.index,
        size: stage.size,
        crc: stage.crc,
        crc_policy: stage.crc_policy,
        payload,
        start: stage.start,
        start_ack: stage.start_ack,
        bin_ack: sit_cmd(SIT_BIN_ACK, stage.index),
        crc_request,
        crc_ack,
        done: stage.done,
        done_ack: stage.done_ack,
    })
}

fn payload_source_for_stage(name: &str, index: u32) -> Result<PayloadSource> {
    match (name, index) {
        ("MAIN", 2) | ("VSS", 3) | ("APM", 4) => Ok(PayloadSource::FirmwareImage),
        ("NV_NORM", 5) => Ok(PayloadSource::NvNormalCopy),
        ("NV_PROT", 6) => Ok(PayloadSource::NvProtectedCopy),
        _ => Err(anyhow!("unsupported stage {name} index {index}")),
    }
}

pub fn sit_cmd(base: u32, index: u32) -> u32 {
    base | ((index << 4) & 0xfff0)
}

pub fn build_bin_packet(
    index: u32,
    total: u32,
    offset: u32,
    payload: &[u8],
) -> Result<SitBinPacket> {
    if payload.len() > u16::MAX as usize - 8 {
        return Err(anyhow!("payload too large for SIT BIN packet"));
    }
    let command = sit_cmd(SIT_BIN, index) as u16;
    let length = (payload.len() + 8) as u16;
    let mut bytes = Vec::with_capacity(SIT_HEADER_SIZE as usize + payload.len());
    bytes.extend_from_slice(&command.to_le_bytes());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(&total.to_le_bytes());
    bytes.extend_from_slice(&offset.to_le_bytes());
    bytes.extend_from_slice(payload);
    Ok(SitBinPacket {
        command,
        length,
        total,
        offset,
        bytes,
    })
}

pub fn chunk_fit_ring(head: u32, remain: u32) -> u32 {
    let chunk = remain.min(SIT_CHUNK);
    let wire = EXYNOS_HEADER_SIZE + SIT_HEADER_SIZE + chunk;
    let room = LEGACY_RAW_TXQ_SIZE.saturating_sub(head);
    if wire <= room {
        return chunk;
    }
    if room <= EXYNOS_HEADER_SIZE + SIT_HEADER_SIZE {
        return chunk;
    }
    room - EXYNOS_HEADER_SIZE - SIT_HEADER_SIZE
}

fn parse_toc_entry(bytes: &[u8]) -> Result<TocEntry> {
    if bytes.len() != TOC_ENTRY_SIZE {
        return Err(anyhow!("bad TOC entry size"));
    }
    let name_end = bytes[..12].iter().position(|b| *b == 0).unwrap_or(12);
    let name = std::str::from_utf8(&bytes[..name_end])
        .map_err(|_| anyhow!("TOC name is not UTF-8"))?
        .to_string();
    Ok(TocEntry {
        name,
        boot_offset: le_u32(&bytes[12..16]),
        memory_offset: le_u32(&bytes[16..20]),
        size: le_u32(&bytes[20..24]),
        crc: le_u32(&bytes[24..28]),
        index: le_u32(&bytes[28..32]),
    })
}

fn require_stage(
    toc: &[TocEntry],
    image_len: usize,
    name: &str,
    index: u32,
    size: Option<u32>,
) -> Result<()> {
    let entry = find_stage(toc, name).ok_or_else(|| anyhow!("missing stage {name}"))?;
    if entry.index != index {
        return Err(anyhow!(
            "stage {name} has index {}, expected {index}",
            entry.index
        ));
    }
    if let Some(size) = size {
        if entry.size != size {
            return Err(anyhow!(
                "stage {name} has size 0x{:x}, expected 0x{size:x}",
                entry.size
            ));
        }
    }
    if entry.size == 0 {
        return Err(anyhow!("stage {name} has zero size"));
    }
    let end = entry
        .boot_offset
        .checked_add(entry.size)
        .ok_or_else(|| anyhow!("stage {name} bounds overflow"))?;
    if end as usize > image_len {
        return Err(anyhow!("stage {name} extends past image"));
    }
    Ok(())
}

fn le_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes.try_into().expect("slice has four bytes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sit_commands_match_factory_stage_bits() {
        assert_eq!(sit_cmd(SIT_READY, 0), 0xa00b);
        assert_eq!(sit_cmd(SIT_START, 1), 0xa110);
        assert_eq!(sit_cmd(SIT_BIN, 2), 0xa12b);
        assert_eq!(sit_cmd(SIT_CRC, 2), 0xa321);
        assert_eq!(sit_cmd(SIT_DONE, 6), 0xa16d);
    }

    #[test]
    fn bin_packet_is_factory_header_without_exynos_wrap() {
        let packet = build_bin_packet(2, 0x5917acc, 0x7e8, &[1, 2, 3]).unwrap();
        assert_eq!(packet.command, 0xa12b);
        assert_eq!(packet.length, 11);
        assert_eq!(
            &packet.bytes,
            &[0x2b, 0xa1, 0x0b, 0x00, 0xcc, 0x7a, 0x91, 0x05, 0xe8, 0x07, 0, 0, 1, 2, 3]
        );
    }

    #[test]
    fn ack4_reader_preserves_split_and_coalesced_words() {
        let mut reader = Ack4Reader::default();
        assert!(reader.push(&[0x2b, 0xc1]).is_empty());
        assert_eq!(reader.pending_len(), 2);
        assert_eq!(reader.push(&[0, 0]), vec![0xc12b]);
        assert_eq!(
            reader.push(&[0x20, 0xc1, 0, 0, 0x2d, 0xc1, 0, 0]),
            vec![0xc120, 0xc12d]
        );
    }

    #[test]
    fn reviewed_crc_policy_is_name_and_index_bound() {
        assert_eq!(crc_policy("MAIN", 2), StageCrcPolicy::Required);
        assert_eq!(crc_policy("VSS", 3), StageCrcPolicy::NotRequired);
        assert_eq!(crc_policy("APM", 4), StageCrcPolicy::NotRequired);
        assert_eq!(crc_policy("NV_NORM", 5), StageCrcPolicy::NotRequired);
        assert_eq!(crc_policy("MAIN", 3), StageCrcPolicy::Unsupported);
    }

    #[test]
    fn parse_and_validate_reviewed_toc_layout() {
        let image = reviewed_test_image();

        let toc = parse_toc(&image).unwrap();
        assert_eq!(toc.len(), 7);
        assert_eq!(find_stage(&toc, "MAIN").unwrap().crc, 0xb0f14905);
        validate_reviewed_panther_layout(&toc, image.len()).unwrap();
        assert_eq!(
            stage_plan(find_stage(&toc, "MAIN").unwrap()).unwrap().start,
            0xa120
        );
    }

    #[test]
    fn reviewed_boot_plan_has_factory_order_and_no_extra_ready() {
        let image = reviewed_test_image();
        let toc = parse_toc(&image).unwrap();
        let plan = reviewed_boot_plan(&toc, image.len()).unwrap();
        assert_eq!(plan.steps.len(), 13);
        assert_eq!(plan.steps[0], BootStep::LoadBoot { size: 0x16800 });
        assert_eq!(plan.steps[1], BootStep::StartBootloader);
        assert_eq!(
            plan.steps[2],
            BootStep::Ready {
                request: SIT_READY,
                ack: SIT_READY_ACK
            }
        );
        assert_eq!(
            plan.steps[3],
            BootStep::TocStart {
                request: 0xa110,
                ack: 0xc110
            }
        );
        assert_eq!(
            plan.steps[4],
            BootStep::TocBin {
                total: 0x410,
                ack: 0xc11b
            }
        );
        assert_eq!(
            plan.steps[5],
            BootStep::TocDone {
                request: 0xa11d,
                ack: 0xc11d
            }
        );
        assert!(matches!(
            &plan.steps[6],
            BootStep::Stage(stage)
                if stage.name == "MAIN" && stage.crc_policy == StageCrcPolicy::Required
        ));
        assert!(matches!(
            &plan.steps[10],
            BootStep::Stage(stage)
                if stage.name == "NV_PROT" && stage.crc_policy == StageCrcPolicy::NotRequired
        ));
        assert_eq!(
            plan.steps[11],
            BootStep::Fin {
                request: SIT_FIN,
                ack: SIT_FIN_ACK
            }
        );
        assert_eq!(plan.steps[12], BootStep::CompleteNormalBoot);
        assert_eq!(
            plan.steps
                .iter()
                .filter(|step| matches!(step, BootStep::Ready { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn executor_actions_bind_payload_sources_and_crc_policy() {
        let image = reviewed_test_image();
        let toc = parse_toc(&image).unwrap();
        let plan = reviewed_boot_plan(&toc, image.len()).unwrap();
        let actions = plan_executor_actions(&plan).unwrap();
        assert_eq!(actions.len(), 13);
        assert_eq!(actions[0], ExecutorAction::LoadBootImage { size: 0x16800 });
        assert_eq!(
            actions[2],
            ExecutorAction::WriteWordExpectAck {
                request: SIT_READY,
                ack: SIT_READY_ACK
            }
        );
        assert_eq!(
            actions[4],
            ExecutorAction::SendTocBin {
                total: 0x410,
                ack: 0xc11b
            }
        );
        assert!(matches!(
            &actions[6],
            ExecutorAction::TransferStage {
                name,
                index: 2,
                payload: PayloadSource::FirmwareImage,
                crc_policy: StageCrcPolicy::Required,
                bin_ack: 0xc12b,
                crc_request: Some(0xa321),
                crc_ack: Some(0xc320),
                done: 0xa12d,
                done_ack: 0xc12d,
                ..
            } if name == "MAIN"
        ));
        assert!(matches!(
            &actions[9],
            ExecutorAction::TransferStage {
                name,
                index: 5,
                payload: PayloadSource::NvNormalCopy,
                crc_policy: StageCrcPolicy::NotRequired,
                crc_request: None,
                crc_ack: None,
                ..
            } if name == "NV_NORM"
        ));
        assert_eq!(
            actions[11],
            ExecutorAction::WriteWordExpectAck {
                request: SIT_FIN,
                ack: SIT_FIN_ACK
            }
        );
        assert_eq!(actions[12], ExecutorAction::CompleteNormalBoot);
    }

    #[test]
    fn executor_action_validation_fails_closed_on_bad_stage_contract() {
        let valid_main = ExecutorAction::TransferStage {
            name: "MAIN".into(),
            index: 2,
            size: 0x100000,
            crc: 0,
            crc_policy: StageCrcPolicy::Required,
            payload: PayloadSource::FirmwareImage,
            start: 0xa120,
            start_ack: 0xc120,
            bin_ack: 0xc12b,
            crc_request: Some(0xa321),
            crc_ack: Some(0xc320),
            done: 0xa12d,
            done_ack: 0xc12d,
        };

        let mut actions = vec![
            ExecutorAction::LoadBootImage { size: 0x16800 },
            ExecutorAction::StartBootloader,
            valid_main.clone(),
            ExecutorAction::CompleteNormalBoot,
        ];
        assert!(validate_executor_actions(&actions).is_ok());

        if let ExecutorAction::TransferStage { crc_policy, .. } = &mut actions[2] {
            *crc_policy = StageCrcPolicy::Unsupported;
        }
        assert!(validate_executor_actions(&actions).is_err());

        actions[2] = valid_main;
        if let ExecutorAction::TransferStage { payload, .. } = &mut actions[2] {
            *payload = PayloadSource::NvNormalCopy;
        }
        assert!(validate_executor_actions(&actions).is_err());
    }

    #[test]
    fn ack_expectation_reports_expected_and_got_words() {
        assert_eq!(expect_ack(0xc12b, 0xc12b), Ok(()));
        assert_eq!(
            expect_ack(0xc12b, 0xc120),
            Err(AckMismatch {
                expected: 0xc12b,
                got: 0xc120
            })
        );
        assert_eq!(ack_step_outcome(0xc12b, 0xc12b), BootStepOutcome::Success);
        assert_eq!(
            ack_step_outcome(0xc12b, 0xc120),
            BootStepOutcome::AckMismatch(AckMismatch {
                expected: 0xc12b,
                got: 0xc120
            })
        );
    }

    #[test]
    fn boot_progress_stops_after_failure() {
        let image = reviewed_test_image();
        let toc = parse_toc(&image).unwrap();
        let plan = reviewed_boot_plan(&toc, image.len()).unwrap();
        let mut progress = BootProgress::new(plan);
        assert_eq!(progress.next_step_index(), 0);
        assert!(matches!(
            progress.current_step(),
            Some(BootStep::LoadBoot { size: 0x16800 })
        ));
        progress.record_success().unwrap();
        progress.record_success().unwrap();
        assert!(matches!(
            progress.current_step(),
            Some(BootStep::Ready {
                request: SIT_READY,
                ..
            })
        ));
        let failure = progress
            .record_failure(BootFailureClass::Timeout)
            .expect("first failure is recorded");
        assert_eq!(failure.step_index, 2);
        assert_eq!(
            failure.disposition,
            BootFailureDisposition::FreshBootRequired
        );
        assert!(progress.current_step().is_none());
        assert!(progress.record_success().is_err());
        assert!(progress
            .record_failure(BootFailureClass::AckMismatch)
            .is_err());
    }

    #[test]
    fn boot_progress_records_executor_outcomes() {
        let image = reviewed_test_image();
        let toc = parse_toc(&image).unwrap();
        let plan = reviewed_boot_plan(&toc, image.len()).unwrap();
        let mut progress = BootProgress::new(plan);

        assert_eq!(
            progress.record_outcome(BootStepOutcome::Success).unwrap(),
            None
        );
        assert_eq!(progress.next_step_index(), 1);

        let failure = progress
            .record_outcome(BootStepOutcome::AckMismatch(AckMismatch {
                expected: SIT_READY_ACK,
                got: SIT_FIN_ACK,
            }))
            .unwrap()
            .expect("ack mismatch records a failure");
        assert_eq!(failure.step_index, 1);
        assert_eq!(failure.class, BootFailureClass::AckMismatch);
        assert_eq!(
            failure.disposition,
            BootFailureDisposition::StopBeforeHardware
        );
        assert!(progress.record_outcome(BootStepOutcome::Success).is_err());
    }

    #[test]
    fn boot_progress_maps_late_executor_outcome_to_fresh_boot_required() {
        let image = reviewed_test_image();
        let toc = parse_toc(&image).unwrap();
        let plan = reviewed_boot_plan(&toc, image.len()).unwrap();
        let mut progress = BootProgress::new(plan);

        progress.record_outcome(BootStepOutcome::Success).unwrap();
        progress.record_outcome(BootStepOutcome::Success).unwrap();
        let failure = progress
            .record_outcome(BootStepOutcome::Timeout)
            .unwrap()
            .expect("timeout records a failure");
        assert_eq!(failure.step_index, 2);
        assert_eq!(failure.class, BootFailureClass::Timeout);
        assert_eq!(
            failure.disposition,
            BootFailureDisposition::FreshBootRequired
        );
    }

    #[test]
    fn boot_progress_completes_once_and_rejects_extra_success() {
        let image = reviewed_test_image();
        let toc = parse_toc(&image).unwrap();
        let plan = reviewed_boot_plan(&toc, image.len()).unwrap();
        let mut progress = BootProgress::new(plan.clone());
        for _ in 0..plan.steps.len() {
            progress.record_success().unwrap();
        }
        assert!(progress.is_complete());
        assert!(progress.current_step().is_none());
        assert!(progress.record_success().is_err());
        assert!(progress.record_failure(BootFailureClass::Io).is_err());
    }

    #[test]
    fn failure_disposition_is_conservative_after_bootloader_start() {
        assert_eq!(
            failure_disposition(0),
            BootFailureDisposition::StopBeforeHardware
        );
        assert_eq!(
            failure_disposition(1),
            BootFailureDisposition::StopBeforeHardware
        );
        assert_eq!(
            failure_disposition(2),
            BootFailureDisposition::FreshBootRequired
        );
    }

    #[test]
    fn reviewed_layout_fails_closed_on_bad_nv_size() {
        let mut image = vec![0u8; 0x300000];
        write_entry(&mut image, 0, "TOC", 0, 0, 0x410, 0, 7);
        write_entry(&mut image, 1, "BOOT", 0x1000, 0, 0x16800, 0, 1);
        write_entry(&mut image, 2, "MAIN", 0x20000, 0, 0x100000, 0, 2);
        write_entry(&mut image, 3, "VSS", 0x120000, 0, 0x1000, 0, 3);
        write_entry(&mut image, 4, "APM", 0x121000, 0, 0xb498, 0, 4);
        write_entry(&mut image, 5, "NV_NORM", 0, 0, 0x80001, 0, 5);
        write_entry(&mut image, 6, "NV_PROT", 0, 0, 0x80000, 0, 6);
        let toc = parse_toc(&image).unwrap();
        assert!(validate_reviewed_panther_layout(&toc, image.len()).is_err());
    }

    #[test]
    fn chunk_fit_preserves_ring_end() {
        assert_eq!(chunk_fit_ring(0x10, 0x2000), SIT_CHUNK);
        let head = LEGACY_RAW_TXQ_SIZE - (EXYNOS_HEADER_SIZE + SIT_HEADER_SIZE + 0x100);
        assert_eq!(chunk_fit_ring(head, 0x2000), 0x100);
    }

    fn reviewed_test_image() -> Vec<u8> {
        let mut image = vec![0u8; 0x300000];
        write_entry(&mut image, 0, "TOC", 0, 0, 0x410, 0, 7);
        write_entry(&mut image, 1, "BOOT", 0x1000, 0, 0x16800, 0, 1);
        write_entry(&mut image, 2, "MAIN", 0x20000, 0, 0x100000, 0xb0f14905, 2);
        write_entry(&mut image, 3, "VSS", 0x120000, 0, 0x1000, 1, 3);
        write_entry(&mut image, 4, "APM", 0x121000, 0, 0xb498, 2, 4);
        write_entry(&mut image, 5, "NV_NORM", 0, 0, 0x80000, 0, 5);
        write_entry(&mut image, 6, "NV_PROT", 0, 0, 0x80000, 0, 6);
        image
    }

    fn write_entry(
        image: &mut [u8],
        slot: usize,
        name: &str,
        boot_offset: u32,
        memory_offset: u32,
        size: u32,
        crc: u32,
        index: u32,
    ) {
        let start = slot * TOC_ENTRY_SIZE;
        let entry = &mut image[start..start + TOC_ENTRY_SIZE];
        let name_bytes = name.as_bytes();
        entry[..name_bytes.len()].copy_from_slice(name_bytes);
        entry[12..16].copy_from_slice(&boot_offset.to_le_bytes());
        entry[16..20].copy_from_slice(&memory_offset.to_le_bytes());
        entry[20..24].copy_from_slice(&size.to_le_bytes());
        entry[24..28].copy_from_slice(&crc.to_le_bytes());
        entry[28..32].copy_from_slice(&index.to_le_bytes());
    }
}
