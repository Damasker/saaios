use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use thiserror::Error;

use crate::{valid_app_id, AppManifest, ManifestError};

const MANIFEST_FILE: &str = "manifest.toml";
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
const STAGING_ATTEMPTS: u64 = 32;
static NEXT_STAGE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledApp {
    pub manifest: AppManifest,
    pub code_dir: PathBuf,
    pub data_dir: PathBuf,
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("package root is not a directory: {0}")]
    PackageRoot(PathBuf),
    #[error("package source must be outside the managed app store: {0}")]
    SourceInsideStore(PathBuf),
    #[error("manifest exceeds the {MAX_MANIFEST_BYTES}-byte limit: {0}")]
    ManifestTooLarge(PathBuf),
    #[error("manifest validation failed at {path}: {source}")]
    Manifest {
        path: PathBuf,
        #[source]
        source: ManifestError,
    },
    #[error("application {0:?} is already installed")]
    Duplicate(String),
    #[error("invalid application id {0:?}")]
    InvalidAppId(String),
    #[error("package contains an unsupported file type: {0}")]
    UnsupportedEntry(PathBuf),
    #[error("application executable is missing or is not a regular file: {0}")]
    MissingExecutable(PathBuf),
    #[error("application executable is not executable: {0}")]
    NotExecutable(PathBuf),
    #[error("manifest changed while package was copied into staging")]
    ManifestChanged,
    #[error("installed directory name does not match manifest id at {0}")]
    InstalledIdMismatch(PathBuf),
    #[error("{operation} failed for {path}: {source}")]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

#[derive(Debug, Clone)]
pub struct AppStore {
    apps_dir: PathBuf,
    data_dir: PathBuf,
}

impl AppStore {
    pub fn new(saaios_data_root: impl AsRef<Path>) -> Self {
        let root = saaios_data_root.as_ref();
        Self {
            apps_dir: root.join("apps"),
            data_dir: root.join("var").join("apps"),
        }
    }

    pub fn apps_dir(&self) -> &Path {
        &self.apps_dir
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn ensure_layout(&self) -> Result<(), StoreError> {
        create_dir_all(&self.apps_dir)?;
        create_dir_all(&self.data_dir)?;
        Ok(())
    }

    pub fn install(&self, package_root: impl AsRef<Path>) -> Result<InstalledApp, StoreError> {
        self.ensure_layout()?;
        let package_root = package_root.as_ref();
        let package_metadata = symlink_metadata(package_root)?;
        if !package_metadata.file_type().is_dir() {
            return Err(StoreError::PackageRoot(package_root.to_path_buf()));
        }
        self.reject_managed_source(package_root)?;

        let source_manifest = read_manifest(&package_root.join(MANIFEST_FILE))?;
        let target = self.apps_dir.join(&source_manifest.id);
        if path_exists(&target)? {
            return Err(StoreError::Duplicate(source_manifest.id));
        }

        let staging = self.create_staging_dir(&source_manifest.id)?;
        let result = (|| {
            copy_tree(package_root, &staging)?;
            let staged_manifest = read_manifest(&staging.join(MANIFEST_FILE))?;
            if staged_manifest != source_manifest {
                return Err(StoreError::ManifestChanged);
            }
            verify_executable(&staging.join(&staged_manifest.exec))?;
            if path_exists(&target)? {
                return Err(StoreError::Duplicate(staged_manifest.id));
            }
            rename(&staging, &target)?;

            let app_data = self.data_dir.join(&staged_manifest.id);
            if let Err(error) = create_dir_all(&app_data) {
                let _ = fs::remove_dir_all(&target);
                return Err(error);
            }
            Ok(InstalledApp {
                manifest: staged_manifest,
                code_dir: target,
                data_dir: app_data,
            })
        })();

        if result.is_err() && staging.exists() {
            let _ = fs::remove_dir_all(&staging);
        }
        result
    }

    pub fn load(&self, app_id: &str) -> Result<Option<InstalledApp>, StoreError> {
        if !valid_app_id(app_id) {
            return Ok(None);
        }
        let code_dir = self.apps_dir.join(app_id);
        if !path_exists(&code_dir)? {
            return Ok(None);
        }
        let metadata = symlink_metadata(&code_dir)?;
        if !metadata.file_type().is_dir() {
            return Err(StoreError::UnsupportedEntry(code_dir));
        }
        let manifest = read_manifest(&code_dir.join(MANIFEST_FILE))?;
        if manifest.id != app_id {
            return Err(StoreError::InstalledIdMismatch(code_dir));
        }
        verify_executable(&code_dir.join(&manifest.exec))?;
        Ok(Some(InstalledApp {
            data_dir: self.data_dir.join(app_id),
            code_dir,
            manifest,
        }))
    }

    pub fn scan(&self) -> Result<Vec<InstalledApp>, StoreError> {
        self.ensure_layout()?;
        let entries = fs::read_dir(&self.apps_dir).map_err(|source| StoreError::Io {
            operation: "scan installed applications",
            path: self.apps_dir.clone(),
            source,
        })?;
        let mut app_ids = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|source| StoreError::Io {
                operation: "read installed application entry",
                path: self.apps_dir.clone(),
                source,
            })?;
            let name = entry.file_name();
            let Some(app_id) = name.to_str() else {
                return Err(StoreError::UnsupportedEntry(entry.path()));
            };
            if app_id.starts_with(".install-") {
                continue;
            }
            if !valid_app_id(app_id) {
                return Err(StoreError::UnsupportedEntry(entry.path()));
            }
            app_ids.push(app_id.to_owned());
        }
        app_ids.sort_unstable();

