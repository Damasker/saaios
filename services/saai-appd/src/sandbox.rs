use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use nix::mount::{mount, MsFlags};
use nix::sched::{unshare, CloneFlags};
use nix::unistd::Uid;
use seccompiler::{BpfProgram, SeccompAction, SeccompFilter, TargetArch};

use crate::Capability;

/// The paths one sandboxed app process needs mediated (ADR-020 section 4):
/// the two shared parent directories every installed app lives under get
/// masked with an empty tmpfs, and only this app's own code/data
/// subdirectories are bound back through the mask. The raw entity store
/// is masked with no reveal at all -- `saai-entityd`'s socket is the only
/// sanctioned path to that data (S06).
pub struct SandboxPaths {
    pub apps_dir: PathBuf,
    pub code_dir: PathBuf,
    pub apps_data_dir: PathBuf,
    pub data_dir: PathBuf,
    pub entities_dir: PathBuf,
}

/// Installs ADR-020's process isolation for one app. Must run inside
/// `std::process::Command::pre_exec` -- after `fork()`, before
/// `execve()`, in the (single-threaded) child -- because
/// `unshare(CLONE_NEWNS | ...)` moves the *calling* thread into the new
/// namespaces (unlike `CLONE_NEWPID`/`CLONE_NEWUSER`, which this kernel
/// does not support at all -- confirmed by an on-device spike, see
/// ADR-020's Контекст).
///
/// Production is fail-closed: an unprivileged daemon cannot construct the
/// required namespaces and must refuse the launch. `allow_unsandboxed` exists
/// only for explicit host integration tests whose temporary shell scripts run
/// under an ordinary developer account; PID 1 never passes that override.
pub fn apply(
    paths: &SandboxPaths,
    granted: &[Capability],
    allow_unsandboxed: bool,
) -> io::Result<()> {
    if !Uid::effective().is_root() {
        return if allow_unsandboxed {
            eprintln!("saai-appd: sandbox explicitly bypassed for host test");
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "sandbox requires root namespace and mount privileges",
            ))
        };
    }

    let net_internet = granted.contains(&Capability::NetInternet);
    let mut flags = CloneFlags::CLONE_NEWNS | CloneFlags::CLONE_NEWIPC | CloneFlags::CLONE_NEWUTS;
    if !net_internet {
        flags |= CloneFlags::CLONE_NEWNET;
    }
    unshare(flags).map_err(nix_to_io)?;

    // Detach this process's mount tree from the host's before touching
    // anything else -- without this, mount namespaces created by
    // unshare() default to propagating events back to the namespace they
    // were cloned from, so the masks below would leak onto every other
    // process on the system instead of staying local to this one.
    mount(
        None::<&str>,
        "/",
        None::<&str>,
        MsFlags::MS_REC | MsFlags::MS_PRIVATE,
        None::<&str>,
    )
    .map_err(nix_to_io)?;

    // Scratch mountpoints live under /run/saaios, which nothing here ever
    // masks, keyed by this (freshly forked, uniquely numbered) process's
    // own pid so concurrent launches of different apps -- or successive
    // crash-restarts of the same one -- never collide.
    let scratch_root =
        PathBuf::from("/run/saaios/.sandbox-reveal").join(std::process::id().to_string());
    mask_and_reveal(
        &paths.apps_dir,
        &paths.code_dir,
        &scratch_root.join("code"),
        true,
    )?;
    mask_and_reveal(
        &paths.apps_data_dir,
        &paths.data_dir,
        &scratch_root.join("data"),
        false,
    )?;
    mask_only(&paths.entities_dir)?;
    // Both leaf scratch dirs (code/data) are already gone -- this only
    // removes the now-empty per-pid parent they shared.
    let _ = fs::remove_dir(&scratch_root);

    install_seccomp_filter()?;

    Ok(())
}

/// Masks `parent` with an empty tmpfs, hiding every sibling of `own`,
/// while keeping `own`'s original content reachable at that exact path.
///
/// The naive version of this -- bind-mount `own` onto itself, *then* mask
/// `parent` -- does not work: masking `parent` re-resolves every path
/// underneath it from scratch, so `own` immediately starts pointing into
/// the fresh, empty tmpfs rather than the pre-mask content it used to
/// point to (confirmed with a throwaway on-device C spike before writing
/// this the working way). The original content has to be pinned
/// *outside* `parent` first (bind-mount to `scratch`, unaffected by
/// anything that happens to `parent`), then moved back into place with
/// `MS_MOVE` once the mask is already on and `own`'s directory exists
/// again inside the new tmpfs.
fn mask_and_reveal(parent: &Path, own: &Path, scratch: &Path, read_only: bool) -> io::Result<()> {
    fs::create_dir_all(scratch)?;
    mount(
        Some(own),
        scratch,
        None::<&str>,
        MsFlags::MS_BIND,
        None::<&str>,
    )
    .map_err(nix_to_io)?;

    mount(
        None::<&str>,
        parent,
        Some("tmpfs"),
        MsFlags::empty(),
        None::<&str>,
    )
    .map_err(nix_to_io)?;

    fs::create_dir_all(own)?;
    mount(
        Some(scratch),
        own,
        None::<&str>,
        MsFlags::MS_MOVE,
        None::<&str>,
    )
    .map_err(nix_to_io)?;
    let _ = fs::remove_dir(scratch);

    if read_only {
        mount(
            None::<&str>,
            own,
            None::<&str>,
            MsFlags::MS_BIND | MsFlags::MS_REMOUNT | MsFlags::MS_RDONLY,
            None::<&str>,
        )
        .map_err(nix_to_io)?;
    }
    Ok(())
}

