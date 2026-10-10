use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use hdi_contract::{ContractError, ProbeName, ProbeRequest};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

const DMI_KEYS: &[&str] = &[
    "product_name",
    "product_version",
    "board_name",
    "bios_version",
    "bios_date",
    "sys_vendor",
];

/// Where a read stops. `AfterAllow` returns after the probe is accepted and
/// does not store evidence or grant the next request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    None,
    AfterAllow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JournalVerdict {
    Allow,
    Deny,
    Interrupted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalEntry {
    pub pass: u32,
    pub probe: String,
    pub verdict: JournalVerdict,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    pub sha256: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum ExecError {
    #[error("probe denied: {reason}")]
    Denied { reason: String },
    #[error("executor interrupted after allow")]
    Interrupted,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub struct Executor {
    fixture: PathBuf,
    campaign: PathBuf,
}

impl Executor {
    pub fn open(
        fixture: impl Into<PathBuf>,
        campaign: impl Into<PathBuf>,
    ) -> std::io::Result<Self> {
        let fixture = fixture.into();
        let campaign = campaign.into();
        fs::create_dir_all(campaign.join("evidence"))?;
        let journal = campaign.join("journal.jsonl");
        if !journal.exists() {
            fs::write(&journal, "")?;
        }
        Ok(Self { fixture, campaign })
    }

    pub fn execute(&mut self, raw: &str) -> Result<Evidence, ExecError> {
        self.execute_fault(raw, Fault::None)
    }

    pub fn execute_fault(&mut self, raw: &str, fault: Fault) -> Result<Evidence, ExecError> {
        let request = match ProbeRequest::parse(raw) {
            Ok(request) => request,
            Err(error) => {
                let (pass, probe) = preview(raw);
                let reason = reason_code(&error);
                self.append_journal(JournalEntry {
                    pass,
                    probe,
                    verdict: JournalVerdict::Deny,
                    sha256: None,
                    reason: Some(reason.to_string()),
                })?;
                return Err(ExecError::Denied {
                    reason: reason.to_string(),
                });
            }
        };
        let bytes = match self.read_request(&request) {
            Ok(bytes) => bytes,
            Err(ExecError::Denied { reason }) => {
                self.append_journal(JournalEntry {
                    pass: request.pass,
                    probe: request.requested_probe.as_str().to_string(),
                    verdict: JournalVerdict::Deny,
                    sha256: None,
                    reason: Some(reason.clone()),
                })?;
                return Err(ExecError::Denied { reason });
            }
            Err(error) => return Err(error),
        };
        if fault == Fault::AfterAllow {
            self.append_journal(JournalEntry {
                pass: request.pass,
                probe: request.requested_probe.as_str().to_string(),
                verdict: JournalVerdict::Interrupted,
                sha256: None,
                reason: Some("interrupted".to_string()),
            })?;
            return Err(ExecError::Interrupted);
        }
        let sha256 = self.store(&bytes)?;
        self.append_journal(JournalEntry {
            pass: request.pass,
            probe: request.requested_probe.as_str().to_string(),
            verdict: JournalVerdict::Allow,
            sha256: Some(sha256.clone()),
            reason: None,
        })?;
        Ok(Evidence { sha256, bytes })
    }

    pub fn journal(&self) -> std::io::Result<Vec<JournalEntry>> {
        let text = fs::read_to_string(self.campaign.join("journal.jsonl"))?;
        let mut entries = Vec::new();
        for line in text.lines() {
            if line.is_empty() {
                continue;
            }
            entries.push(serde_json::from_str(line).map_err(io_invalid)?);
        }
        Ok(entries)
    }

    pub fn evidence_files(&self) -> std::io::Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        for entry in fs::read_dir(self.campaign.join("evidence"))? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                files.push(entry.path());
            }
        }
        files.sort();
        Ok(files)
    }

    fn read_request(&self, request: &ProbeRequest) -> Result<Vec<u8>, ExecError> {
        match request.requested_probe {
            ProbeName::PciId => self.read_named("pci.txt"),
            ProbeName::PciDrivers => self.read_named("pci_drivers.txt"),
            ProbeName::PciTree => self.read_named("pci_tree.txt"),
            ProbeName::PciSlot => self.read_slot(request),
            ProbeName::UsbBrief => self.read_named("usb.txt"),
            ProbeName::UsbTree => self.read_named("usb_tree.txt"),
            ProbeName::Modules => self.read_named("modules.txt"),
            ProbeName::DrmClass => self.read_named("drm.txt"),
            ProbeName::PciSysfsList => self.read_named("pci_sysfs.txt"),
            ProbeName::PlatformNodes => self.read_named("platform.txt"),
            ProbeName::DmiAllowlist => {
                let raw = self.read_named("dmi.txt")?;
                let text = String::from_utf8_lossy(&raw);
                Ok(filter_dmi(&text).into_bytes())
            }
            ProbeName::KernelWarningsRedacted => self.read_named("kernel_warnings.txt"),
            ProbeName::SysfsRead => self.read_sysfs(request),
        }
    }

    fn read_slot(&self, request: &ProbeRequest) -> Result<Vec<u8>, ExecError> {
        let slot = request
            .arguments
            .get("slot")
            .and_then(Value::as_str)
            .ok_or_else(|| ExecError::Denied {
                reason: "invalid_arguments".into(),
            })?;
        let raw = self.read_named("pci.txt")?;
        let text = String::from_utf8_lossy(&raw);
        let mut matched = String::new();
        for line in text.lines() {
            if line.starts_with(slot) {
                matched.push_str(line);
                matched.push('\n');
            }
        }
        if matched.is_empty() {
            return Err(ExecError::Denied {
                reason: "missing_fixture".into(),
            });
        }
        Ok(matched.into_bytes())
    }

    fn read_sysfs(&self, request: &ProbeRequest) -> Result<Vec<u8>, ExecError> {
        let path = request
            .arguments
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| ExecError::Denied {
                reason: "invalid_arguments".into(),
            })?;
        let Some(relative) = path.strip_prefix("/sys/") else {
            return Err(ExecError::Denied {
                reason: "path_escape".into(),
            });
        };
        if relative.is_empty()
            || relative.contains("..")
            || relative.contains('\\')
            || Path::new(relative).is_absolute()
        {
            return Err(ExecError::Denied {
                reason: "path_escape".into(),
            });
        }
        let candidate = self.fixture.join("sys").join(relative);
        self.read_inside_fixture(&candidate)
    }

    fn read_named(&self, name: &str) -> Result<Vec<u8>, ExecError> {
        if name.contains("..") || name.contains('/') || name.contains('\\') {
            return Err(ExecError::Denied {
                reason: "path_escape".into(),
            });
        }
        self.read_inside_fixture(&self.fixture.join(name))
    }

    fn read_inside_fixture(&self, candidate: &Path) -> Result<Vec<u8>, ExecError> {
        if !candidate.is_file() {
            return Err(ExecError::Denied {
                reason: "missing_fixture".into(),
            });
        }
        let root = self.fixture.canonicalize()?;
        let canon = candidate.canonicalize()?;
        if !canon.starts_with(&root) {
            return Err(ExecError::Denied {
                reason: "path_escape".into(),
            });
        }
        Ok(fs::read(canon)?)
    }

    fn store(&self, bytes: &[u8]) -> Result<String, ExecError> {
        let sha256 = hex::encode(Sha256::digest(bytes));
        let path = self.campaign.join("evidence").join(format!("{sha256}.txt"));
        if path.is_file() {
            return Ok(sha256);
        }
        let partial = path.with_extension("txt.partial");
        fs::write(&partial, bytes)?;
        fs::rename(&partial, &path)?;
        Ok(sha256)
    }

    fn append_journal(&self, entry: JournalEntry) -> Result<(), ExecError> {
        debug_assert!(entry
            .reason
            .as_deref()
            .is_none_or(|reason| { !reason.contains("..") && !reason.contains("/etc/") }));
        let mut line = serde_json::to_string(&entry).map_err(io_invalid)?;
        line.push('\n');
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.campaign.join("journal.jsonl"))?;
        file.write_all(line.as_bytes())?;
        Ok(())
    }
}