        let mut installed = Vec::with_capacity(app_ids.len());
        for app_id in app_ids {
            let Some(app) = self.load(&app_id)? else {
                return Err(StoreError::Io {
                    operation: "load application observed during scan",
                    path: self.apps_dir.join(app_id),
                    source: io::Error::new(
                        io::ErrorKind::NotFound,
                        "application disappeared during scan",
                    ),
                });
            };
            installed.push(app);
        }
        Ok(installed)
    }

    pub fn remove(&self, app_id: &str) -> Result<bool, StoreError> {
        if !valid_app_id(app_id) {
            return Err(StoreError::InvalidAppId(app_id.to_owned()));
        }
        let Some(installed) = self.load(app_id)? else {
            return Ok(false);
        };
        let tombstone = self.unique_sibling("remove", app_id)?;
        rename(&installed.code_dir, &tombstone)?;
        fs::remove_dir_all(&tombstone).map_err(|source| StoreError::Io {
            operation: "remove application code",
            path: tombstone,
            source,
        })?;
        Ok(true)
    }

    fn reject_managed_source(&self, package_root: &Path) -> Result<(), StoreError> {
        let source = canonicalize(package_root)?;
        for managed in [&self.apps_dir, &self.data_dir] {
            let managed = canonicalize(managed)?;
            if source.starts_with(&managed) {
                return Err(StoreError::SourceInsideStore(source));
            }
        }
        Ok(())
    }

    fn create_staging_dir(&self, app_id: &str) -> Result<PathBuf, StoreError> {
        let first = NEXT_STAGE.fetch_add(STAGING_ATTEMPTS, Ordering::Relaxed);
        for offset in 0..STAGING_ATTEMPTS {
            let staging = self.apps_dir.join(format!(
                ".install-{app_id}-{}-{}",
                std::process::id(),
                first + offset
            ));
            match fs::create_dir(&staging) {
                Ok(()) => return Ok(staging),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(source) => {
                    return Err(StoreError::Io {
                        operation: "create staging directory",
                        path: staging,
                        source,
                    });
                }
            }
        }
        Err(StoreError::Io {
            operation: "create unique staging directory",
            path: self.apps_dir.clone(),
            source: io::Error::new(io::ErrorKind::AlreadyExists, "staging names exhausted"),
        })
    }

    fn unique_sibling(&self, operation: &str, app_id: &str) -> Result<PathBuf, StoreError> {
        let first = NEXT_STAGE.fetch_add(STAGING_ATTEMPTS, Ordering::Relaxed);
        for offset in 0..STAGING_ATTEMPTS {
            let candidate = self.apps_dir.join(format!(
                ".{operation}-{app_id}-{}-{}",
                std::process::id(),
                first + offset
            ));
            match fs::symlink_metadata(&candidate) {
                Ok(_) => continue,
                Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(candidate),
                Err(source) => {
                    return Err(StoreError::Io {
                        operation: "inspect temporary package path",
                        path: candidate,
                        source,
                    });
                }
            }
        }
        Err(StoreError::Io {
            operation: "choose unique temporary package path",
            path: self.apps_dir.clone(),
            source: io::Error::new(io::ErrorKind::AlreadyExists, "temporary names exhausted"),
        })
    }
}

