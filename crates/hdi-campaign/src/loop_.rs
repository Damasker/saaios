use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;

use hdi_contract::{control_from_redacted, discovery_gain, Claim, ProbeName, ProbeRequest};
use hdi_exec::Executor;
use serde::Serialize;
use serde_json::{json, Value};
use thiserror::Error;

use crate::investigator::Investigator;
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
            if storable(&claim)
                && investigator_claims
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
        status: RunStatus::Completed,
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
