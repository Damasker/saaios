use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use protocol::{Envelope, MessageKind};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditRecord {
    pub id: Uuid,
    pub ts: DateTime<Utc>,
    pub correlation_id: Uuid,
    pub causation_id: Option<Uuid>,
    pub kind: MessageKind,
    pub payload: serde_json::Value,
}

impl From<&Envelope> for AuditRecord {
    fn from(env: &Envelope) -> Self {
        Self {
            id: env.msg_id,
            ts: env.ts,
            correlation_id: env.correlation_id,
            causation_id: env.causation_id,
            kind: env.kind.clone(),
            payload: env.payload.clone(),
        }
    }
}

#[derive(Debug)]
pub struct AuditLog {
    path: PathBuf,
    file: Mutex<File>,
}

impl AuditLog {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)
            .with_context(|| format!("open audit log {}", path.display()))?;
        Ok(Self {
            path,
            file: Mutex::new(file),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn append_envelope(&self, env: &Envelope) -> Result<()> {
        self.append(&AuditRecord::from(env))
    }

    pub fn append(&self, record: &AuditRecord) -> Result<()> {
        let mut line = serde_json::to_string(record)?;
        line.push('\n');
        let mut file = self.file.lock().expect("audit lock");
        file.write_all(line.as_bytes())?;
        file.flush()?;
        Ok(())
    }

    pub fn list_by_correlation(&self, correlation_id: Uuid) -> Result<Vec<AuditRecord>> {
        Ok(self
            .read_all()?
            .into_iter()
            .filter(|r| r.correlation_id == correlation_id)
            .collect())
    }

    pub fn read_all(&self) -> Result<Vec<AuditRecord>> {
        let file =
            File::open(&self.path).with_context(|| format!("read {}", self.path.display()))?;
        let reader = BufReader::new(file);
        let mut out = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            out.push(serde_json::from_str(&line)?);
        }
        Ok(out)
    }

    /// Dry-run replay: reconstruct a request chain without executing tools.
    pub fn replay(&self, correlation_id: Uuid) -> Result<ReplayReport> {
        let records = self.list_by_correlation(correlation_id)?;
        if records.is_empty() {
            anyhow::bail!("no audit records for correlation_id={correlation_id}");
        }

        let mut steps = Vec::new();
        let mut side_effects = Vec::new();
        let mut summary = None;
        let mut pending_confirmation = None;

        for record in &records {
            let step = match &record.kind {
                MessageKind::UserRequest => {
                    let text = record
                        .payload
                        .get("text")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    ReplayStep {
                        kind: record.kind.clone(),
                        description: format!("user request: {text}"),
                        side_effect: false,
                    }
                }
                MessageKind::ToolCall => {
                    let tool = record
                        .payload
                        .get("tool")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let dangerous = matches!(
                        tool,
                        "process.kill_request" | "system.reboot_request" | "storage.format"
                    );
                    if dangerous {
                        side_effects.push(format!(
                            "would invoke `{tool}` args={}",
                            record.payload.get("arguments").cloned().unwrap_or_default()
                        ));
                    }
                    ReplayStep {
                        kind: record.kind.clone(),
                        description: format!("tool call: {tool}"),
                        side_effect: dangerous,
                    }
                }
                MessageKind::PolicyDecision => {
                    let tool = record
                        .payload
                        .get("tool")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let verdict = record.payload.get("verdict").cloned().unwrap_or_default();
                    ReplayStep {
                        kind: record.kind.clone(),
                        description: format!("policy for `{tool}`: {verdict}"),
                        side_effect: false,
                    }
                }
                MessageKind::ToolResult => {
                    let tool = record
                        .payload
                        .get("tool")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let ok = record
                        .payload
                        .get("ok")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    ReplayStep {
                        kind: record.kind.clone(),
                        description: format!("tool result: {tool} ok={ok}"),
                        side_effect: false,
                    }
                }
                MessageKind::ConfirmationRequest => {
                    let summary_text = record
                        .payload
                        .get("summary")
                        .and_then(|v| v.as_str())
                        .unwrap_or("confirmation required")
                        .to_string();
                    pending_confirmation = Some(summary_text.clone());
                    ReplayStep {
                        kind: record.kind.clone(),
                        description: format!("confirmation requested: {summary_text}"),
                        side_effect: false,
                    }
                }
                MessageKind::ConfirmationResponse => {
                    let confirmed = record
                        .payload
                        .get("confirmed")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    ReplayStep {
                        kind: record.kind.clone(),
                        description: format!("confirmation response: confirmed={confirmed}"),
                        side_effect: false,
                    }
                }
                MessageKind::DiagnoseResult => {
                    let s = record
                        .payload
                        .get("summary")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    summary = Some(s.clone());
                    ReplayStep {
                        kind: record.kind.clone(),
                        description: format!("diagnose: {s}"),
                        side_effect: false,
                    }
                }
                MessageKind::AssistantMessage => {
                    let text = record
                        .payload
                        .get("text")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if summary.is_none() {
                        summary = Some(text.clone());
                    }
                    ReplayStep {
                        kind: record.kind.clone(),
                        description: format!("assistant: {text}"),
                        side_effect: false,
                    }
                }
                _other => ReplayStep {
                    kind: record.kind.clone(),
                    description: format!("event: {}", record.payload),
                    side_effect: false,
                },
            };
            steps.push(step);
        }

        Ok(ReplayReport {
            correlation_id,
            steps,
            side_effects_avoided: side_effects,
            summary,
            pending_confirmation,
            dry_run: true,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReplayStep {
    pub kind: MessageKind,
    pub description: String,
    pub side_effect: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReplayReport {
    pub correlation_id: Uuid,
    pub steps: Vec<ReplayStep>,
    pub side_effects_avoided: Vec<String>,
    pub summary: Option<String>,
    pub pending_confirmation: Option<String>,
    pub dry_run: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::{Envelope, MessageKind};
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn causation_chain_roundtrip() {
        let dir = tempdir().unwrap();
        let log = AuditLog::open(dir.path().join("audit.jsonl")).unwrap();
        let corr = Uuid::new_v4();
        let req = Envelope::new(
            MessageKind::UserRequest,
            corr,
            None,
            json!({"text": "slow"}),
        );
        let tool = Envelope::new(
            MessageKind::ToolCall,
            corr,
            Some(req.msg_id),
            json!({"tool": "system.metrics"}),
        );
        log.append_envelope(&req).unwrap();
        log.append_envelope(&tool).unwrap();
        let chain = log.list_by_correlation(corr).unwrap();
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].kind, MessageKind::UserRequest);
        assert_eq!(chain[1].causation_id, Some(req.msg_id));
    }

    #[test]
    fn replay_is_dry_run_and_flags_dangerous_tools() {
        let dir = tempdir().unwrap();
        let log = AuditLog::open(dir.path().join("audit.jsonl")).unwrap();
        let corr = Uuid::new_v4();
        log.append_envelope(&Envelope::new(
            MessageKind::UserRequest,
            corr,
            None,
            json!({"text": "Почему тормозит?"}),
        ))
        .unwrap();
        log.append_envelope(&Envelope::new(
            MessageKind::ToolCall,
            corr,
            None,
            json!({"tool": "system.metrics", "arguments": {}}),
        ))
        .unwrap();
        log.append_envelope(&Envelope::new(
            MessageKind::ToolCall,
            corr,
            None,
            json!({"tool": "process.kill_request", "arguments": {"pid": 4312}}),
        ))
        .unwrap();
        log.append_envelope(&Envelope::new(
            MessageKind::PolicyDecision,
            corr,
            None,
            json!({"tool": "process.kill_request", "verdict": "ask_user"}),
        ))
        .unwrap();
        log.append_envelope(&Envelope::new(
            MessageKind::DiagnoseResult,
            corr,
            None,
            json!({"summary": "runaway-worker is hot"}),
        ))
        .unwrap();

        let report = log.replay(corr).unwrap();
        assert!(report.dry_run);
        assert_eq!(report.steps.len(), 5);
        assert!(report
            .side_effects_avoided
            .iter()
            .any(|s| s.contains("process.kill_request")));
        assert_eq!(report.summary.as_deref(), Some("runaway-worker is hot"));
    }
}
