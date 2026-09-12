//! S12 Change 4: writes staged, verified OTA content to the one
//! currently-owned SaaiOS partition set (`init_boot_a`/`vendor_boot_a`,
//! ADR-044) with a mandatory backup-before-write and automatic restore on
//! any failure. One-shot CLI, run manually via serial -- same "prove
//! before wiring into `native-init.c`" precedent as `saai-taskd`
//! (ADR-030) and `saai-ota-stage` (ADR-046). Never invents a new
//! partition-resolution mechanism -- `partition_io::
//! resolve_partition_device_node` is the same `PARTNAME` scan ADR-044
//! found already proven in `native-init.c::create_partition_node()`.

mod partition_io;

use clap::{Parser, Subcommand};
use ed25519_dalek::VerifyingKey;
use partition_io::{backup_write_verify, resolve_partition_device_node, restore_from_backup};
use saai_ota_manifest::{check_installable, verify_partition_bytes, SignedManifest};
use std::path::{Path, PathBuf};
use system_tools::{system_identity, ToolsMode};

/// ADR-044's write allow-list -- the only names `install` will ever
/// resolve and write to for real.
const DEFAULT_ALLOWED_PARTITIONS: &[&str] = &["init_boot_a", "vendor_boot_a"];

#[derive(Debug, Parser)]
#[command(name = "saai-ota-write")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Resolves a partition by PARTNAME and copies its entire current
    /// content to a local file. Read-only with respect to the partition.
    Backup {
        #[arg(long)]
        partition: String,
        #[arg(long, default_value = "/sys/class/block")]
        sys_class_block: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Writes a local file's content to a resolved partition, backing up
    /// its current content first and verifying the write by reading it
    /// back. Automatically restores from the backup on any failure. This
    /// is the one subcommand that can change a real boot partition.
    RawWrite {
        #[arg(long)]
        partition: String,
        #[arg(long, default_value = "/sys/class/block")]
        sys_class_block: PathBuf,
        #[arg(long)]
        content: PathBuf,
        #[arg(long)]
        backup: PathBuf,
    },
    /// Writes a previously taken backup file back to a resolved
    /// partition -- the manual rollback path.
    Restore {
        #[arg(long)]
        partition: String,
        #[arg(long, default_value = "/sys/class/block")]
        sys_class_block: PathBuf,
        #[arg(long)]
        backup: PathBuf,
    },
    /// Real install path: re-verifies a staged, signed manifest (staged
    /// files may be stale since Change 3 ran, so nothing from staging is
    /// trusted blindly), then backs up and writes each of its
    /// allow-listed partitions.
    Install {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        verifying_key: PathBuf,
        #[arg(long, default_value = "/data/saaios/update-staging")]
        staging_dir: PathBuf,
        #[arg(long, default_value = "/data/saaios/system/installed-version")]
        installed_version_file: PathBuf,
        #[arg(long, default_value = "/data/saaios/system/backup")]
        backup_dir: PathBuf,
        #[arg(long, default_value = "/sys/class/block")]
        sys_class_block: PathBuf,
        #[arg(long = "allow-partition")]
        allow_partitions: Vec<String>,
        /// Redirects one partition's write target to a local file instead
        /// of resolving it against real hardware. Exists ONLY so this
        /// command's manifest-verify-then-write wiring can be tested
        /// without a real partition in the loop -- never pass this for a
        /// real install. Format: `name=path`.
        #[arg(long = "unsafe-test-target-override", value_parser = parse_override_arg)]
        test_overrides: Vec<(String, PathBuf)>,
    },
}

fn parse_override_arg(raw: &str) -> Result<(String, PathBuf), String> {
    let (name, path) = raw
        .split_once('=')
        .ok_or_else(|| format!("expected name=path, got '{raw}'"))?;
    Ok((name.to_string(), PathBuf::from(path)))
}

fn fatal(msg: impl std::fmt::Display) -> ! {
    eprintln!("saai-ota-write: {msg}");
    std::process::exit(1);
}

