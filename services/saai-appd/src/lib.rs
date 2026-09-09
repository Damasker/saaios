use std::collections::HashSet;

use semver::Version;
use serde::Deserialize;
use thiserror::Error;

mod daemon;
mod store;
mod supervisor;
mod wire;

pub use daemon::{run_daemon, AppdError, DaemonConfig};
pub use store::{AppStore, InstalledApp, StoreError};
pub use supervisor::{
    AppEvent, AppEventKind, AppState, AppSupervisor, LaunchOutcome, SupervisorError,
    SupervisorPolicy,
};
pub use wire::{
    decode_request, encode_message, AppSummary, ClientRequest, LifecycleEvent, LifecycleEventKind,
    ProtocolError, ResponseResult, ServerMessage, WireError, APPD_WIRE_SCHEMA_V1,
    MAX_WIRE_MESSAGE_BYTES,
};

pub const MANIFEST_SCHEMA_V1: u32 = 1;
pub const MAX_APP_ID_LEN: usize = 253;
pub const MAX_APP_NAME_CHARS: usize = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiKind {
    Wayland,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppManifest {
    pub schema: u32,
    pub id: String,
    pub name: String,
    pub exec: String,
    pub version: Version,
    pub ui: UiKind,
    pub single_instance: bool,
    /// Requested capabilities only. S05 never turns these into grants.
    pub capabilities: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawManifest {
    schema: u32,
    id: String,
    name: String,
    exec: String,
    version: String,
    ui: String,
    single_instance: bool,
    capabilities: Vec<String>,
}

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("manifest is not valid TOML: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("unsupported manifest schema {0}")]
    UnsupportedSchema(u32),
    #[error("invalid application id {0:?}")]
    InvalidId(String),
    #[error("application name must contain 1..={MAX_APP_NAME_CHARS} trimmed characters")]
    InvalidName,
    #[error("exec must be a normalized relative path below bin/")]
    InvalidExec,
    #[error("version is not valid SemVer: {0}")]
    InvalidVersion(#[from] semver::Error),
    #[error("unsupported UI kind {0:?}")]
    UnsupportedUi(String),
    #[error("invalid capability name {0:?}")]
    InvalidCapability(String),
    #[error("duplicate capability {0:?}")]
    DuplicateCapability(String),
}

impl AppManifest {
    pub fn parse_toml(input: &str) -> Result<Self, ManifestError> {
        let raw: RawManifest = toml::from_str(input)?;
        if raw.schema != MANIFEST_SCHEMA_V1 {
            return Err(ManifestError::UnsupportedSchema(raw.schema));
        }
        if !valid_app_id(&raw.id) {
            return Err(ManifestError::InvalidId(raw.id));
        }
        let name_chars = raw.name.chars().count();
        if name_chars == 0 || name_chars > MAX_APP_NAME_CHARS || raw.name.trim() != raw.name {
            return Err(ManifestError::InvalidName);
        }
        if !valid_exec_path(&raw.exec) {
            return Err(ManifestError::InvalidExec);
        }
        let version = Version::parse(&raw.version)?;
        let ui = match raw.ui.as_str() {
            "wayland" => UiKind::Wayland,
            _ => return Err(ManifestError::UnsupportedUi(raw.ui)),
        };

        let mut unique = HashSet::with_capacity(raw.capabilities.len());
        for capability in &raw.capabilities {
            if !valid_dotted_name(capability, 2) {
                return Err(ManifestError::InvalidCapability(capability.clone()));
            }
            if !unique.insert(capability.as_str()) {
                return Err(ManifestError::DuplicateCapability(capability.clone()));
            }
        }

        Ok(Self {
            schema: raw.schema,
            id: raw.id,
            name: raw.name,
            exec: raw.exec,
            version,
            ui,
            single_instance: raw.single_instance,
            capabilities: raw.capabilities,
        })
    }
}

fn valid_app_id(value: &str) -> bool {
    value.len() <= MAX_APP_ID_LEN && valid_dotted_name(value, 3)
}

fn valid_dotted_name(value: &str, minimum_segments: usize) -> bool {
    let segments: Vec<_> = value.split('.').collect();
    segments.len() >= minimum_segments
        && segments.iter().all(|segment| {
            let bytes = segment.as_bytes();
            !bytes.is_empty()
                && bytes.len() <= 63
                && bytes[0].is_ascii_lowercase()
                && bytes[bytes.len() - 1].is_ascii_alphanumeric()
                && bytes
                    .iter()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
        })
}

fn valid_exec_path(value: &str) -> bool {
    if value.is_empty() || value.starts_with('/') || value.ends_with('/') || value.contains('\\') {
        return false;
    }
    let segments: Vec<_> = value.split('/').collect();
    segments.len() >= 2
        && segments[0] == "bin"
        && segments.iter().all(|segment| {
            !segment.is_empty()
                && *segment != "."
                && *segment != ".."
                && segment.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'+')
                })
        })
}

