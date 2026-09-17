use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::io::Write as _;
use std::os::unix::process::CommandExt;
use std::os::unix::process::ExitStatusExt as _;
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
    /// ADR-097: this app's own private session-D-Bus daemon, if one has
    /// been started -- never in `children` on purpose (crash/restart/
    /// pid-reporting logic there is about the app process itself; this
    /// is supervisor-owned plumbing, not something `list`'s pids or the
    /// crash-limiter should ever see).
    dbus_daemon: Option<Child>,
}

impl AppRuntime {
    fn new(installed: InstalledApp) -> Self {
        Self {
            installed,
            children: Vec::new(),
            failures: VecDeque::new(),
            crash_limited: false,
            granted: Vec::new(),
            dbus_daemon: None,
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
    /// Shared persistent root replaced by the sandbox's default-deny view.
    data_root: PathBuf,
    runtime_dir: PathBuf,
    wayland_display: String,
    allow_unsandboxed: bool,
    policy: SupervisorPolicy,
    apps: HashMap<String, AppRuntime>,
    /// ADR-097: where the (optional, prebuilt Alpine) session `dbus-daemon`
    /// binary and its `lib/`/`share/dbus-1/session.conf` live. Defaulted,
    /// not required -- `ensure_dbus_daemon` degrades to "no bus for this
    /// launch" when nothing is deployed at this path (host tests, an
    /// image that hasn't staged it), the same tolerance `bundled_fonts`
    /// already has for a per-app resource that is not always present.
    dbus_binary_dir: PathBuf,
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
            allow_unsandboxed: false,
            policy: SupervisorPolicy::default(),
            apps: HashMap::new(),
            dbus_binary_dir: PathBuf::from("/data/saaios/system/dbus"),
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
            allow_unsandboxed: false,
            policy,
            apps: HashMap::new(),
            dbus_binary_dir: PathBuf::from("/data/saaios/system/dbus"),
        })
    }

    /// Enables the non-root bypass used by host process tests. Production
    /// callers must keep the secure default (`false`).
    pub fn with_unsandboxed_host_fallback(mut self, enabled: bool) -> Self {
        self.allow_unsandboxed = enabled;
        self
    }

    /// ADR-097: overrides where the session `dbus-daemon` binary/libs/
    /// config live. Production's default (`/data/saaios/system/dbus`)
    /// covers the real image; this exists for tests and for a build that
    /// stages the binary somewhere else.
    pub fn with_dbus_binary_dir(mut self, dir: impl AsRef<Path>) -> Self {
        self.dbus_binary_dir = dir.as_ref().to_path_buf();
        self
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
        let dbus_address = self.ensure_dbus_daemon(&mut runtime, app_id)?;
        let child = self.spawn(app_id, &runtime.installed, &runtime.granted, &dbus_address)?;
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
        stop_dbus_daemon(&mut runtime.dbus_daemon);
        let _ = fs::remove_file(self.dbus_socket_path(app_id));
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
                    let dbus_address = match self.ensure_dbus_daemon(&mut runtime, &app_id) {
                        Ok(address) => address,
                        Err(error) => {
                            self.apps.insert(app_id.clone(), runtime);
                            return Err(error);
                        }
                    };
                    let child = match self.spawn(
                        &app_id,
                        &runtime.installed,
                        &runtime.granted,
                        &dbus_address,
                    ) {
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

    /// ADR-097: deterministic per-app path -- no bookkeeping needed to
    /// find an app's own bus back later (`stop()` derives the same path
    /// to clean up), and distinct apps never collide.
    fn dbus_socket_path(&self, app_id: &str) -> PathBuf {
        PathBuf::from(format!("/run/saaios/dbus/{app_id}.sock"))
    }

    /// ADR-097: starts this app's own private session `dbus-daemon` if
    /// one is not already running, and returns its socket path either
    /// way. Never shared across apps -- a fresh daemon per app_id, torn
    /// down in `stop()`, is what keeps this from becoming a real
    /// inter-app channel (see `SandboxPaths::dbus_socket`'s doc comment).
    ///
    /// Tolerant, not fail-closed: if `dbus_binary_dir` has nothing staged
    /// (host tests, an image that hasn't shipped it yet), this returns
    /// the socket path anyway with no daemon behind it -- an app that
    /// hard-requires D-Bus fails exactly as it already did before this
    /// change, every other app is unaffected.
    fn ensure_dbus_daemon(
        &self,
        runtime: &mut AppRuntime,
        app_id: &str,
    ) -> Result<PathBuf, SupervisorError> {
        let socket_path = self.dbus_socket_path(app_id);
        let alive = runtime
            .dbus_daemon
            .as_mut()
            .is_some_and(|child| matches!(child.try_wait(), Ok(None)));
        if alive {
            return Ok(socket_path);
        }
        runtime.dbus_daemon = None;

        let binary = self.dbus_binary_dir.join("bin/dbus-daemon");
        if !binary.is_file() {
            return Ok(socket_path);
        }

        if let Some(parent) = socket_path.parent() {
            fs::create_dir_all(parent).map_err(|source| SupervisorError::Process {
                operation: "create dbus runtime dir",
                app_id: app_id.to_owned(),
                source,
            })?;
        }
        let _ = fs::remove_file(&socket_path);

        let config_file = self.dbus_binary_dir.join("share/dbus-1/session.conf");
        let lib_dir = self.dbus_binary_dir.join("lib");
        let child = Command::new(&binary)
            .env_clear()
            .env("LD_LIBRARY_PATH", &lib_dir)
            .arg("--nofork")
            .arg(format!("--config-file={}", config_file.display()))
            .arg(format!("--address=unix:path={}", socket_path.display()))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|source| SupervisorError::Process {
                operation: "start session dbus",
                app_id: app_id.to_owned(),
                source,
            })?;
        runtime.dbus_daemon = Some(child);

        // `--nofork` creates its listening socket essentially immediately
        // (no chroot/heavy init -- ADR-097's manual spike measured well
        // under this) -- a short bounded poll keeps the common case fast
        // and still fails safe (an unrevealed socket, same as the
        // tolerant path above) if it never appears.
        let deadline = Instant::now() + Duration::from_millis(500);
        while !socket_path.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(socket_path)
    }

    fn spawn(
        &self,
        app_id: &str,
        installed: &InstalledApp,
        granted: &[Capability],
        dbus_address: &Path,
    ) -> Result<Child, SupervisorError> {
        let executable = installed.code_dir.join(&installed.manifest.exec);
        // S08 Change 6: mirrors the FONTCONFIG_PATH convention below --
        // an app that bundles its own dynamically-linked runtime (GTK4/Qt
        // via Alpine's musl packages, ADR-021) ships the loader and every
        // shared library it needs under its own `<code_dir>/lib`; the
        // sandbox reveals it at `/lib` only for that one app (see
        // `SandboxPaths::lib_dir`'s doc comment for why this device needs
        // it at all).
        let bundled_lib = installed.code_dir.join("lib");
        let lib_dir = bundled_lib.is_dir().then_some(bundled_lib);
        let sandbox_paths = SandboxPaths {
            data_root: self.data_root.clone(),
            code_dir: installed.code_dir.clone(),
            data_dir: installed.data_dir.clone(),
            wayland_socket: self.runtime_dir.join(&self.wayland_display),
            portal_socket: PathBuf::from("/run/saaios/portal.sock"),
            dbus_socket: dbus_address.to_path_buf(),
            lib_dir,
        };
        let granted = granted.to_vec();
        let allow_unsandboxed = self.allow_unsandboxed;
        let mut command = Command::new(executable);
        command
            .current_dir(&installed.code_dir)
            .env_clear()
            .env("SAAIOS_APP_ID", app_id)
            .env("SAAIOS_DATA_DIR", &installed.data_dir)
            .env("XDG_RUNTIME_DIR", &self.runtime_dir)
            .env("WAYLAND_DISPLAY", &self.wayland_display)
            // S08 Change 4: mechanism-level env vars GTK4/Qt already read on
            // their own -- not a settings VALUE (SaaiOS has no real
            // settings/theme source yet; that is S09/S10's job, not
            // appd's), just plumbing that keeps a sandboxed, D-Bus-less
            // launch from wasting startup time on lookups that can never
            // succeed here. Confirmed harmless and unnecessary for
            // correctness (a real Alpine-built gtk4-demo already opens a
            // display and commits a frame without them -- see the S08
            // sprint doc's Change 4 Evidence) -- set anyway because they
            // are cheap and remove real log noise (failed a11y-bus/XCB-
            // detection attempts) a real device run would otherwise carry
            // on every launch.
            .env("QT_QPA_PLATFORM", "wayland")
            .env("GTK_A11Y", "none")
            .env("NO_AT_BRIDGE", "1")
            // ADR-097: set unconditionally, like the vars above -- a
            // toolkit that never looks at D-Bus ignores this exactly as
            // it ignores QT_QPA_PLATFORM on GTK. When no daemon could be
            // started (dbus_socket_path with nothing listening on it),
            // this points at a dead socket, which is exactly the launch
            // behavior an app hard-requiring D-Bus already had before
            // this change -- no new failure mode, just a real fix when
            // the binary is deployed.
            .env(
                "DBUS_SESSION_BUS_ADDRESS",
                format!("unix:path={}", dbus_address.display()),
            )
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        // ADR-021: each app ships its own complete runtime bundle, fonts
        // included, under its own code_dir -- there is no shared system
        // fontconfig location on this device to fall back to instead. Only
        // set when the app actually bundled one; fontconfig's own built-in
        // defaults (and toolkits' tolerance for missing fonts) cover an
        // app that didn't.
        let bundled_fonts = installed.code_dir.join("etc").join("fonts");
        if bundled_fonts.is_dir() {
            command.env("FONTCONFIG_PATH", &bundled_fonts);
        }
        // S08 Change 6: explicit, not relied-on-by-default -- the loader
        // bind-mounted at `/lib` (see `SandboxPaths::lib_dir`) already
        // gets checked by musl's own compiled-in default search path,
        // but setting LD_LIBRARY_PATH directly does not depend on that
        // assumption holding across Alpine musl builds.
        if sandbox_paths.lib_dir.is_some() {
            command.env("LD_LIBRARY_PATH", "/lib");
        }
        // Safety: this closure runs after fork(), before execve(), in the
        // still-single-threaded child -- the only place unshare()/mount()
        // can actually put the exec'd process itself into new namespaces
        // (ADR-020). It only calls into libc/nix syscalls and does not
        // rely on any state shared with the parent process.
        unsafe {
            command.pre_exec(move || {
                sandbox::apply(&sandbox_paths, &granted, allow_unsandboxed).map_err(|error| {
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
            stop_dbus_daemon(&mut runtime.dbus_daemon);
        }
    }
}

/// ADR-097: shared by `stop()` and `Drop` -- kills and reaps this app's
/// private session-D-Bus daemon, if it has one. Leaves the socket file
/// itself for the caller (`stop()` removes it explicitly; `Drop` runs at
/// process exit, where the whole `/run/saaios/dbus` tree either outlives
/// nothing meaningful or is cleaned up by the next boot regardless).
fn stop_dbus_daemon(dbus_daemon: &mut Option<Child>) {
    if let Some(mut child) = dbus_daemon.take() {
        let _ = child.kill();
        let _ = child.wait();
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
    if kind == AppEventKind::Crashed {
        log_crash_diagnostic(app_id, pid, &status);
    }
    AppEvent {
        app_id: app_id.to_owned(),
        kind,
        pid: Some(pid),
        exit_code: status.code(),
    }
}

/// S28 diagnostic only: `status.code()` collapses a signal kill down to
/// `None`, which is exactly the case under investigation (a sandboxed
/// app that dies with no panic message reaching anywhere, since its
/// own stdout/stderr are discarded per ADR-020). Appended, not
/// overwritten, since a relaunch-and-tap repro cycle is the whole
/// point.
fn log_crash_diagnostic(app_id: &str, pid: u32, status: &ExitStatus) {
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/data/saaios/var/appd-crashes.log")
    {
        let _ = writeln!(
            file,
            "app_id={app_id} pid={pid} exit_code={:?} signal={:?} core_dumped={}",
            status.code(),
            status.signal(),
            status.core_dumped(),
        );
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
            Self::build(script, single_instance, false)
        }

        /// Same as `new`, but the installed package also ships its own
        /// `etc/fonts` directory -- for exercising the S08 Change 4
        /// conditional `FONTCONFIG_PATH` injection.
        fn with_bundled_fonts(script: &str, single_instance: bool) -> Self {
            Self::build(script, single_instance, true)
        }

        fn build(script: &str, single_instance: bool, bundle_fonts: bool) -> Self {
            let temporary = TempDir::new().unwrap();
            let source = temporary.path().join("package");
            fs::create_dir_all(source.join("bin")).unwrap();
            if bundle_fonts {
                fs::create_dir_all(source.join("etc").join("fonts")).unwrap();
            }
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
            .with_unsandboxed_host_fallback(true)
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
            "#!/bin/sh\nprintf '%s\\n%s\\n%s\\n%s\\n%s\\n%s\\n%s\\n%s\\n%s\\n' \"$SAAIOS_APP_ID\" \"$SAAIOS_DATA_DIR\" \"$XDG_RUNTIME_DIR\" \"$WAYLAND_DISPLAY\" \"${SHOULD_NOT_LEAK-unset}\" \"$QT_QPA_PLATFORM\" \"$GTK_A11Y\" \"$NO_AT_BRIDGE\" \"${FONTCONFIG_PATH-unset}\" >\"$SAAIOS_DATA_DIR/context\"\n",
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
        // S08 Change 4: mechanism-level toolkit env vars, always set --
        // and FONTCONFIG_PATH, which must stay unset when the app (like
        // this fixture) never bundled its own etc/fonts.
        assert_eq!(lines[5], "wayland");
        assert_eq!(lines[6], "none");
        assert_eq!(lines[7], "1");
        assert_eq!(lines[8], "unset");
    }

    #[test]
    fn fontconfig_path_set_only_when_app_bundles_its_own_fonts() {
        let fixture = Fixture::with_bundled_fonts(
            "#!/bin/sh\nprintf '%s\\n' \"${FONTCONFIG_PATH-unset}\" >\"$SAAIOS_DATA_DIR/fontconfig\"\n",
            true,
        );
        let mut supervisor = fixture.supervisor();

        supervisor.launch(APP_ID, &[]).unwrap();
        let value = wait_for_file(&fixture.store.data_dir().join(APP_ID).join("fontconfig"));

        let expected = fixture
            .store
            .apps_dir()
            .join(APP_ID)
            .join("etc")
            .join("fonts");
        assert_eq!(value.trim(), expected.to_str().unwrap());
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