fn filter_dmi(text: &str) -> String {
    let mut out = String::new();
    for line in text.lines() {
        if DMI_KEYS.iter().any(|key| {
            line.starts_with(&format!("{key}:")) || line.starts_with(&format!("{key} :"))
        }) {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

fn preview(raw: &str) -> (u32, String) {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return (0, "invalid".to_string());
    };
    let pass = value
        .get("pass")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .min(u128::from(u32::MAX) as u64) as u32;
    let probe = value
        .get("requested_probe")
        .and_then(Value::as_str)
        .unwrap_or("invalid");
    (pass, safe_label(probe))
}

fn safe_label(label: &str) -> String {
    if label.is_empty()
        || label.len() > 64
        || label.contains("..")
        || label.contains('/')
        || label.contains('\\')
        || label
            .chars()
            .any(|ch| ch.is_control() || ch == '"' || ch == '\'')
    {
        "redacted".to_string()
    } else {
        label.to_string()
    }
}

fn reason_code(error: &ContractError) -> &'static str {
    match error {
        ContractError::UnknownProbe => "unknown_probe",
        ContractError::InvalidProbe => "invalid_probe",
        ContractError::InvalidArguments => "invalid_arguments",
        ContractError::ShellMetacharacters => "shell_metacharacters",
        ContractError::RiskNotReadOnly => "risk",
        ContractError::PathEscape => "path_escape",
        ContractError::PassLimit => "pass_limit",
        _ => "rejected",
    }
}

fn io_invalid(error: serde_json::Error) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, error)
}