#[cfg(test)]
mod tests {
    use super::{AppManifest, ManifestError, UiKind};

    const VALID: &str = r#"
schema = 1
id = "org.saaios.example.notes"
name = "Заметки"
exec = "bin/notes"
version = "0.1.0"
ui = "wayland"
single_instance = true
capabilities = ["space.entities.read", "space.entities.write"]
"#;

    #[test]
    fn parses_manifest_v1() {
        let manifest = AppManifest::parse_toml(VALID).unwrap();
        assert_eq!(manifest.id, "org.saaios.example.notes");
        assert_eq!(manifest.version.to_string(), "0.1.0");
        assert_eq!(manifest.ui, UiKind::Wayland);
        assert!(manifest.single_instance);
    }

    #[test]
    fn rejects_unknown_schema() {
        let invalid = VALID.replacen("schema = 1", "schema = 2", 1);
        assert!(matches!(
            AppManifest::parse_toml(&invalid),
            Err(ManifestError::UnsupportedSchema(2))
        ));
    }

    #[test]
    fn rejects_unknown_field() {
        let invalid = format!("{VALID}\nroot = true\n");
        assert!(matches!(
            AppManifest::parse_toml(&invalid),
            Err(ManifestError::Toml(_))
        ));
    }

    #[test]
    fn rejects_unsafe_ids() {
        for id in [
            "notes",
            "org.saaios.Notes",
            "org.saaios../notes",
            "org..notes",
        ] {
            let invalid = VALID.replacen("org.saaios.example.notes", id, 1);
            assert!(matches!(
                AppManifest::parse_toml(&invalid),
                Err(ManifestError::InvalidId(_))
            ));
        }
    }

    #[test]
    fn rejects_exec_outside_normalized_bin_path() {
        for exec in [
            "/bin/notes",
            "../bin/notes",
            "bin/../notes",
            "notes",
            "bin\\notes",
        ] {
            let invalid = VALID.replacen("bin/notes", exec, 1);
            assert!(matches!(
                AppManifest::parse_toml(&invalid),
                Err(ManifestError::InvalidExec)
            ));
        }
    }

    #[test]
    fn rejects_invalid_version_and_ui() {
        let invalid_version = VALID.replacen("0.1.0", "latest", 1);
        assert!(matches!(
            AppManifest::parse_toml(&invalid_version),
            Err(ManifestError::InvalidVersion(_))
        ));

        let invalid_ui = VALID.replacen("wayland", "android", 1);
        assert!(matches!(
            AppManifest::parse_toml(&invalid_ui),
            Err(ManifestError::UnsupportedUi(_))
        ));
    }

    #[test]
    fn rejects_invalid_or_duplicate_capabilities() {
        let duplicate = VALID.replacen(
            "\"space.entities.read\", \"space.entities.write\"",
            "\"space.entities.read\", \"space.entities.read\"",
            1,
        );
        assert!(matches!(
            AppManifest::parse_toml(&duplicate),
            Err(ManifestError::DuplicateCapability(_))
        ));

        let invalid = VALID.replacen("space.entities.read", "Space.Read", 1);
        assert!(matches!(
            AppManifest::parse_toml(&invalid),
            Err(ManifestError::InvalidCapability(_))
        ));
    }

    #[test]
    fn rejects_untrimmed_or_empty_name() {
        for name in [" Заметки", ""] {
            let invalid = VALID.replacen("Заметки", name, 1);
            assert!(matches!(
                AppManifest::parse_toml(&invalid),
                Err(ManifestError::InvalidName)
            ));
        }
    }
}
