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
    /// S10 (ADR-038): which space this fact belongs to. `None` is a
    /// global/system fact, visible from every space (the same "shared
    /// across spaces" role S06's `SaaiOS` space already plays) --
    /// existing facts recorded before this field existed, and anything
    /// remembered by a caller with no space context (a direct console
    /// session), both land here rather than defaulting to any one real
    /// space.
    #[serde(default)]
    pub space_id: Option<String>,
    /// The `correlation_id` of the request that created this fact --
    /// "происхождение" (provenance), reusing the id the rest of the
    /// audit trail already keys everything by rather than inventing a
    /// second identifier for the same thing.
    #[serde(default)]
    pub origin_correlation_id: Option<Uuid>,
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
            space_id: None,
            origin_correlation_id: None,
        }
    }

    /// Whether a caller asking on behalf of `space_id` may see this fact:
    /// its own space, or a global fact (`None`) visible everywhere. A
    /// caller with no space context of its own (`space_id: None`, a
    /// direct console session) sees everything -- unchanged from this
    /// store's behavior before spaces existed here at all.
    fn visible_to(&self, space_id: Option<&str>) -> bool {
        match space_id {
            None => true,
            Some(space) => self.space_id.is_none() || self.space_id.as_deref() == Some(space),
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

    /// Latest non-deleted fact per key (append-only log compact view),
    /// unfiltered by space -- callers that need space isolation filter
    /// with `visible_to` themselves (`recall`/`list_recent`/`forget` all
    /// do). Kept unfiltered here since a `key` is only unique per space
    /// in principle; today keys aren't namespaced by space at all, so
    /// this compacts across all of them the same way it always has.
    pub fn latest_by_key(&self) -> Result<Vec<MemoryFact>> {
        let mut map = std::collections::HashMap::<String, MemoryFact>::new();
        for fact in self.read_all()? {
            map.insert(fact.key.clone(), fact);
        }
        let mut facts: Vec<_> = map.into_values().filter(|f| !f.deleted).collect();
        facts.sort_by_key(|b| std::cmp::Reverse(b.ts));
        Ok(facts)
    }

    /// `space_id: None` -- no space context (a direct console session) --
    /// sees every fact, exactly this store's behavior before ADR-038.
    /// `Some(space)` sees that space's own facts plus global ones.
    pub fn list_recent(&self, limit: usize, space_id: Option<&str>) -> Result<Vec<MemoryFact>> {
        let mut facts: Vec<_> = self
            .latest_by_key()?
            .into_iter()
            .filter(|f| f.visible_to(space_id))
            .collect();
        if facts.len() > limit {
            facts.truncate(limit);
        }
        Ok(facts)
    }

    /// Substring match on key, value, or tags (case-insensitive), scoped
    /// to `space_id` the same way `list_recent` is.
    pub fn recall(&self, query: &str, space_id: Option<&str>) -> Result<Vec<MemoryFact>> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return self.list_recent(20, space_id);
        }
        Ok(self
            .latest_by_key()?
            .into_iter()
            .filter(|f| f.visible_to(space_id))
            .filter(|f| {
                f.key.to_lowercase().contains(&q)
                    || f.value.to_lowercase().contains(&q)
                    || f.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect())
    }

    /// Only tombstones a fact visible to `space_id` -- a space-scoped
    /// caller cannot forget another space's fact just by knowing its key.
    pub fn forget(&self, key: &str, space_id: Option<&str>) -> Result<Option<MemoryFact>> {
        let latest = self
            .latest_by_key()?
            .into_iter()
            .find(|f| f.key == key && f.visible_to(space_id));
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

    pub fn format_context(&self, limit: usize, space_id: Option<&str>) -> Result<String> {
        let facts = self.list_recent(limit, space_id)?;
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

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
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
        fact.space_id = ctx.space_id.clone();
        fact.origin_correlation_id = Some(ctx.correlation_id);
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

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let facts = self
            .store
            .recall(&query, ctx.space_id.as_deref())
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

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolOutput, ToolError> {
        let key = args
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("key required".into()))?;
        let tomb = self
            .store
            .forget(key, ctx.space_id.as_deref())
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

        let hits = store.recall("pi5", None).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].key, "host.role");

        store.forget("host.role", None).unwrap();
        assert!(store.recall("pi5", None).unwrap().is_empty());
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
            space_id: None,
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

    fn ctx_for_space(space_id: Option<&str>) -> ToolContext {
        ToolContext {
            correlation_id: Uuid::new_v4(),
            call_id: Uuid::new_v4(),
            space_id: space_id.map(String::from),
        }
    }

    #[test]
    fn a_fact_remembered_in_one_space_does_not_leak_into_another() {
        let tmp = NamedTempFile::new().unwrap();
        let store = MemoryStore::open(tmp.path()).unwrap();
        let mut home_fact = MemoryFact::new("plant", "needs watering Tuesdays");
        home_fact.space_id = Some("home".into());
        store.remember(home_fact).unwrap();

        assert_eq!(store.recall("plant", Some("home")).unwrap().len(), 1);
        assert!(store.recall("plant", Some("work")).unwrap().is_empty());
    }

    #[test]
    fn a_global_fact_is_visible_from_every_space() {
        let tmp = NamedTempFile::new().unwrap();
        let store = MemoryStore::open(tmp.path()).unwrap();
        // space_id left None -- a global/system fact.
        store
            .remember(MemoryFact::new("timezone", "Europe/Kyiv"))
            .unwrap();

        assert_eq!(store.recall("timezone", Some("home")).unwrap().len(), 1);
        assert_eq!(store.recall("timezone", Some("work")).unwrap().len(), 1);
        assert_eq!(store.recall("timezone", None).unwrap().len(), 1);
    }

    #[test]
    fn a_caller_with_no_space_context_sees_every_fact_unchanged_from_before_adr_038() {
        let tmp = NamedTempFile::new().unwrap();
        let store = MemoryStore::open(tmp.path()).unwrap();
        let mut home_fact = MemoryFact::new("plant", "needs watering");
        home_fact.space_id = Some("home".into());
        store.remember(home_fact).unwrap();
        let mut work_fact = MemoryFact::new("deploy", "Fridays only");
        work_fact.space_id = Some("work".into());
        store.remember(work_fact).unwrap();

        assert_eq!(store.list_recent(20, None).unwrap().len(), 2);
    }

    #[test]
    fn forget_does_not_remove_another_spaces_fact() {
        let tmp = NamedTempFile::new().unwrap();
        let store = MemoryStore::open(tmp.path()).unwrap();
        let mut work_fact = MemoryFact::new("deploy", "Fridays only");
        work_fact.space_id = Some("work".into());
        store.remember(work_fact).unwrap();

        assert_eq!(store.forget("deploy", Some("home")).unwrap(), None);
        assert_eq!(store.recall("deploy", Some("work")).unwrap().len(), 1);

        store.forget("deploy", Some("work")).unwrap();
        assert!(store.recall("deploy", Some("work")).unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_tool_call_made_on_behalf_of_a_space_stamps_the_fact_with_it() {
        let tmp = NamedTempFile::new().unwrap();
        let store = Arc::new(MemoryStore::open(tmp.path()).unwrap());
        let mut reg = ToolRegistry::new();
        install_memory_tools(&mut reg, store);
        let ctx = ctx_for_space(Some("home"));
        let out = reg
            .execute(
                "memory.remember",
                json!({"key":"routine","value":"water plants"}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(out.ok);
        assert_eq!(out.value["space_id"], "home");
        assert_eq!(
            out.value["origin_correlation_id"],
            ctx.correlation_id.to_string()
        );
    }

    #[tokio::test]
    async fn recall_through_the_tool_is_scoped_to_the_callers_space() {
        let tmp = NamedTempFile::new().unwrap();
        let store = Arc::new(MemoryStore::open(tmp.path()).unwrap());
        let mut reg = ToolRegistry::new();
        install_memory_tools(&mut reg, store);

        reg.execute(
            "memory.remember",
            json!({"key":"routine","value":"water plants"}),
            &ctx_for_space(Some("home")),
        )
        .await
        .unwrap();

        let seen_from_home = reg
            .execute(
                "memory.recall",
                json!({"query":"routine"}),
                &ctx_for_space(Some("home")),
            )
            .await
            .unwrap();
        assert_eq!(seen_from_home.value["count"], 1);

        let seen_from_work = reg
            .execute(
                "memory.recall",
                json!({"query":"routine"}),
                &ctx_for_space(Some("work")),
            )
            .await
            .unwrap();
        assert_eq!(seen_from_work.value["count"], 0);
    }
}
