use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tool_registry::{
    RiskLevel, ToolContext, ToolError, ToolExecutor, ToolOutput, ToolRegistry, ToolSpec,
};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryFact {
    pub id: Uuid,
    pub ts: DateTime<Utc>,
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub source: Option<String>,
    /// Soft-delete marker; recall skips deleted facts.
    #[serde(default)]
    pub deleted: bool,
}

impl MemoryFact {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            ts: Utc::now(),
            key: key.into(),
            value: value.into(),
            tags: Vec::new(),
            source: None,
            deleted: false,
        }
    }
}

#[derive(Debug)]
pub struct MemoryStore {
    path: PathBuf,
    file: Mutex<File>,
}

impl MemoryStore {
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
            .with_context(|| format!("open memory store {}", path.display()))?;
        Ok(Self {
            path,
            file: Mutex::new(file),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn remember(&self, mut fact: MemoryFact) -> Result<MemoryFact> {
        if fact.id.is_nil() {
            fact.id = Uuid::new_v4();
        }
        if fact.ts.timestamp() == 0 {
            fact.ts = Utc::now();
        }
        self.append(&fact)?;
        Ok(fact)
    }

    pub fn append(&self, fact: &MemoryFact) -> Result<()> {
        let mut line = serde_json::to_string(fact)?;
        line.push('\n');
        let mut file = self.file.lock().expect("memory lock");
        file.write_all(line.as_bytes())?;
        file.flush()?;
        Ok(())
    }

    pub fn read_all(&self) -> Result<Vec<MemoryFact>> {
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

    /// Latest non-deleted fact per key (append-only log compact view).
    pub fn latest_by_key(&self) -> Result<Vec<MemoryFact>> {
        let mut map = std::collections::HashMap::<String, MemoryFact>::new();
        for fact in self.read_all()? {
            map.insert(fact.key.clone(), fact);
        }
        let mut facts: Vec<_> = map.into_values().filter(|f| !f.deleted).collect();
        facts.sort_by_key(|b| std::cmp::Reverse(b.ts));
        Ok(facts)
    }

    pub fn list_recent(&self, limit: usize) -> Result<Vec<MemoryFact>> {
        let mut facts = self.latest_by_key()?;
        if facts.len() > limit {
            facts.truncate(limit);
        }
        Ok(facts)
    }

    /// Substring match on key, value, or tags (case-insensitive).
    pub fn recall(&self, query: &str) -> Result<Vec<MemoryFact>> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return self.list_recent(20);
        }
        Ok(self
            .latest_by_key()?
            .into_iter()
            .filter(|f| {
                f.key.to_lowercase().contains(&q)
                    || f.value.to_lowercase().contains(&q)
                    || f.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect())
    }

    pub fn forget(&self, key: &str) -> Result<Option<MemoryFact>> {
        let latest = self.latest_by_key()?.into_iter().find(|f| f.key == key);
        let Some(prev) = latest else {
            return Ok(None);
        };
        let mut tomb = prev.clone();
        tomb.id = Uuid::new_v4();
        tomb.ts = Utc::now();
        tomb.deleted = true;
        self.append(&tomb)?;
        Ok(Some(tomb))
    }

    pub fn format_context(&self, limit: usize) -> Result<String> {
        let facts = self.list_recent(limit)?;
        if facts.is_empty() {
            return Ok(String::new());
        }
        let mut out = String::from("\n\nKnown facts (memory):\n");
        for f in facts {
            out.push_str(&format!("- {}: {}\n", f.key, f.value));
        }
        Ok(out)
    }
}

pub fn install_memory_tools(registry: &mut ToolRegistry, store: Arc<MemoryStore>) {
    registry.register(Arc::new(RememberTool {
        store: store.clone(),
        spec: ToolSpec {
            name: "memory.remember".into(),
            description: "Store a durable fact (key/value) in local memory".into(),
            risk: RiskLevel::Low,
            timeout_ms: 1000,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "key": {"type": "string"},
                    "value": {"type": "string"},
                    "tags": {"type": "array", "items": {"type": "string"}}
                },
                "required": ["key", "value"]
            }),
            output_schema: json!({"type": "object"}),
            requires_confirmation: false,
        },
    }));

