use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use nix::mount::{mount, MsFlags};
use nix::sched::{unshare, CloneFlags};
use nix::unistd::Uid;
use seccompiler::{BpfProgram, SeccompAction, SeccompFilter, TargetArch};

use crate::Capability;

/// The complete filesystem view explicitly revealed to one application.
/// Everything else below the shared SaaiOS data and runtime roots is hidden.
pub struct SandboxPaths {
    pub data_root: PathBuf,
    pub code_dir: PathBuf,
    pub data_dir: PathBuf,
    pub wayland_socket: PathBuf,
    pub portal_socket: PathBuf,
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

    let scratch_root =
        PathBuf::from("/tmp/.saaios-sandbox-reveal").join(std::process::id().to_string());
    fs::create_dir_all(&scratch_root)?;

    let code_pin = scratch_root.join("code");
    let data_pin = scratch_root.join("data");
    let wayland_pin = scratch_root.join("wayland");
    let portal_pin = scratch_root.join("portal");
    pin_directory(&paths.code_dir, &code_pin)?;
    pin_directory(&paths.data_dir, &data_pin)?;
    pin_file(&paths.wayland_socket, &wayland_pin)?;
    pin_file(&paths.portal_socket, &portal_pin)?;

    // Hide the complete persistent root, including system binaries, packages,
    // grants, runtime memory/audit data and raw entity files. Reveal only this
    // app's immutable code and writable data directory.
    mask_tmpfs(&paths.data_root, "mode=0755,size=4m", true)?;
    reveal_directory(&code_pin, &paths.code_dir, true)?;
    reveal_directory(&data_pin, &paths.data_dir, false)?;

    // `/run` contains privileged appd/entityd sockets. Only Wayland and the
    // capability-checking portal cross the application boundary.
    mask_tmpfs(Path::new("/run"), "mode=0755,size=4m", true)?;
    reveal_file(&wayland_pin, &paths.wayland_socket)?;
    reveal_file(&portal_pin, &paths.portal_socket)?;

    for path in ["/metadata", "/proc", "/sys", "/saaios"] {
        mask_if_present(Path::new(path))?;
    }
    mask_device_tree(&scratch_root)?;

    let _ = fs::remove_dir_all(&scratch_root);
    mask_tmpfs(Path::new("/tmp"), "mode=1777,size=16m", true)?;

    make_root_read_only()?;
    drop_all_capabilities()?;

    install_seccomp_filter()?;

    Ok(())
}

fn pin_directory(source: &Path, scratch: &Path) -> io::Result<()> {
    fs::create_dir_all(scratch)?;
    mount(
        Some(source),
        scratch,
        None::<&str>,
        MsFlags::MS_BIND,
        None::<&str>,
    )
    .map_err(nix_to_io)
}

fn pin_file(source: &Path, scratch: &Path) -> io::Result<()> {
    let file_type = fs::symlink_metadata(source)?.file_type();
    if file_type.is_dir() || file_type.is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "sandbox endpoint is not a direct file: {}",
                source.display()
            ),
        ));
    }
    File::create(scratch)?;
    mount(
        Some(source),
        scratch,
        None::<&str>,
        MsFlags::MS_BIND,
        None::<&str>,
    )
    .map_err(nix_to_io)
}

fn reveal_directory(scratch: &Path, destination: &Path, read_only: bool) -> io::Result<()> {
    fs::create_dir_all(destination)?;
    mount(
        Some(scratch),
        destination,
        None::<&str>,
        MsFlags::MS_MOVE,
        None::<&str>,
    )
    .map_err(nix_to_io)?;
    let _ = fs::remove_dir(scratch);

    let flags = MsFlags::MS_BIND
        | MsFlags::MS_REMOUNT
        | MsFlags::MS_NOSUID
        | MsFlags::MS_NODEV
        | if read_only {
            MsFlags::MS_RDONLY
        } else {
            MsFlags::MS_NOEXEC
        };
    mount(None::<&str>, destination, None::<&str>, flags, None::<&str>).map_err(nix_to_io)
}

fn reveal_file(scratch: &Path, destination: &Path) -> io::Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    File::create(destination)?;
    mount(
        Some(scratch),
        destination,
        None::<&str>,
        MsFlags::MS_MOVE,
        None::<&str>,
    )
    .map_err(nix_to_io)?;
    fs::remove_file(scratch)
}

fn mask_tmpfs(path: &Path, options: &str, no_exec: bool) -> io::Result<()> {
    let mut flags = MsFlags::MS_NOSUID | MsFlags::MS_NODEV;
    if no_exec {
        flags |= MsFlags::MS_NOEXEC;
    }
    mount(None::<&str>, path, Some("tmpfs"), flags, Some(options)).map_err(nix_to_io)
}