fn main() {
    let args = Args::parse();
    match args.command {
        Command::Backup {
            partition,
            sys_class_block,
            out,
        } => cmd_backup(&partition, &sys_class_block, &out),
        Command::RawWrite {
            partition,
            sys_class_block,
            content,
            backup,
        } => cmd_raw_write(&partition, &sys_class_block, &content, &backup),
        Command::Restore {
            partition,
            sys_class_block,
            backup,
        } => cmd_restore(&partition, &sys_class_block, &backup),
        Command::Install {
            manifest,
            verifying_key,
            staging_dir,
            installed_version_file,
            backup_dir,
            sys_class_block,
            allow_partitions,
            test_overrides,
        } => cmd_install(
            &manifest,
            &verifying_key,
            &staging_dir,
            &installed_version_file,
            &backup_dir,
            &sys_class_block,
            allow_partitions,
            test_overrides,
        ),
    }
}

fn resolve_or_fatal(sys_class_block: &Path, partition: &str, device_node: &Path) {
    if let Err(err) = resolve_partition_device_node(sys_class_block, partition, device_node) {
        fatal(format!("resolving '{partition}': {err}"));
    }
}

fn cmd_backup(partition: &str, sys_class_block: &Path, out: &Path) {
    let device_node = PathBuf::from(format!("/dev/saaios-ota-{partition}"));
    resolve_or_fatal(sys_class_block, partition, &device_node);
    let content = std::fs::read(&device_node)
        .unwrap_or_else(|err| fatal(format!("reading {}: {err}", device_node.display())));
    std::fs::write(out, &content)
        .unwrap_or_else(|err| fatal(format!("writing {}: {err}", out.display())));
    println!(
        "saai-ota-write: backed up '{partition}' ({} bytes) -> {}",
        content.len(),
        out.display()
    );
}

fn cmd_raw_write(partition: &str, sys_class_block: &Path, content_path: &Path, backup_path: &Path) {
    let device_node = PathBuf::from(format!("/dev/saaios-ota-{partition}"));
    resolve_or_fatal(sys_class_block, partition, &device_node);
    let content = std::fs::read(content_path)
        .unwrap_or_else(|err| fatal(format!("reading {}: {err}", content_path.display())));

    match backup_write_verify(&device_node, backup_path, &content) {
        Ok(report) => println!(
            "saai-ota-write: '{partition}' written and verified ({} bytes written, {} bytes backed up to {})",
            report.written_bytes,
            report.backed_up_bytes,
            backup_path.display()
        ),
        Err(err) => fatal(format!("writing '{partition}': {err}")),
    }
}

fn cmd_restore(partition: &str, sys_class_block: &Path, backup_path: &Path) {
    let device_node = PathBuf::from(format!("/dev/saaios-ota-{partition}"));
    resolve_or_fatal(sys_class_block, partition, &device_node);
    match restore_from_backup(&device_node, backup_path) {
        Ok(report) => println!(
            "saai-ota-write: '{partition}' restored from {} ({} bytes)",
            backup_path.display(),
            report.written_bytes
        ),
        Err(err) => fatal(format!("restoring '{partition}': {err}")),
    }
}

