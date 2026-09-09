use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::Capability;

pub const GRANT_SCHEMA_V1: u32 = 1;
const WRITE_ATTEMPTS: u64 = 32;
static NEXT_WRITE: AtomicU64 = AtomicU64::new(0);

/// The recorded outcome of showing the user the requested-capability screen
/// for one app (ADR-020 section 2). This is deliberately not a copy of
/// `AppManifest.capabilities`: `requested_hash` is what makes a manifest
/// update that adds capabilities force a fresh decision instead of quietly
/// inheriting an old "yes" that never covered the new request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrantRecord {
    pub schema: u32,
    pub app_id: String,
    /// Canonical capability names (`Capability::as_str()`), sorted and
    /// deduplicated. Empty when the user declined.
    pub granted: Vec<String>,
    /// SHA-256 over the sorted, deduplicated set of *requested* capability
    /// names -- see `GrantStore::requested_hash`.
    pub requested_hash: String,
}

#[derive(Debug, Error)]
pub enum GrantError {
    #[error("{operation} failed for {path}: {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("grant record at {path} is not valid JSON: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("unsupported grant schema {0}")]
    UnsupportedSchema(u32),
    #[error("grant record app_id {found:?} does not match requested {expected:?}")]
    AppIdMismatch { expected: String, found: String },
}

/// Durable store for effective-grant decisions, one JSON file per app,
/// deliberately outside every path `saai-appd` ever bind-mounts into a
/// sandboxed app's own namespace (ADR-020): an app that could edit its own
/// grant record could grant itself anything.
#[derive(Debug, Clone)]
pub struct GrantStore {
    grants_dir: PathBuf,
}

impl GrantStore {
    pub fn new(saaios_data_root: impl AsRef<Path>) -> Self {
        Self {
            grants_dir: saaios_data_root
                .as_ref()
                .join("var")
                .join("appd")
                .join("grants"),
        }
    }