fn read_manifest(path: &Path) -> Result<AppManifest, StoreError> {
    let metadata = symlink_metadata(path)?;
    if !metadata.file_type().is_file() {
        return Err(StoreError::UnsupportedEntry(path.to_path_buf()));
    }
    if metadata.len() > MAX_MANIFEST_BYTES {
        return Err(StoreError::ManifestTooLarge(path.to_path_buf()));
    }
    let input = fs::read_to_string(path).map_err(|source| StoreError::Io {
        operation: "read manifest",
        path: path.to_path_buf(),
        source,
    })?;
    AppManifest::parse_toml(&input).map_err(|source| StoreError::Manifest {
        path: path.to_path_buf(),
        source,
    })
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), StoreError> {
    let entries = fs::read_dir(source).map_err(|source_error| StoreError::Io {
        operation: "read package directory",
        path: source.to_path_buf(),
        source: source_error,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source_error| StoreError::Io {
            operation: "read package entry",
            path: source.to_path_buf(),
            source: source_error,
        })?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type().map_err(|source_error| StoreError::Io {
            operation: "inspect package entry",
            path: source_path.clone(),
            source: source_error,
        })?;
        if file_type.is_dir() {
            create_dir(&destination_path)?;
            copy_tree(&source_path, &destination_path)?;
        } else if file_type.is_file() {
            fs::copy(&source_path, &destination_path).map_err(|source_error| StoreError::Io {
                operation: "copy package file",
                path: source_path,
                source: source_error,
            })?;
        } else {
            return Err(StoreError::UnsupportedEntry(source_path));
        }
    }
    Ok(())
}

fn verify_executable(path: &Path) -> Result<(), StoreError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(StoreError::MissingExecutable(path.to_path_buf()));
        }
        Err(source) => {
            return Err(StoreError::Io {
                operation: "inspect executable",
                path: path.to_path_buf(),
                source,
            });
        }
    };
    if !metadata.file_type().is_file() {
        return Err(StoreError::MissingExecutable(path.to_path_buf()));
    }
    if !is_executable(&metadata) {
        return Err(StoreError::NotExecutable(path.to_path_buf()));
    }
    Ok(())
}

#[cfg(unix)]
fn is_executable(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_metadata: &fs::Metadata) -> bool {
    true
}

