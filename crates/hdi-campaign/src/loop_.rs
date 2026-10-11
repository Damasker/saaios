use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;

use hdi_contract::{
    control_from_redacted, discovery_gain, Claim, ClaimKind, ProbeName, ProbeRequest,
};
use sha2::{Digest, Sha256};
use hdi_exec::Executor;
use serde::Serialize;
use serde_json::{json, Value};
use thiserror::Error;

use crate::intent_gate::INVESTIGATE_HARDWARE;
use crate::investigator::{Hunter, Investigator};
use crate::metrics::Metrics;

const MAX_PASSES: u32 = 5;

/// Redacted bundle shown to the investigator. No sealed-key field exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InvestigatorInput {
    pub campaign_id: String,
    pub pass: u32,
    pub files: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Completed,
    /// The fixture directory changed after the campaign opened it.
    /// Later probes are not executed.
    PackageChanged,
}

#[derive(Debug)]
pub struct RunReport {
    pub status: RunStatus,
    pub metrics: Metrics,
    pub inputs: Vec<String>,
    pub requests: Vec<String>,
    pub executed: u32,
    pub investigator_calls: u32,
    /// Discovery gain after each pass, in order.
    pub pass_gain: Vec<usize>,
    pub claims: Vec<Claim>,
    pub control_facts: usize,
}

#[derive(Debug, Error)]
pub enum CampaignError {
    #[error("initial bundle is missing {0}")]
    MissingFile(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// One campaign for `investigate_hardware`. Any other action id returns
/// `Ok(None)` and does not create a probe journal.
pub fn campaign_from_action(
    action_id: &str,
    fixture: &Path,
    campaign: &Path,
) -> Result<Option<RunReport>, CampaignError> {
    if action_id != INVESTIGATE_HARDWARE {
        return Ok(None);
    }
    let mut hunter = Hunter::new();
    Ok(Some(run_campaign(
        fixture,
        campaign,
        &["pci.txt"],
        &mut hunter,
    )?))
}

pub fn run_campaign(
    fixture: &Path,
    campaign: &Path,
    initial: &[&str],
    investigator: &mut dyn Investigator,
) -> Result<RunReport, CampaignError> {
    let bundle_dir = campaign.join("bundle");
    fs::create_dir_all(&bundle_dir)?;
    let mut files = BTreeMap::new();
    for name in initial {
        let text = fs::read_to_string(fixture.join(name))
            .map_err(|_| CampaignError::MissingFile((*name).to_string()))?;
        fs::write(bundle_dir.join(name), &text)?;
        files.insert((*name).to_string(), text);
    }

    let mut executor = Executor::open(fixture, campaign)?;
    let opened = package_digest(fixture)?;
    let mut status = RunStatus::Completed;
    let mut inputs = Vec::new();
    let mut requests = Vec::new();
    let mut rejected = HashSet::new();
    let mut executed = 0u32;
    let mut calls = 0u32;
    let mut investigator_claims: Vec<Claim> = Vec::new();
    let mut pass_gain = Vec::new();

    for pass in 1..=MAX_PASSES {
        let input = InvestigatorInput {
            campaign_id: "hdi".to_string(),
            pass,
            files: files.clone(),
        };
        let input_json = serde_json::to_string(&input).map_err(io_invalid)?;
        inputs.push(input_json.clone());
        let raw = investigator.propose(&input_json);
        calls += 1;
        requests.push(raw.clone());
        if !rejected.contains(&raw) {
            if let Some(stamped) = stamp_pass(&raw, pass) {
                if let Ok(request) = ProbeRequest::parse(&stamped) {
                    if package_digest(fixture)? != opened {
                        status = RunStatus::PackageChanged;
                        break;
                    }
                    match executor.execute(&stamped) {
                        Ok(evidence) => {
                            executed += 1;
                            absorb(
                                &mut files,
                                &bundle_dir,
                                request.requested_probe,
                                &evidence.bytes,
                            )?;
                        }
                        Err(hdi_exec::ExecError::Denied { .. }) => {
                            rejected.insert(raw);
                        }
                        Err(hdi_exec::ExecError::Interrupted) => {}
                        Err(hdi_exec::ExecError::Io(error)) => return Err(error.into()),
                    }
                } else {
                    rejected.insert(raw);
                }
            } else {
                rejected.insert(raw);
            }
        }
        let seen = InvestigatorInput {
            campaign_id: "hdi".to_string(),
            pass,
            files: files.clone(),
        };
        let seen_json = serde_json::to_string(&seen).map_err(io_invalid)?;
        for claim in investigator.observe(&seen_json) {
            let Some(claim) = admit_claim(claim, &files) else {
                continue;
            };
            if investigator_claims
                .iter()
                .all(|known| !known.same_fact(&claim))
            {
                investigator_claims.push(claim);
            }
        }
        let control = control_from_redacted(&bundle_dir)?;
        pass_gain.push(discovery_gain(&investigator_claims, &control));
    }

    let control = control_from_redacted(&bundle_dir)?;
    let metrics = Metrics::from_run(&investigator_claims, &control, executed, &[]);
    Ok(RunReport {
        status,
        metrics,
        inputs,
        requests,
        executed,
        investigator_calls: calls,
        pass_gain,
        claims: investigator_claims,
        control_facts: control.len(),
    })
}

/// `DeviceAbsent` is kept only when every cited file is already in the
/// bundle. An unread file becomes `InsufficientEvidence` with value `unread`.
fn admit_claim(mut claim: Claim, files: &BTreeMap<String, String>) -> Option<Claim> {
    if claim.kind == ClaimKind::DeviceAbsent
        && claim
            .evidence_ids
            .iter()
            .any(|id| !files.contains_key(id))
    {
        claim.kind = ClaimKind::InsufficientEvidence;
        claim.value = "unread".to_string();
    }
    if storable(&claim) {
        Some(claim)
    } else {
        None
    }
}

fn package_digest(root: &Path) -> Result<String, CampaignError> {
    let mut paths = Vec::new();
    collect_files(root, root, &mut paths)?;
    paths.sort();
    let mut hasher = Sha256::new();
    for relative in paths {
        hasher.update(relative.as_bytes());
        hasher.update([0]);
        let bytes = fs::read(root.join(&relative))?;
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(&bytes);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn collect_files(root: &Path, dir: &Path, paths: &mut Vec<String>) -> Result<(), CampaignError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, paths)?;
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|error| CampaignError::Io(std::io::Error::new(std::io::ErrorKind::Other, error)))?;
        paths.push(relative.to_string_lossy().replace('\\', "/"));
    }
    Ok(())
}