    /// SHA-256 over the sorted, deduplicated set of capability names --
    /// independent of the order they appear in the manifest, so reordering
    /// the same set never forces a re-prompt, but adding or removing one
    /// always does.
    pub fn requested_hash(capabilities: &[Capability]) -> String {
        let mut names: Vec<&str> = capabilities.iter().map(|c| c.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        let mut hasher = Sha256::new();
        for name in &names {
            hasher.update(name.as_bytes());
            hasher.update(b"\n");
        }
        format!("{:x}", hasher.finalize())
    }

    fn path_for(&self, app_id: &str) -> PathBuf {
        self.grants_dir.join(format!("{app_id}.json"))
    }

    pub fn load(&self, app_id: &str) -> Result<Option<GrantRecord>, GrantError> {
        let path = self.path_for(app_id);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                return Err(GrantError::Io {
                    operation: "read",
                    path,
                    source,
                })
            }
        };
        let record: GrantRecord =
            serde_json::from_slice(&bytes).map_err(|source| GrantError::Json {
                path: path.clone(),
                source,
            })?;
        if record.schema != GRANT_SCHEMA_V1 {
            return Err(GrantError::UnsupportedSchema(record.schema));
        }
        if record.app_id != app_id {
            return Err(GrantError::AppIdMismatch {
                expected: app_id.to_owned(),
                found: record.app_id,
            });
        }
        Ok(Some(record))
    }

    /// True when a stored decision exists and its `requested_hash` still
    /// matches exactly what `capabilities` requests today -- i.e. the user
    /// has already been asked about this exact set and consent does not
    /// need to be re-collected before the next launch.
    pub fn covers(&self, app_id: &str, capabilities: &[Capability]) -> Result<bool, GrantError> {
        if capabilities.is_empty() {
            // Nothing requested, nothing to ask about -- trivially covered
            // even before any decision has ever been recorded for this
            // app_id.
            return Ok(true);
        }
        let expected = Self::requested_hash(capabilities);
        Ok(self
            .load(app_id)?
            .is_some_and(|record| record.requested_hash == expected))
    }

    /// Returns the capabilities actually granted (empty if never decided,
    /// declined, or the stored decision no longer covers the current
    /// request -- callers must not treat a stale record as a grant).
    pub fn effective_capabilities(
        &self,
        app_id: &str,
        capabilities: &[Capability],
    ) -> Result<Vec<Capability>, GrantError> {
        if capabilities.is_empty() {
            return Ok(Vec::new());
        }
        if !self.covers(app_id, capabilities)? {
            return Ok(Vec::new());
        }
        let record = self
            .load(app_id)?
            .expect("covers() just confirmed a record exists for a non-empty request");
        Ok(capabilities
            .iter()
            .copied()
            .filter(|capability| {
                record
                    .granted
                    .iter()
                    .any(|name| name == capability.as_str())
            })
            .collect())
    }

    /// Records the user's accept/decline-all decision for the capabilities
    /// currently requested by the manifest. `accepted = false` stores an
    /// empty `granted` list (a real decision, not "unknown") -- covers()
    /// still returns true for the same request afterwards, so the app does
    /// not get re-prompted on every launch after being declined once.
    pub fn record_decision(
        &self,
        app_id: &str,
        capabilities: &[Capability],
        accepted: bool,
    ) -> Result<GrantRecord, GrantError> {
        fs::create_dir_all(&self.grants_dir).map_err(|source| GrantError::Io {
            operation: "create_dir_all",
            path: self.grants_dir.clone(),
            source,
        })?;
        let mut granted: Vec<String> = if accepted {
            capabilities
                .iter()
                .map(|capability| capability.as_str().to_owned())
                .collect()
        } else {
            Vec::new()
        };
        granted.sort_unstable();
        granted.dedup();
        let record = GrantRecord {
            schema: GRANT_SCHEMA_V1,
            app_id: app_id.to_owned(),
            granted,
            requested_hash: Self::requested_hash(capabilities),
        };
        self.write_atomic(app_id, &record)?;
        Ok(record)
    }

    fn write_atomic(&self, app_id: &str, record: &GrantRecord) -> Result<(), GrantError> {
        let mut bytes = serde_json::to_vec(record).map_err(|source| GrantError::Json {
            path: self.path_for(app_id),
            source,
        })?;
        bytes.push(b'\n');

        let target = self.path_for(app_id);
        let first = NEXT_WRITE.fetch_add(WRITE_ATTEMPTS, Ordering::Relaxed);
        let mut temporary = None;
        for offset in 0..WRITE_ATTEMPTS {
            let candidate = self.grants_dir.join(format!(
                ".{app_id}.{}.{}.tmp",
                std::process::id(),
                first + offset
            ));
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(&candidate) {
                Ok(mut file) => {
                    file.write_all(&bytes).map_err(|source| GrantError::Io {
                        operation: "write",
                        path: candidate.clone(),
                        source,
                    })?;
                    file.sync_all().map_err(|source| GrantError::Io {
                        operation: "fsync",
                        path: candidate.clone(),
                        source,
                    })?;
                    temporary = Some(candidate);
                    break;
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(source) => {
                    return Err(GrantError::Io {
                        operation: "create",
                        path: candidate,
                        source,
                    })
                }
            }
        }
        let temporary = temporary.ok_or_else(|| GrantError::Io {
            operation: "create",
            path: self.grants_dir.clone(),
            source: io::Error::new(io::ErrorKind::AlreadyExists, "exhausted temp name attempts"),
        })?;

        if let Err(source) = fs::rename(&temporary, &target) {
            let _ = fs::remove_file(&temporary);
            return Err(GrantError::Io {
                operation: "rename",
                path: target,
                source,
            });
        }
        File::open(&self.grants_dir)
            .and_then(|dir| dir.sync_all())
            .map_err(|source| GrantError::Io {
                operation: "fsync_dir",
                path: self.grants_dir.clone(),
                source,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{GrantError, GrantStore};
    use crate::Capability;
    use tempfile::tempdir;

    #[test]
    fn no_record_means_no_coverage_and_no_grants() {
        let root = tempdir().unwrap();
        let store = GrantStore::new(root.path());
        let requested = [Capability::NetInternet];
        assert!(!store.covers("org.saaios.example", &requested).unwrap());
        assert_eq!(
            store
                .effective_capabilities("org.saaios.example", &requested)
                .unwrap(),
            Vec::new()
        );
    }

    #[test]
    fn accepted_decision_grants_exactly_the_requested_set() {
        let root = tempdir().unwrap();
        let store = GrantStore::new(root.path());
        let requested = [Capability::SpaceEntitiesRead, Capability::NetInternet];
        store
            .record_decision("org.saaios.example", &requested, true)
            .unwrap();
        assert!(store.covers("org.saaios.example", &requested).unwrap());
        let mut granted = store
            .effective_capabilities("org.saaios.example", &requested)
            .unwrap();
        granted.sort_by_key(|capability| capability.as_str());
        assert_eq!(
            granted,
            [Capability::NetInternet, Capability::SpaceEntitiesRead]
        );
    }

    #[test]
    fn declined_decision_is_remembered_as_zero_grants_not_unknown() {
        let root = tempdir().unwrap();
        let store = GrantStore::new(root.path());
        let requested = [Capability::NetInternet];
        store
            .record_decision("org.saaios.example", &requested, false)
            .unwrap();
        // Covered (user was asked and answered), but nothing granted.
        assert!(store.covers("org.saaios.example", &requested).unwrap());
        assert_eq!(
            store
                .effective_capabilities("org.saaios.example", &requested)
                .unwrap(),
            Vec::new()
        );
    }

    #[test]
    fn expanded_manifest_request_invalidates_old_consent() {
        let root = tempdir().unwrap();
        let store = GrantStore::new(root.path());
        let original = [Capability::SpaceEntitiesRead];
        store
            .record_decision("org.saaios.example", &original, true)
            .unwrap();

        let expanded = [Capability::SpaceEntitiesRead, Capability::NetInternet];
        assert!(!store.covers("org.saaios.example", &expanded).unwrap());
        assert_eq!(
            store
                .effective_capabilities("org.saaios.example", &expanded)
                .unwrap(),
            Vec::new()
        );

        // The narrower, previously-accepted set is unaffected.
        assert!(store.covers("org.saaios.example", &original).unwrap());
    }

    #[test]
    fn reordered_request_is_still_the_same_hash() {
        let root = tempdir().unwrap();
        let store = GrantStore::new(root.path());
        store
            .record_decision(
                "org.saaios.example",
                &[Capability::SpaceEntitiesRead, Capability::NetInternet],
                true,
            )
            .unwrap();
        assert!(store
            .covers(
                "org.saaios.example",
                &[Capability::NetInternet, Capability::SpaceEntitiesRead],
            )
            .unwrap());
    }

    #[test]
    fn empty_request_never_needs_consent() {
        let root = tempdir().unwrap();
        let store = GrantStore::new(root.path());
        // No decision has ever been recorded for this app_id -- an empty
        // request must still be trivially covered, not treated as unknown.
        assert!(store.covers("org.saaios.example", &[]).unwrap());
        assert_eq!(
            store
                .effective_capabilities("org.saaios.example", &[])
                .unwrap(),
            Vec::new()
        );
    }

    #[test]
    fn rejects_record_with_mismatched_schema() {
        let root = tempdir().unwrap();
        let store = GrantStore::new(root.path());
        std::fs::create_dir_all(root.path().join("var").join("appd").join("grants")).unwrap();
        std::fs::write(
            root.path()
                .join("var")
                .join("appd")
                .join("grants")
                .join("org.saaios.example.json"),
            r#"{"schema":2,"app_id":"org.saaios.example","granted":[],"requested_hash":"x"}"#,
        )
        .unwrap();
        assert!(matches!(
            store.load("org.saaios.example"),
            Err(GrantError::UnsupportedSchema(2))
        ));
    }
}