#[allow(clippy::too_many_arguments)]
fn cmd_install(
    manifest_path: &Path,
    verifying_key_path: &Path,
    staging_dir: &Path,
    installed_version_file: &Path,
    backup_dir: &Path,
    sys_class_block: &Path,
    allow_partitions: Vec<String>,
    test_overrides: Vec<(String, PathBuf)>,
) {
    let manifest_json = std::fs::read_to_string(manifest_path)
        .unwrap_or_else(|err| fatal(format!("reading {}: {err}", manifest_path.display())));
    let signed: SignedManifest = serde_json::from_str(&manifest_json).unwrap_or_else(|err| {
        fatal(format!(
            "{}: malformed manifest: {err}",
            manifest_path.display()
        ))
    });

    let verifying_key = read_verifying_key(verifying_key_path);
    let device_target = detect_device_target();
    let installed_version = read_installed_version(installed_version_file);
    let allow_list: Vec<&str> = if allow_partitions.is_empty() {
        DEFAULT_ALLOWED_PARTITIONS.to_vec()
    } else {
        allow_partitions.iter().map(String::as_str).collect()
    };

    if let Err(err) = check_installable(
        &signed,
        &verifying_key,
        &device_target,
        &installed_version,
        &allow_list,
    ) {
        fatal(format!("manifest rejected, nothing written: {err}"));
    }
    println!(
        "saai-ota-write: manifest re-verified (version={}), proceeding to write {} partition(s)",
        signed.body.version,
        signed.body.partitions.len()
    );

    if !test_overrides.is_empty() {
        eprintln!(
            "saai-ota-write: WARNING -- --unsafe-test-target-override is active for {} partition(s), this is NOT a real install",
            test_overrides.len()
        );
    }

    std::fs::create_dir_all(backup_dir)
        .unwrap_or_else(|err| fatal(format!("creating {}: {err}", backup_dir.display())));

    let mut all_ok = true;
    for entry in &signed.body.partitions {
        let staged_path = staging_dir.join(format!("{}.img", entry.name));
        let staged = match std::fs::read(&staged_path) {
            Ok(bytes) => bytes,
            Err(err) => {
                eprintln!(
                    "saai-ota-write: '{}' skipped: reading staged file {}: {err}",
                    entry.name,
                    staged_path.display()
                );
                all_ok = false;
                continue;
            }
        };
        if let Err(err) = verify_partition_bytes(&signed.body, &entry.name, &staged) {
            eprintln!(
                "saai-ota-write: '{}' skipped: staged content no longer matches manifest: {err}",
                entry.name
            );
            all_ok = false;
            continue;
        }

        let backup_path = backup_dir.join(format!("{}.img.bak", entry.name));
        let override_path = test_overrides
            .iter()
            .find(|(name, _)| name == &entry.name)
            .map(|(_, path)| path.clone());

        let target = match override_path {
            Some(path) => path,
            None => {
                let device_node = PathBuf::from(format!("/dev/saaios-ota-{}", entry.name));
                resolve_or_fatal(sys_class_block, &entry.name, &device_node);
                device_node
            }
        };

        match backup_write_verify(&target, &backup_path, &staged) {
            Ok(report) => println!(
                "saai-ota-write: '{}' written and verified ({} bytes, backup {} bytes at {})",
                entry.name,
                report.written_bytes,
                report.backed_up_bytes,
                backup_path.display()
            ),
            Err(err) => {
                eprintln!("saai-ota-write: '{}' FAILED: {err}", entry.name);
                all_ok = false;
            }
        }
    }

    if !all_ok {
        fatal("one or more partitions failed to install -- see above; any partition that did write was backed up first");
    }
    println!("saai-ota-write: install complete, all partitions written and verified");
}

fn detect_device_target() -> String {
    let identity = system_identity(ToolsMode::RealLinux);
    identity
        .get("target")
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| {
            fatal("could not determine device target (androidboot.hardware) from /proc/bootconfig")
        })
}

fn read_installed_version(path: &Path) -> semver::Version {
    match std::fs::read_to_string(path) {
        Ok(text) => semver::Version::parse(text.trim())
            .unwrap_or_else(|err| fatal(format!("{}: not valid semver: {err}", path.display()))),
        Err(_) => semver::Version::new(0, 0, 0),
    }
}

fn read_verifying_key(path: &Path) -> VerifyingKey {
    let hex_str = std::fs::read_to_string(path)
        .unwrap_or_else(|err| fatal(format!("reading {}: {err}", path.display())));
    let bytes = hex::decode(hex_str.trim())
        .unwrap_or_else(|err| fatal(format!("{}: not valid hex: {err}", path.display())));
    let bytes: [u8; 32] = bytes
        .try_into()
        .unwrap_or_else(|_| fatal(format!("{}: expected 32 bytes", path.display())));
    VerifyingKey::from_bytes(&bytes).unwrap_or_else(|err| {
        fatal(format!(
            "{}: not a valid ed25519 key: {err}",
            path.display()
        ))
    })
}