    registry.register(Arc::new(RecallTool {
        store: store.clone(),
        spec: ToolSpec {
            name: "memory.recall".into(),
            description: "Search local memory facts by substring query".into(),
            risk: RiskLevel::Low,
            timeout_ms: 1000,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string"}
                },
                "required": ["query"]
            }),
            output_schema: json!({"type": "object"}),
            requires_confirmation: false,
        },
    }));

    registry.register(Arc::new(ForgetTool {
        store,
        spec: ToolSpec {
            name: "memory.forget".into(),
            description: "Soft-delete a memory fact by key".into(),
            risk: RiskLevel::Medium,
            timeout_ms: 1000,
            input_schema: json!({
                "type": "object",
                "properties": {
                    "key": {"type": "string"}
                },
                "required": ["key"]
            }),
            output_schema: json!({"type": "object"}),
            requires_confirmation: false,
        },
    }));
}

struct RememberTool {
    store: Arc<MemoryStore>,
    spec: ToolSpec,
}

struct RecallTool {
    store: Arc<MemoryStore>,
    spec: ToolSpec,
}

struct ForgetTool {
    store: Arc<MemoryStore>,
    spec: ToolSpec,
}

#[async_trait]
impl ToolExecutor for RememberTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
        let key = args
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("key required".into()))?
            .to_string();
        let value = args
            .get("value")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("value required".into()))?
            .to_string();
        let tags = args
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| t.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let mut fact = MemoryFact::new(key, value);
        fact.tags = tags;
        fact.source = Some("tool".into());
        let fact = self
            .store
            .remember(fact)
            .map_err(|e| ToolError::Execution(e.to_string()))?;
        Ok(ToolOutput {
            ok: true,
            value: serde_json::to_value(fact).unwrap_or(json!({})),
            error: None,
        })
    }
}

#[async_trait]
impl ToolExecutor for RecallTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let facts = self
            .store
            .recall(&query)
            .map_err(|e| ToolError::Execution(e.to_string()))?;
        Ok(ToolOutput {
            ok: true,
            value: json!({ "facts": facts, "count": facts.len() }),
            error: None,
        })
    }
}

#[async_trait]
impl ToolExecutor for ForgetTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
        let key = args
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("key required".into()))?;
        let tomb = self
            .store
            .forget(key)
            .map_err(|e| ToolError::Execution(e.to_string()))?;
        Ok(ToolOutput {
            ok: true,
            value: json!({ "forgotten": tomb.is_some(), "key": key }),
            error: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn remember_recall_forget() {
        let tmp = NamedTempFile::new().unwrap();
        let store = MemoryStore::open(tmp.path()).unwrap();
        store
            .remember(MemoryFact::new("host.role", "pi5 appliance"))
            .unwrap();
        store
            .remember(MemoryFact::new("owner", "Mykhailo"))
            .unwrap();

        let hits = store.recall("pi5").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].key, "host.role");

        store.forget("host.role").unwrap();
        assert!(store.recall("pi5").unwrap().is_empty());
        assert_eq!(store.latest_by_key().unwrap().len(), 1);
    }

    #[test]
    fn latest_wins_for_same_key() {
        let tmp = NamedTempFile::new().unwrap();
        let store = MemoryStore::open(tmp.path()).unwrap();
        store.remember(MemoryFact::new("city", "Kyiv")).unwrap();
        store.remember(MemoryFact::new("city", "Odesa")).unwrap();
        let facts = store.latest_by_key().unwrap();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].value, "Odesa");
    }

    #[tokio::test]
    async fn tools_roundtrip() {
        let tmp = NamedTempFile::new().unwrap();
        let store = Arc::new(MemoryStore::open(tmp.path()).unwrap());
        let mut reg = ToolRegistry::new();
        install_memory_tools(&mut reg, store);
        let ctx = ToolContext {
            correlation_id: Uuid::new_v4(),
            call_id: Uuid::new_v4(),
        };
        let out = reg
            .execute("memory.remember", json!({"key":"lang","value":"uk"}), &ctx)
            .await
            .unwrap();
        assert!(out.ok);
        let out = reg
            .execute("memory.recall", json!({"query":"lang"}), &ctx)
            .await
            .unwrap();
        assert_eq!(out.value["count"], 1);
    }
}