fn mask_if_present(path: &Path) -> io::Result<()> {
    if path.exists() {
        mask_tmpfs(path, "mode=0555,size=64k", true)?;
    }
    Ok(())
}

fn mask_device_tree(scratch_root: &Path) -> io::Result<()> {
    let safe_devices = ["null", "zero", "random", "urandom"];
    let available = safe_devices
        .into_iter()
        .filter(|name| Path::new("/dev").join(name).exists())
        .collect::<Vec<_>>();
    for name in &available {
        pin_file(
            &Path::new("/dev").join(name),
            &scratch_root.join(format!("dev-{name}")),
        )?;
    }
    mount(
        None::<&str>,
        "/dev",
        Some("tmpfs"),
        MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC,
        Some("mode=0755,size=1m"),
    )
    .map_err(nix_to_io)?;
    for name in available {
        reveal_file(
            &scratch_root.join(format!("dev-{name}")),
            &Path::new("/dev").join(name),
        )?;
    }
    fs::create_dir("/dev/shm")?;
    mask_tmpfs(Path::new("/dev/shm"), "mode=1777,size=8m", false)
}

fn make_root_read_only() -> io::Result<()> {
    // A read-only bind of the root mount protects writable initramfs files
    // such as `/init`. The explicit app-data bind mount remains a separate
    // writable child mount; code is already a separate read-only child mount.
    mount(
        Some("/"),
        "/",
        None::<&str>,
        MsFlags::MS_BIND,
        None::<&str>,
    )
    .map_err(nix_to_io)?;
    mount(
        None::<&str>,
        "/",
        None::<&str>,
        MsFlags::MS_BIND | MsFlags::MS_REMOUNT | MsFlags::MS_RDONLY,
        None::<&str>,
    )
    .map_err(nix_to_io)
}

#[repr(C)]
struct CapabilityHeader {
    version: u32,
    pid: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CapabilityData {
    effective: u32,
    permitted: u32,
    inheritable: u32,
}

fn drop_all_capabilities() -> io::Result<()> {
    for capability in 0..64 {
        let result = unsafe { libc::prctl(libc::PR_CAPBSET_DROP, capability, 0, 0, 0) };
        if result < 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::EINVAL) {
                return Err(error);
            }
        }
    }
    let ambient = unsafe {
        libc::prctl(
            libc::PR_CAP_AMBIENT,
            libc::PR_CAP_AMBIENT_CLEAR_ALL,
            0,
            0,
            0,
        )
    };
    if ambient < 0 {
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::EINVAL) {
            return Err(error);
        }
    }

    const LINUX_CAPABILITY_VERSION_3: u32 = 0x2008_0522;
    let mut header = CapabilityHeader {
        version: LINUX_CAPABILITY_VERSION_3,
        pid: 0,
    };
    let mut data = [CapabilityData {
        effective: 0,
        permitted: 0,
        inheritable: 0,
    }; 2];
    let result = unsafe { libc::syscall(libc::SYS_capset, &mut header, data.as_mut_ptr()) };
    if result < 0 {
        return Err(io::Error::last_os_error());
    }
    let result = unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) };
    if result < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
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
    libc::SYS_bpf,
    libc::SYS_perf_event_open,
    libc::SYS_open_by_handle_at,
    libc::SYS_name_to_handle_at,
    libc::SYS_mknodat,
    libc::SYS_swapon,
    libc::SYS_swapoff,
    libc::SYS_keyctl,
    libc::SYS_add_key,
    libc::SYS_request_key,
];

#[cfg(target_arch = "x86_64")]
const ARCH_DENIED_SYSCALLS: &[i64] = &[libc::SYS_mknod];

#[cfg(target_arch = "aarch64")]
const ARCH_DENIED_SYSCALLS: &[i64] = &[];

fn install_seccomp_filter() -> io::Result<()> {
    let mut rules = BTreeMap::new();
    for syscall in DENIED_SYSCALLS.iter().chain(ARCH_DENIED_SYSCALLS) {
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
            data_root: PathBuf::from("/nonexistent"),
            code_dir: PathBuf::from("/nonexistent/apps/org.saaios.example"),
            data_dir: PathBuf::from("/nonexistent/var/apps/org.saaios.example"),
            wayland_socket: PathBuf::from("/nonexistent/run/wayland-1"),
            portal_socket: PathBuf::from("/nonexistent/run/portal.sock"),
        };
        let error = apply(&paths, &[Capability::NetInternet], false).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(apply(&paths, &[Capability::NetInternet], true).is_ok());
    }
}