/// Masks `path` with an empty tmpfs and reveals nothing underneath it --
/// used for the raw entity store, which has no sanctioned direct-file
/// access path at all (S06's `saai-entityd` socket is the only door).
fn mask_only(path: &Path) -> io::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    mount(
        None::<&str>,
        path,
        Some("tmpfs"),
        MsFlags::empty(),
        None::<&str>,
    )
    .map_err(nix_to_io)
}

/// Deny-list compensating for the missing PID/user namespace isolation
/// (ADR-020's Известные ограничения): `ptrace`/`kill`/`tkill`/`tgkill`
/// close off interfering with another process now that this one can
/// still see the whole system's `/proc`. `mount`/`umount2`/`pivot_root`/
/// `chroot` stop the mount setup above from being undone or bypassed
/// from inside. `unshare`/`setns` stop leaving the assigned namespaces.
/// `reboot`/`init_module`/`delete_module`/`kexec_load`/`personality`
/// have no legitimate use from an application process.
///
/// Known gap, not hidden: `clone` itself is intentionally left alone --
/// blanket-denying it would break ordinary thread creation (every
/// runtime this platform hosts, Rust's own std included, spawns threads
/// via `clone`), and safely restricting only its `CLONE_NEW*`-flagged
/// uses needs argument-based BPF filtering this version does not
/// attempt. `unshare`/`setns` alone already close the two simplest
/// namespace-escape paths.
const DENIED_SYSCALLS: &[i64] = &[
    libc::SYS_ptrace,
    libc::SYS_kill,
    libc::SYS_tkill,
    libc::SYS_tgkill,
    libc::SYS_mount,
    libc::SYS_umount2,
    libc::SYS_pivot_root,
    libc::SYS_chroot,
    libc::SYS_unshare,
    libc::SYS_setns,
    libc::SYS_reboot,
    libc::SYS_init_module,
    libc::SYS_delete_module,
    libc::SYS_kexec_load,
    libc::SYS_personality,
];

fn install_seccomp_filter() -> io::Result<()> {
    let mut rules = BTreeMap::new();
    for syscall in DENIED_SYSCALLS {
        rules.insert(*syscall, Vec::new());
    }
    let filter = SeccompFilter::new(
        rules,
        SeccompAction::Allow,
        SeccompAction::Errno(libc::EPERM as u32),
        target_arch(),
    )
    .map_err(|error| io::Error::other(error.to_string()))?;
    let program: BpfProgram = filter
        .try_into()
        .map_err(|error: seccompiler::BackendError| io::Error::other(error.to_string()))?;
    seccompiler::apply_filter(&program).map_err(|error| io::Error::other(error.to_string()))
}

#[cfg(target_arch = "aarch64")]
fn target_arch() -> TargetArch {
    TargetArch::aarch64
}

#[cfg(target_arch = "x86_64")]
fn target_arch() -> TargetArch {
    TargetArch::x86_64
}

fn nix_to_io(error: nix::Error) -> io::Error {
    io::Error::from_raw_os_error(error as i32)
}

#[cfg(test)]
mod tests {
    use super::apply;
    use crate::Capability;
    use std::path::PathBuf;

    /// The non-root fallback is what every host/CI environment actually
    /// exercises (R620's own test suite runs unprivileged) -- this is the
    /// one behavior of `apply()` this crate's tests can check directly;
    /// the real mount/namespace behavior requires root and is verified
    /// physically on-device instead (see the S07 sprint doc's Evidence).
    #[test]
    fn non_root_is_fail_closed_unless_test_override_is_explicit() {
        let paths = super::SandboxPaths {
            apps_dir: PathBuf::from("/nonexistent/apps"),
            code_dir: PathBuf::from("/nonexistent/apps/org.saaios.example"),
            apps_data_dir: PathBuf::from("/nonexistent/var/apps"),
            data_dir: PathBuf::from("/nonexistent/var/apps/org.saaios.example"),
            entities_dir: PathBuf::from("/nonexistent/var/entities"),
        };
        let error = apply(&paths, &[Capability::NetInternet], false).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(apply(&paths, &[Capability::NetInternet], true).is_ok());
    }
}