fn storable(claim: &Claim) -> bool {
    if claim.evidence_ids.is_empty() || claim.evidence_ids.iter().any(String::is_empty) {
        return false;
    }
    let text = format!(
        "{} {} {}",
        claim.subject,
        claim.value,
        claim.hypothesis.as_deref().unwrap_or("")
    );
    !text.contains("insmod") && !text.contains("modprobe") && !text.contains(".ko")
}

fn absorb(
    files: &mut BTreeMap<String, String>,
    bundle_dir: &Path,
    probe: ProbeName,
    bytes: &[u8],
) -> Result<(), CampaignError> {
    let Some(name) = bundle_name(probe) else {
        return Ok(());
    };
    if files.contains_key(name) {
        return Ok(());
    }
    let text = String::from_utf8_lossy(bytes).into_owned();
    fs::write(bundle_dir.join(name), &text)?;
    files.insert(name.to_string(), text);
    Ok(())
}

fn bundle_name(probe: ProbeName) -> Option<&'static str> {
    Some(match probe {
        ProbeName::PciId | ProbeName::PciSlot => "pci.txt",
        ProbeName::PciDrivers => "pci_drivers.txt",
        ProbeName::PciTree => "pci_tree.txt",
        ProbeName::UsbBrief => "usb.txt",
        ProbeName::UsbTree => "usb_tree.txt",
        ProbeName::Modules => "modules.txt",
        ProbeName::DrmClass => "drm.txt",
        ProbeName::PciSysfsList => "pci_sysfs.txt",
        ProbeName::DmiAllowlist => "dmi.txt",
        ProbeName::PlatformNodes => "platform.txt",
        ProbeName::PlatformDrivers => "platform_drivers.txt",
        ProbeName::KernelWarningsRedacted => "kernel_warnings.txt",
        ProbeName::SysfsRead => return None,
    })
}

fn stamp_pass(raw: &str, pass: u32) -> Option<String> {
    let mut value: Value = serde_json::from_str(raw).ok()?;
    let object = value.as_object_mut()?;
    object.insert("pass".to_string(), json!(pass));
    object.entry("campaign_id").or_insert_with(|| json!("hdi"));
    serde_json::to_string(&value).ok()
}

fn io_invalid(error: serde_json::Error) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, error)
}