fn path_exists(path: &Path) -> Result<bool, StoreError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(StoreError::Io {
            operation: "inspect path",
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn symlink_metadata(path: &Path) -> Result<fs::Metadata, StoreError> {
    fs::symlink_metadata(path).map_err(|source| StoreError::Io {
        operation: "inspect path",
        path: path.to_path_buf(),
        source,
    })
}

fn canonicalize(path: &Path) -> Result<PathBuf, StoreError> {
    fs::canonicalize(path).map_err(|source| StoreError::Io {
        operation: "canonicalize path",
        path: path.to_path_buf(),
        source,
    })
}

fn create_dir(path: &Path) -> Result<(), StoreError> {
    fs::create_dir(path).map_err(|source| StoreError::Io {
        operation: "create directory",
        path: path.to_path_buf(),
        source,
    })
}

fn create_dir_all(path: &Path) -> Result<(), StoreError> {
    fs::create_dir_all(path).map_err(|source| StoreError::Io {
        operation: "create directory tree",
        path: path.to_path_buf(),
        source,
    })
}

fn rename(source_path: &Path, destination_path: &Path) -> Result<(), StoreError> {
    fs::rename(source_path, destination_path).map_err(|source| StoreError::Io {
        operation: "commit staged package",
        path: destination_path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::TempDir;

    use super::{AppStore, StoreError};

    const APP_ID: &str = "org.saaios.example.notes";

    fn package(parent: &Path, directory: &str, id: &str, payload: &str) -> std::path::PathBuf {
        let root = parent.join(directory);
        fs::create_dir_all(root.join("bin")).unwrap();
        fs::write(
            root.join("manifest.toml"),
            format!(
                "schema = 1\nid = \"{id}\"\nname = \"Notes\"\nexec = \"bin/notes\"\nversion = \"0.1.0\"\nui = \"wayland\"\nsingle_instance = true\ncapabilities = []\n"
            ),
        )
        .unwrap();
        let executable = root.join("bin/notes");
        fs::write(&executable, payload).unwrap();
        make_executable(&executable);
        root
    }

    #[cfg(unix)]
    fn make_executable(path: &Path) {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[cfg(not(unix))]
    fn make_executable(_path: &Path) {}

    #[test]
    fn installs_code_and_creates_separate_data_directory() {
        let temporary = TempDir::new().unwrap();
        let source = package(temporary.path(), "package", APP_ID, "version-one");
        let store = AppStore::new(temporary.path().join("saaios"));

        let installed = store.install(&source).unwrap();

        assert_eq!(installed.manifest.id, APP_ID);
        assert_eq!(
            fs::read_to_string(installed.code_dir.join("bin/notes")).unwrap(),
            "version-one"
        );
        assert!(installed.data_dir.is_dir());
        assert!(store.apps_dir().read_dir().unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".install-")));
    }

    #[test]
    fn duplicate_does_not_replace_code_or_touch_neighbor_data() {
        let temporary = TempDir::new().unwrap();
        let store = AppStore::new(temporary.path().join("saaios"));
        let first = package(temporary.path(), "first", APP_ID, "original");
        store.install(&first).unwrap();
        let canary = store.data_dir().join("org.saaios.example.canary");
        fs::create_dir_all(&canary).unwrap();
        fs::write(canary.join("keep"), "untouched").unwrap();

        let duplicate = package(temporary.path(), "duplicate", APP_ID, "replacement");
        assert!(matches!(
            store.install(&duplicate),
            Err(StoreError::Duplicate(id)) if id == APP_ID
        ));
        assert_eq!(
            fs::read_to_string(store.apps_dir().join(APP_ID).join("bin/notes")).unwrap(),
            "original"
        );
        assert_eq!(
            fs::read_to_string(canary.join("keep")).unwrap(),
            "untouched"
        );
    }

    #[test]
    fn failed_validation_leaves_no_code_data_or_staging() {
        let temporary = TempDir::new().unwrap();
        let store = AppStore::new(temporary.path().join("saaios"));
        let source = package(temporary.path(), "package", APP_ID, "payload");
        fs::remove_file(source.join("bin/notes")).unwrap();

        assert!(matches!(
            store.install(&source),
            Err(StoreError::MissingExecutable(_))
        ));
        assert!(!store.apps_dir().join(APP_ID).exists());
        assert!(!store.data_dir().join(APP_ID).exists());
        assert!(store.apps_dir().read_dir().unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".install-")));
    }

    #[test]
    fn rejects_package_source_inside_managed_trees() {
        let temporary = TempDir::new().unwrap();
        let store = AppStore::new(temporary.path().join("saaios"));
        store.ensure_layout().unwrap();
        let source = package(store.data_dir(), "incoming", APP_ID, "payload");

        assert!(matches!(
            store.install(&source),
            Err(StoreError::SourceInsideStore(_))
        ));
        assert!(!store.apps_dir().join(APP_ID).exists());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_without_following_it() {
        use std::os::unix::fs::symlink;

        let temporary = TempDir::new().unwrap();
        let store = AppStore::new(temporary.path().join("saaios"));
        let source = package(temporary.path(), "package", APP_ID, "payload");
        let outside = temporary.path().join("outside-secret");
        fs::write(&outside, "must-not-copy").unwrap();
        symlink(&outside, source.join("secret-link")).unwrap();

        assert!(matches!(
            store.install(&source),
            Err(StoreError::UnsupportedEntry(_))
        ));
        assert!(!store.apps_dir().join(APP_ID).exists());
    }

    #[test]
    fn load_rejects_directory_manifest_id_mismatch() {
        let temporary = TempDir::new().unwrap();
        let store = AppStore::new(temporary.path().join("saaios"));
        store.ensure_layout().unwrap();
        let wrong_directory = store.apps_dir().join(APP_ID);
        let source = package(
            temporary.path(),
            "source",
            "org.saaios.example.other",
            "payload",
        );
        fs::rename(source, &wrong_directory).unwrap();

        assert!(matches!(
            store.load(APP_ID),
            Err(StoreError::InstalledIdMismatch(path)) if path == wrong_directory
        ));
    }

    #[test]
    fn scan_rebuilds_sorted_registry_from_installed_directories() {
        let temporary = TempDir::new().unwrap();
        let store = AppStore::new(temporary.path().join("saaios"));
        let second = package(
            temporary.path(),
            "second",
            "org.saaios.example.second",
            "second",
        );
        let first = package(
            temporary.path(),
            "first",
            "org.saaios.example.first",
            "first",
        );
        store.install(&second).unwrap();
        store.install(&first).unwrap();
        fs::create_dir(store.apps_dir().join(".install-interrupted")).unwrap();

        let installed = store.scan().unwrap();

        assert_eq!(
            installed
                .iter()
                .map(|app| app.manifest.id.as_str())
                .collect::<Vec<_>>(),
            ["org.saaios.example.first", "org.saaios.example.second"]
        );
    }

    #[test]
    fn scan_rejects_unmanaged_entries() {
        let temporary = TempDir::new().unwrap();
        let store = AppStore::new(temporary.path().join("saaios"));
        store.ensure_layout().unwrap();
        fs::write(store.apps_dir().join("not-an-app"), "unexpected").unwrap();

        assert!(matches!(store.scan(), Err(StoreError::UnsupportedEntry(_))));
    }

    #[test]
    fn remove_deletes_only_code_and_preserves_app_and_neighbor_data() {
        let temporary = TempDir::new().unwrap();
        let store = AppStore::new(temporary.path().join("saaios"));
        let source = package(temporary.path(), "package", APP_ID, "payload");
        let installed = store.install(&source).unwrap();
        fs::write(installed.data_dir.join("keep"), "app-data").unwrap();
        let canary = store.data_dir().join("org.saaios.example.canary");
        fs::create_dir_all(&canary).unwrap();
        fs::write(canary.join("keep"), "neighbor-data").unwrap();

        assert!(store.remove(APP_ID).unwrap());

        assert!(!installed.code_dir.exists());
        assert_eq!(
            fs::read_to_string(installed.data_dir.join("keep")).unwrap(),
            "app-data"
        );
        assert_eq!(
            fs::read_to_string(canary.join("keep")).unwrap(),
            "neighbor-data"
        );
        assert!(!store.remove(APP_ID).unwrap());
    }

    #[test]
    fn remove_rejects_unvalidated_id_before_path_use() {
        let temporary = TempDir::new().unwrap();
        let store = AppStore::new(temporary.path().join("saaios"));

        assert!(matches!(
            store.remove("../neighbor"),
            Err(StoreError::InvalidAppId(_))
        ));
    }
}
