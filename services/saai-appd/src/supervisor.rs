use std::collections::{HashMap, VecDeque};
use std::io;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use thiserror::Error;

use crate::sandbox::{self, SandboxPaths};
use crate::{AppStore, Capability, InstalledApp, StoreError};

pub const DEFAULT_CRASH_LIMIT: usize = 3;
pub const DEFAULT_CRASH_WINDOW: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupervisorPolicy {
    pub crash_limit: usize,
    pub crash_window: Duration,
}

impl Default for SupervisorPolicy {
    fn default() -> Self {
        Self {
            crash_limit: DEFAULT_CRASH_LIMIT,
            crash_window: DEFAULT_CRASH_WINDOW,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    Running,
    Stopped,
    CrashLimited,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEventKind {
    Running,
    Stopped,
    Crashed,
    CrashLimited,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppEvent {
    pub app_id: String,
    pub kind: AppEventKind,
    pub pid: Option<u32>,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchOutcome {
    Started { pid: u32 },
    Existing { pid: u32 },
}

#[derive(Debug, Error)]
pub enum SupervisorError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("application {0:?} is not installed")]
    NotInstalled(String),
    #[error("supervisor policy requires a non-zero crash limit and window")]
    InvalidPolicy,
    #[error("failed to {operation} application {app_id:?}: {source}")]
    Process {
        operation: &'static str,
        app_id: String,
        #[source]
        source: io::Error,
    },
}

#[derive(Debug)]
struct AppRuntime {
    installed: InstalledApp,
    children: Vec<Child>,
    failures: VecDeque<Instant>,
    crash_limited: bool,
    /// The capability set this instance was actually launched with --
    /// `poll()`'s crash-restart reuses this exact set (ADR-020: a
    /// restarted process must get the same sandbox as its first launch,
    /// not silently re-derive it from whatever the grant store says at
    /// the moment it happens to crash).
    granted: Vec<Capability>,
}

impl AppRuntime {
    fn new(installed: InstalledApp) -> Self {
        Self {
            installed,
            children: Vec::new(),
            failures: VecDeque::new(),
            crash_limited: false,
            granted: Vec::new(),
        }
    }

    fn record_failure(&mut self, now: Instant, policy: SupervisorPolicy) -> usize {
        while self
            .failures
            .front()
            .is_some_and(|failure| now.duration_since(*failure) >= policy.crash_window)
        {
            self.failures.pop_front();
        }
        self.failures.push_back(now);
        self.failures.len()
    }
}

#[derive(Debug)]
pub struct AppSupervisor {
    store: AppStore,
    /// Only used to derive the entity-store mask path (ADR-020 section 4)
    /// -- `store.apps_dir()`/`store.data_dir()` already cover the other
    /// two masked parents.
    data_root: PathBuf,
    runtime_dir: PathBuf,
    wayland_display: String,
    policy: SupervisorPolicy,
    apps: HashMap<String, AppRuntime>,
}

impl AppSupervisor {
    pub fn new(
        store: AppStore,
        data_root: impl AsRef<Path>,
        runtime_dir: impl AsRef<Path>,
        wayland_display: impl Into<String>,
    ) -> Self {
        Self {
            store,
            data_root: data_root.as_ref().to_path_buf(),
            runtime_dir: runtime_dir.as_ref().to_path_buf(),
            wayland_display: wayland_display.into(),
            policy: SupervisorPolicy::default(),
            apps: HashMap::new(),
        }
    }

    pub fn with_policy(
        store: AppStore,
        data_root: impl AsRef<Path>,
        runtime_dir: impl AsRef<Path>,
        wayland_display: impl Into<String>,
        policy: SupervisorPolicy,
    ) -> Result<Self, SupervisorError> {
        if policy.crash_limit == 0 || policy.crash_window.is_zero() {
            return Err(SupervisorError::InvalidPolicy);
        }
        Ok(Self {
            store,
            data_root: data_root.as_ref().to_path_buf(),
            runtime_dir: runtime_dir.as_ref().to_path_buf(),
            wayland_display: wayland_display.into(),
            policy,
            apps: HashMap::new(),
        })
    }

    pub fn launch(
        &mut self,
        app_id: &str,
        granted: &[Capability],
    ) -> Result<LaunchOutcome, SupervisorError> {
        let installed = self
            .store
            .load(app_id)?
            .ok_or_else(|| SupervisorError::NotInstalled(app_id.to_owned()))?;

        let mut runtime = self
            .apps
            .remove(app_id)
            .unwrap_or_else(|| AppRuntime::new(installed.clone()));
        runtime.installed = installed;
        if let Err(error) = self.reap_without_restart(app_id, &mut runtime) {
            self.apps.insert(app_id.to_owned(), runtime);
            return Err(error);
        }

        if runtime.installed.manifest.single_instance {
            if let Some(child) = runtime.children.first() {
                let pid = child.id();
                self.apps.insert(app_id.to_owned(), runtime);
                return Ok(LaunchOutcome::Existing { pid });
            }
        }

        if runtime.children.is_empty() {
            runtime.failures.clear();
            runtime.crash_limited = false;
        }
        runtime.granted = granted.to_vec();
        let child = self.spawn(app_id, &runtime.installed, &runtime.granted)?;
        let pid = child.id();
        runtime.children.push(child);
        self.apps.insert(app_id.to_owned(), runtime);
        Ok(LaunchOutcome::Started { pid })
    }

    pub fn stop(&mut self, app_id: &str) -> Result<bool, SupervisorError> {
        let Some(mut runtime) = self.apps.remove(app_id) else {
            return Ok(false);
        };
        let had_children = !runtime.children.is_empty();
        if let Err(error) = stop_children(app_id, &mut runtime.children) {
            self.apps.insert(app_id.to_owned(), runtime);
            return Err(error);
        }
        runtime.crash_limited = false;
        runtime.failures.clear();
        self.apps.insert(app_id.to_owned(), runtime);
        Ok(had_children)
    }

    pub fn state(&self, app_id: &str) -> Option<AppState> {
        self.apps.get(app_id).map(|runtime| {
            if runtime.crash_limited {
                AppState::CrashLimited
            } else if runtime.children.is_empty() {
                AppState::Stopped
            } else {
                AppState::Running
            }
        })
    }

    pub fn pids(&self, app_id: &str) -> Vec<u32> {
        self.apps
            .get(app_id)
            .map(|runtime| runtime.children.iter().map(Child::id).collect())
            .unwrap_or_default()
    }

    pub fn poll(&mut self) -> Result<Vec<AppEvent>, SupervisorError> {
        let mut app_ids: Vec<_> = self.apps.keys().cloned().collect();
        app_ids.sort_unstable();
        let mut events = Vec::new();

        for app_id in app_ids {
            let Some(mut runtime) = self.apps.remove(&app_id) else {
                continue;
            };
            let was_crash_limited = runtime.crash_limited;
            let mut restart_count = 0;
            let mut index = 0;
            while index < runtime.children.len() {
                let status = match runtime.children[index].try_wait() {
                    Ok(status) => status,
                    Err(source) => {
                        let error = SupervisorError::Process {
                            operation: "inspect",
                            app_id: app_id.clone(),
                            source,
                        };
                        self.apps.insert(app_id.clone(), runtime);
                        return Err(error);
                    }
                };
                let Some(_) = status else {
                    index += 1;
                    continue;
                };
                let mut child = runtime.children.swap_remove(index);
                let pid = child.id();
                let status = match child.wait() {
                    Ok(status) => status,
                    Err(source) => {
                        let error = SupervisorError::Process {
                            operation: "reap",
                            app_id: app_id.clone(),
                            source,
                        };
                        self.apps.insert(app_id.clone(), runtime);
                        return Err(error);
                    }
                };
                if status.success() {
                    events.push(exit_event(&app_id, AppEventKind::Stopped, pid, status));
                    continue;
                }

                let failures = runtime.record_failure(Instant::now(), self.policy);
                events.push(exit_event(&app_id, AppEventKind::Crashed, pid, status));
                if failures >= self.policy.crash_limit {
                    runtime.crash_limited = true;
                } else {
                    restart_count += 1;
                }
            }

            if runtime.crash_limited {
                if let Err(error) = stop_children(&app_id, &mut runtime.children) {
                    self.apps.insert(app_id.clone(), runtime);
                    return Err(error);
                }
                if !was_crash_limited {
                    events.push(AppEvent {
                        app_id: app_id.clone(),
                        kind: AppEventKind::CrashLimited,
                        pid: None,
                        exit_code: None,
                    });
                }
            } else {
                for _ in 0..restart_count {
                    let child = match self.spawn(&app_id, &runtime.installed, &runtime.granted) {
                        Ok(child) => child,
                        Err(error) => {
                            self.apps.insert(app_id.clone(), runtime);
                            return Err(error);
                        }
                    };
                    let pid = child.id();
                    runtime.children.push(child);
                    events.push(AppEvent {
                        app_id: app_id.clone(),
                        kind: AppEventKind::Running,
                        pid: Some(pid),
                        exit_code: None,
                    });
                }
            }
            self.apps.insert(app_id, runtime);
        }
        Ok(events)
    }

    fn reap_without_restart(
        &self,
        app_id: &str,
        runtime: &mut AppRuntime,
    ) -> Result<(), SupervisorError> {
        let mut index = 0;
        while index < runtime.children.len() {
            match runtime.children[index].try_wait() {
                Ok(Some(_)) => {
                    let mut child = runtime.children.swap_remove(index);
                    child.wait().map_err(|source| SupervisorError::Process {
                        operation: "reap",
                        app_id: app_id.to_owned(),
                        source,
                    })?;
                }
                Ok(None) => index += 1,
                Err(source) => {
                    return Err(SupervisorError::Process {
                        operation: "inspect",
                        app_id: app_id.to_owned(),
                        source,
                    });
                }
            }
        }
        Ok(())
    }

    fn spawn(
        &self,
        app_id: &str,
        installed: &InstalledApp,
        granted: &[Capability],
    ) -> Result<Child, SupervisorError> {
        let executable = installed.code_dir.join(&installed.manifest.exec);
        let sandbox_paths = SandboxPaths {
            apps_dir: self.store.apps_dir().to_path_buf(),
            code_dir: installed.code_dir.clone(),
            apps_data_dir: self.store.data_dir().to_path_buf(),
            data_dir: installed.data_dir.clone(),
            entities_dir: self.data_root.join("var").join("entities"),
        };
        let granted = granted.to_vec();
        let mut command = Command::new(executable);
        command
            .current_dir(&installed.code_dir)
            .env_clear()
            .env("SAAIOS_APP_ID", app_id)
            .env("SAAIOS_DATA_DIR", &installed.data_dir)
            .env("XDG_RUNTIME_DIR", &self.runtime_dir)
            .env("WAYLAND_DISPLAY", &self.wayland_display)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        // Safety: this closure runs after fork(), before execve(), in the
        // still-single-threaded child -- the only place unshare()/mount()
        // can actually put the exec'd process itself into new namespaces
        // (ADR-020). It only calls into libc/nix syscalls and does not
        // rely on any state shared with the parent process.
        unsafe {
            command.pre_exec(move || {
                sandbox::apply(&sandbox_paths, &granted).map_err(|error| {
                    eprintln!("saai-appd: sandbox setup failed: {error}");
                    error
                })
            });
        }
        command.spawn().map_err(|source| SupervisorError::Process {
            operation: "launch",
            app_id: app_id.to_owned(),
            source,
        })
    }
}

impl Drop for AppSupervisor {
    fn drop(&mut self) {
        for runtime in self.apps.values_mut() {
            for child in &mut runtime.children {
                let _ = child.kill();
                let _ = child.wait();
            }
            runtime.children.clear();
        }
    }
}

fn stop_children(app_id: &str, children: &mut Vec<Child>) -> Result<(), SupervisorError> {
    for child in children.iter_mut() {
        if let Err(source) = child.kill() {
            if source.kind() != io::ErrorKind::InvalidInput {
                return Err(SupervisorError::Process {
                    operation: "stop",
                    app_id: app_id.to_owned(),
                    source,
                });
            }
        }
        child.wait().map_err(|source| SupervisorError::Process {
            operation: "reap stopped process",
            app_id: app_id.to_owned(),
            source,
        })?;
    }
    children.clear();
    Ok(())
}

fn exit_event(app_id: &str, kind: AppEventKind, pid: u32, status: ExitStatus) -> AppEvent {
    AppEvent {
        app_id: app_id.to_owned(),
        kind,
        pid: Some(pid),
        exit_code: status.code(),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::thread;
    use std::time::{Duration, Instant};

    use tempfile::TempDir;

    use super::{AppEventKind, AppState, AppSupervisor, LaunchOutcome, SupervisorPolicy};
    use crate::AppStore;

    const APP_ID: &str = "org.saaios.example.lifecycle";

    struct Fixture {
        _temporary: TempDir,
        store: AppStore,
        runtime_dir: PathBuf,
    }

    impl Fixture {
        fn new(script: &str, single_instance: bool) -> Self {
            let temporary = TempDir::new().unwrap();
            let source = temporary.path().join("package");
            fs::create_dir_all(source.join("bin")).unwrap();
            fs::write(
                source.join("manifest.toml"),
                format!(
                    "schema = 1\nid = \"{APP_ID}\"\nname = \"Lifecycle\"\nexec = \"bin/app\"\nversion = \"0.1.0\"\nui = \"wayland\"\nsingle_instance = {single_instance}\ncapabilities = []\n"
                ),
            )
            .unwrap();
            let executable = source.join("bin/app");
            fs::write(&executable, script).unwrap();
            let mut permissions = fs::metadata(&executable).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&executable, permissions).unwrap();

            let root = temporary.path().join("saaios");
            let runtime_dir = temporary.path().join("runtime");
            fs::create_dir(&runtime_dir).unwrap();
            let store = AppStore::new(&root);
            store.install(source).unwrap();
            Self {
                _temporary: temporary,
                store,
                runtime_dir,
            }
        }

        fn supervisor(&self) -> AppSupervisor {
            let data_root = self.store.apps_dir().parent().unwrap();
            AppSupervisor::with_policy(
                self.store.clone(),
                data_root,
                &self.runtime_dir,
                "wayland-test",
                SupervisorPolicy {
                    crash_limit: 3,
                    crash_window: Duration::from_secs(2),
                },
            )
            .unwrap()
        }
    }

    #[test]
    fn single_instance_returns_existing_pid() {
        let fixture = Fixture::new("#!/bin/sh\nsleep 30\n", true);
        let mut supervisor = fixture.supervisor();

        let LaunchOutcome::Started { pid } = supervisor.launch(APP_ID, &[]).unwrap() else {
            panic!("first launch must start the app");
        };
        assert_eq!(
            supervisor.launch(APP_ID, &[]).unwrap(),
            LaunchOutcome::Existing { pid }
        );
        assert_eq!(supervisor.pids(APP_ID), [pid]);
        assert_eq!(supervisor.state(APP_ID), Some(AppState::Running));
        assert!(supervisor.stop(APP_ID).unwrap());
        assert_eq!(supervisor.state(APP_ID), Some(AppState::Stopped));
    }

    #[test]
    fn multi_instance_manifest_starts_distinct_processes() {
        let fixture = Fixture::new("#!/bin/sh\nsleep 30\n", false);
        let mut supervisor = fixture.supervisor();

        let LaunchOutcome::Started { pid: first } = supervisor.launch(APP_ID, &[]).unwrap() else {
            panic!("first launch must start the app");
        };
        let LaunchOutcome::Started { pid: second } = supervisor.launch(APP_ID, &[]).unwrap() else {
            panic!("second launch must start another instance");
        };
        assert_ne!(first, second);
        assert_eq!(supervisor.pids(APP_ID).len(), 2);
        supervisor.stop(APP_ID).unwrap();
    }

    #[test]
    fn child_receives_only_explicit_runtime_context() {
        let fixture = Fixture::new(
            "#!/bin/sh\nprintf '%s\\n%s\\n%s\\n%s\\n%s\\n' \"$SAAIOS_APP_ID\" \"$SAAIOS_DATA_DIR\" \"$XDG_RUNTIME_DIR\" \"$WAYLAND_DISPLAY\" \"${SHOULD_NOT_LEAK-unset}\" >\"$SAAIOS_DATA_DIR/context\"\n",
            true,
        );
        let mut supervisor = fixture.supervisor();
        std::env::set_var("SHOULD_NOT_LEAK", "secret");

        supervisor.launch(APP_ID, &[]).unwrap();
        let context = wait_for_file(&fixture.store.data_dir().join(APP_ID).join("context"));
        std::env::remove_var("SHOULD_NOT_LEAK");

        let lines: Vec<_> = context.lines().collect();
        assert_eq!(lines[0], APP_ID);
        assert_eq!(
            lines[1],
            fixture.store.data_dir().join(APP_ID).to_str().unwrap()
        );
        assert_eq!(lines[2], fixture.runtime_dir.to_str().unwrap());
        assert_eq!(lines[3], "wayland-test");
        assert_eq!(lines[4], "unset");
    }

    #[test]
    fn three_crashes_in_window_limit_restart_until_explicit_launch() {
        let fixture = Fixture::new("#!/bin/sh\nexit 17\n", true);
        let mut supervisor = fixture.supervisor();
        supervisor.launch(APP_ID, &[]).unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut crashes = 0;
        let mut limited = false;

        while Instant::now() < deadline && !limited {
            thread::sleep(Duration::from_millis(20));
            for event in supervisor.poll().unwrap() {
                if event.kind == AppEventKind::Crashed {
                    crashes += 1;
                    assert_eq!(event.exit_code, Some(17));
                } else if event.kind == AppEventKind::CrashLimited {
                    limited = true;
                }
            }
        }

        assert_eq!(crashes, 3);
        assert!(limited);
        assert_eq!(supervisor.state(APP_ID), Some(AppState::CrashLimited));
        assert!(supervisor.pids(APP_ID).is_empty());
        assert!(supervisor.poll().unwrap().is_empty());
        assert!(matches!(
            supervisor.launch(APP_ID, &[]).unwrap(),
            LaunchOutcome::Started { .. }
        ));
        assert_eq!(supervisor.state(APP_ID), Some(AppState::Running));
    }

    #[test]
    fn explicit_stop_is_not_counted_as_a_crash() {
        let fixture = Fixture::new("#!/bin/sh\nsleep 30\n", true);
        let mut supervisor = fixture.supervisor();
        supervisor.launch(APP_ID, &[]).unwrap();

        assert!(supervisor.stop(APP_ID).unwrap());
        assert!(supervisor.poll().unwrap().is_empty());
        assert_eq!(supervisor.state(APP_ID), Some(AppState::Stopped));
        assert!(matches!(
            supervisor.launch(APP_ID, &[]).unwrap(),
            LaunchOutcome::Started { .. }
        ));
    }

    fn wait_for_file(path: &Path) -> String {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if let Ok(contents) = fs::read_to_string(path) {
                return contents;
            }
            assert!(Instant::now() < deadline, "file was not created: {path:?}");
            thread::sleep(Duration::from_millis(10));
        }
    }
}
