//! Fixture verifier. The sealed key stays beside the snapshot, never in `redacted/`.

use std::fs;
use std::path::Path;

use hdi_contract::{control_from_redacted, Claim, ClaimKind};
use thiserror::Error;

const IGD_SUBJECT: &str = "pci:0000:00:02.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandVerdict {
    /// Outcome A: expected product, integrated GPU not enumerated.
    FirmwareUnenumerated,
    /// Outcome B: integrated GPU is on the bus.
    IgpuPresent,
    /// Outcome C: DMI or device-tree model does not match the stand key.
    IdentityInsufficient,
    /// Device-tree model matches, and the key's display and render nodes
    /// are bound to the named drivers. PCI VGA class `[0300]` is absent.
    PlatformGraphicsMatched,
    /// Device-tree model matches, but the graphics lines do not.
    GraphicsMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordStatus {
    Confirmed,
    Refuted,
}

#[derive(Debug, Error)]
pub enum VerifyError {
    #[error("fixture key is missing or incomplete")]
    InvalidKey,
    #[error("claim has no evidence")]
    NoEvidence,
    #[error("knowledge record contains an executable load")]
    Executable,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Stored {
    machine_id: String,
    claim: Claim,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MachineKnowledge {
    confirmed: Vec<Stored>,
    refuted: Vec<Stored>,
}

impl MachineKnowledge {
    pub fn confirmed_for(&self, machine_id: &str) -> Vec<Claim> {
        self.confirmed
            .iter()
            .filter(|record| record.machine_id == machine_id)
            .map(|record| record.claim.clone())
            .collect()
    }

    pub fn all_text(&self) -> String {
        self.confirmed
            .iter()
            .chain(self.refuted.iter())
            .map(|record| claim_text(&record.claim))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Copy confirmed facts of `machine_id` into that machine's control list.
    pub fn merge_into_control(&self, machine_id: &str, control: &mut Vec<Claim>) {
        for claim in self.confirmed_for(machine_id) {
            if control.iter().all(|known| !known.same_fact(&claim)) {
                control.push(claim);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verification {
    pub verdict: StandVerdict,
    pub machine_id: String,
    expected_bindings: Vec<(String, String)>,
    knowledge: MachineKnowledge,
}

impl Verification {
    pub fn confirmed_for(&self, machine_id: &str) -> Vec<Claim> {
        self.knowledge.confirmed_for(machine_id)
    }

    pub fn knowledge_text(&self) -> String {
        self.knowledge.all_text()
    }

    pub fn merge_into_control(&self, machine_id: &str, control: &mut Vec<Claim>) {
        self.knowledge.merge_into_control(machine_id, control);
    }

    pub fn judge(&mut self, proposal: Claim) -> Result<RecordStatus, VerifyError> {
        reject_executable(&proposal)?;
        if proposal.evidence_ids.is_empty() || proposal.evidence_ids.iter().any(String::is_empty) {
            return Err(VerifyError::NoEvidence);
        }
        let status = self.status_of(&proposal);
        let stored = Stored {
            machine_id: self.machine_id.clone(),
            claim: proposal,
        };
        match status {
            RecordStatus::Confirmed => {
                if self
                    .knowledge
                    .confirmed
                    .iter()
                    .all(|record| !record.claim.same_fact(&stored.claim))
                {
                    self.knowledge.confirmed.push(stored);
                }
            }
            RecordStatus::Refuted => self.knowledge.refuted.push(stored),
        }
        Ok(status)
    }

    fn status_of(&self, proposal: &Claim) -> RecordStatus {
        let igd =
            proposal.subject == IGD_SUBJECT && proposal.value.eq_ignore_ascii_case("8086:0d26");
        match self.verdict {
            StandVerdict::FirmwareUnenumerated => match proposal.kind {
                ClaimKind::NegativeRecipe if igd => RecordStatus::Confirmed,
                ClaimKind::DriverGap if igd => RecordStatus::Refuted,
                _ => RecordStatus::Refuted,
            },
            StandVerdict::IgpuPresent => match proposal.kind {
                ClaimKind::NegativeRecipe if igd => RecordStatus::Refuted,
                ClaimKind::DriverBound | ClaimKind::DriverUnbound | ClaimKind::DriverGap
                    if proposal.evidence_ids.iter().any(|id| !id.is_empty()) =>
                {
                    RecordStatus::Confirmed
                }
                _ => RecordStatus::Refuted,
            },
            StandVerdict::IdentityInsufficient => match proposal.kind {
                ClaimKind::InsufficientEvidence => RecordStatus::Confirmed,
                _ => RecordStatus::Refuted,
            },
            StandVerdict::PlatformGraphicsMatched => {
                if proposal.kind == ClaimKind::DeviceAbsent
                    && proposal.subject == "pci:class:0300"
                    && proposal.value == "0300"
                {
                    RecordStatus::Confirmed
                } else if proposal.kind == ClaimKind::DriverBound
                    && self.expected_bindings.iter().any(|(subject, value)| {
                        subject == &proposal.subject && value == &proposal.value
                    })
                {
                    RecordStatus::Confirmed
                } else {
                    RecordStatus::Refuted
                }
            }
            StandVerdict::GraphicsMismatch => RecordStatus::Refuted,
        }
    }
}

enum FixtureKey {
    PciIgd {
        machine_id: String,
        expected_product: String,
        igd_id: String,
    },
    PlatformDrm {
        machine_id: String,
        expected_model: String,
        display_node: String,
        display_driver: String,
        render_node: String,
        render_driver: String,
    },
}

pub fn verify(redacted: &Path, key_path: &Path) -> Result<Verification, VerifyError> {
    let key = parse_key(&fs::read_to_string(key_path)?)?;
    match key {
        FixtureKey::PciIgd {
            machine_id,
            expected_product,
            igd_id,
        } => verify_pci_igd(redacted, machine_id, expected_product, igd_id),
        FixtureKey::PlatformDrm {
            machine_id,
            expected_model,
            display_node,
            display_driver,
            render_node,
            render_driver,
        } => verify_platform_drm(
            redacted,
            machine_id,
            expected_model,
            display_node,
            display_driver,
            render_node,
            render_driver,
        ),
    }
}

fn verify_pci_igd(
    redacted: &Path,
    machine_id: String,
    expected_product: String,
    igd_id: String,
) -> Result<Verification, VerifyError> {
    let dmi_path = redacted.join("dmi.txt");
    let product = if dmi_path.is_file() {
        product_name(&fs::read_to_string(&dmi_path)?)
    } else {
        None
    };
    let control = control_from_redacted(redacted)?;
    let igd_present = control.iter().any(|claim| {
        claim.kind == ClaimKind::DevicePresent && claim.value.eq_ignore_ascii_case(&igd_id)
    });
    let identity_matches = product.as_deref() == Some(expected_product.as_str());
    let verdict = if !identity_matches {
        StandVerdict::IdentityInsufficient
    } else if igd_present {
        StandVerdict::IgpuPresent
    } else {
        StandVerdict::FirmwareUnenumerated
    };
    let mut verification = Verification {
        verdict,
        machine_id,
        expected_bindings: Vec::new(),
        knowledge: MachineKnowledge::default(),
    };
    match verdict {
        StandVerdict::FirmwareUnenumerated => {
            verification.judge(Claim::new(
                ClaimKind::NegativeRecipe,
                IGD_SUBJECT,
                igd_id,
                vec!["pci.txt".into(), "dmi.txt".into()],
            ))?;
        }
        StandVerdict::IgpuPresent => {}
        StandVerdict::IdentityInsufficient => {
            let observed = product.unwrap_or_else(|| "unread".to_string());
            verification.judge(Claim::new(
                ClaimKind::InsufficientEvidence,
                "dmi:product_name",
                observed,
                vec!["dmi.txt".into()],
            ))?;
        }
        StandVerdict::PlatformGraphicsMatched | StandVerdict::GraphicsMismatch => {}
    }
    Ok(verification)
}

fn verify_platform_drm(
    redacted: &Path,
    machine_id: String,
    expected_model: String,
    display_node: String,
    display_driver: String,
    render_node: String,
    render_driver: String,
) -> Result<Verification, VerifyError> {
    let observed = read_trimmed(&redacted.join("devicetree-model.txt"));
    if observed.as_deref() != Some(expected_model.as_str()) {
        let mut verification = Verification {
            verdict: StandVerdict::IdentityInsufficient,
            machine_id,
            expected_bindings: Vec::new(),
            knowledge: MachineKnowledge::default(),
        };
        verification.judge(Claim::new(
            ClaimKind::InsufficientEvidence,
            "dt:model",
            observed.unwrap_or_else(|| "unread".to_string()),
            vec!["devicetree-model.txt".into()],
        ))?;
        return Ok(verification);
    }
    let drm = read_trimmed(&redacted.join("drm.txt")).unwrap_or_default();
    let modules = read_trimmed(&redacted.join("modules.txt")).unwrap_or_default();
    let pci = read_trimmed(&redacted.join("pci.txt")).unwrap_or_default();
    let matched = node_has_driver(&drm, &display_node, &display_driver)
        && module_loaded(&modules, &display_driver)
        && node_has_driver(&drm, &render_node, &render_driver)
        && module_loaded(&modules, &render_driver)
        && !pci.contains("[0300]");
    let display_subject = format!("drm:{display_node}");
    let render_subject = format!("drm:{render_node}");
    if !matched {
        return Ok(Verification {
            verdict: StandVerdict::GraphicsMismatch,
            machine_id,
            expected_bindings: vec![
                (display_subject, display_driver),
                (render_subject, render_driver),
            ],
            knowledge: MachineKnowledge::default(),
        });
    }
    let mut verification = Verification {
        verdict: StandVerdict::PlatformGraphicsMatched,
        machine_id,
        expected_bindings: vec![
            (display_subject.clone(), display_driver.clone()),
            (render_subject.clone(), render_driver.clone()),
        ],
        knowledge: MachineKnowledge::default(),
    };
    verification.judge(Claim::new(
        ClaimKind::DriverBound,
        display_subject,
        display_driver,
        vec!["drm.txt".into(), "modules.txt".into()],
    ))?;
    verification.judge(Claim::new(
        ClaimKind::DriverBound,
        render_subject,
        render_driver,
        vec!["drm.txt".into(), "modules.txt".into()],
    ))?;
    verification.judge(Claim::new(
        ClaimKind::DeviceAbsent,
        "pci:class:0300",
        "0300",
        vec!["pci.txt".into()],
    ))?;
    Ok(verification)
}

fn parse_key(text: &str) -> Result<FixtureKey, VerifyError> {
    let mut fields = std::collections::BTreeMap::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() || is_executable(value) {
            continue;
        }
        fields.insert(key.trim().to_string(), value.to_string());
    }
    let machine_id = fields
        .get("machine_id")
        .cloned()
        .ok_or(VerifyError::InvalidKey)?;
    if fields.contains_key("expected_model") {
        if fields.contains_key("expected_product") || fields.contains_key("igd_id") {
            return Err(VerifyError::InvalidKey);
        }
        return Ok(FixtureKey::PlatformDrm {
            machine_id,
            expected_model: required(&fields, "expected_model")?,
            display_node: required(&fields, "display_node")?,
            display_driver: required(&fields, "display_driver")?,
            render_node: required(&fields, "render_node")?,
            render_driver: required(&fields, "render_driver")?,
        });
    }
    Ok(FixtureKey::PciIgd {
        machine_id,
        expected_product: required(&fields, "expected_product")?,
        igd_id: required(&fields, "igd_id")?.to_ascii_lowercase(),
    })
}

fn required(
    fields: &std::collections::BTreeMap<String, String>,
    name: &str,
) -> Result<String, VerifyError> {
    fields.get(name).cloned().ok_or(VerifyError::InvalidKey)
}

fn read_trimmed(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    let text = text.trim_start_matches('\u{feff}').trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

fn node_has_driver(drm: &str, node: &str, driver: &str) -> bool {
    drm.lines().map(str::trim).any(|line| {
        line.split_whitespace().next() == Some(node)
            && line
                .split(|ch: char| ch.is_whitespace() || ch == '/' || ch == '\\')
                .any(|token| token.replace('-', "_") == driver)
    })
}

fn module_loaded(modules: &str, driver: &str) -> bool {
    modules
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .any(|name| name == driver)
}

fn product_name(dmi: &str) -> Option<String> {
    dmi.lines().find_map(|line| {
        line.strip_prefix("product_name:")
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn reject_executable(claim: &Claim) -> Result<(), VerifyError> {
    if is_executable(&claim_text(claim)) {
        Err(VerifyError::Executable)
    } else {
        Ok(())
    }
}

fn claim_text(claim: &Claim) -> String {
    format!(
        "{} {} {}",
        claim.subject,
        claim.value,
        claim.hypothesis.as_deref().unwrap_or("")
    )
}

fn is_executable(text: &str) -> bool {
    text.contains("insmod") || text.contains("modprobe") || text.contains(".ko")
}
