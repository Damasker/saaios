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
}
