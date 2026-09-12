//! S12 Change 4: real partition write mechanics, factored so the
//! dangerous half (raw read/write/verify/restore against a path) is
//! fully host-testable against plain files, and only the partition
//! *resolution* half needs a real device to prove for real (already done
//! in Change 1/ADR-044 for the read-only case -- this module reuses the
//! exact same `PARTNAME` scan, just to `mknod` a node this process can
//! also open for writing).

use std::ffi::CString;
use std::fs;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use saai_ota_manifest::sha256_hex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PartitionIoError {
    #[error("partition '{0}' not found under {1}")]
    PartitionNotFound(String, String),
    #[error("{0}: {1}")]
    Io(String, std::io::Error),
    #[error("uevent for '{0}' is missing MAJOR/MINOR")]
    MissingMajorMinor(String),
    #[error("mknod {0}: {1}")]
    Mknod(String, std::io::Error),
    #[error(
        "content mismatch after write: expected sha256={expected} size={expected_size}, got sha256={actual} size={actual_size}"
    )]
    VerifyMismatch {
        expected: String,
        expected_size: u64,
        actual: String,
        actual_size: u64,
    },
    #[error("write failed ({write_error}) AND restoring the backup also failed ({restore_error}) -- manual recovery needed")]
    RestoreAlsoFailed {
        write_error: String,
        restore_error: String,
    },
}

/// Scans `sys_class_block_root` (real device: `/sys/class/block`,
/// injectable here so this exact matching logic can be unit-tested
/// against a fake directory tree without touching real hardware paths)
/// for a block device whose `uevent` carries `PARTNAME=<partition_name>`,
/// then `mknod`s a block-special file at `device_node_path` with that
/// device's real major/minor. Mirrors `create_partition_node()` in
/// `native-init.c` (ADR-044) -- deliberately not a new resolution
/// mechanism, the same one already proven every boot.
pub fn resolve_partition_device_node(
    sys_class_block_root: &Path,
    partition_name: &str,
    device_node_path: &Path,
) -> Result<(), PartitionIoError> {
    let entries = fs::read_dir(sys_class_block_root)
        .map_err(|err| PartitionIoError::Io(sys_class_block_root.display().to_string(), err))?;
    let expected_line = format!("PARTNAME={partition_name}");

    for entry in entries.flatten() {
        let uevent_path = entry.path().join("uevent");
        let Ok(contents) = fs::read_to_string(&uevent_path) else {
            continue;
        };
        if !contents.lines().any(|line| line == expected_line) {
            continue;
        }

        let major = contents
            .lines()
            .find_map(|line| line.strip_prefix("MAJOR="))
            .and_then(|value| value.parse::<u32>().ok());
        let minor = contents
            .lines()
            .find_map(|line| line.strip_prefix("MINOR="))
            .and_then(|value| value.parse::<u32>().ok());
        let (major, minor) = match (major, minor) {
            (Some(major), Some(minor)) => (major, minor),
            _ => {
                return Err(PartitionIoError::MissingMajorMinor(
                    partition_name.to_string(),
                ))
            }
        };

        let _ = fs::remove_file(device_node_path);
        make_block_device_node(device_node_path, major, minor)
            .map_err(|err| PartitionIoError::Mknod(device_node_path.display().to_string(), err))?;
        return Ok(());
    }

    Err(PartitionIoError::PartitionNotFound(
        partition_name.to_string(),
        sys_class_block_root.display().to_string(),
    ))
}

#[cfg(unix)]
fn make_block_device_node(path: &Path, major: u32, minor: u32) -> std::io::Result<()> {
    let c_path = CString::new(path.as_os_str().as_bytes()).map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "path has interior NUL")
    })?;
    let dev = libc::makedev(major, minor);
    let rc = unsafe { libc::mknod(c_path.as_ptr(), libc::S_IFBLK | 0o600, dev) };
    if rc != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

pub struct WriteReport {
    pub backed_up_bytes: u64,
    pub written_bytes: u64,
}

/// Backs up the current content of `target` to `backup_path`, then writes
/// `new_content` and reads it back to confirm an exact match over
/// `new_content`'s length (a partition may be larger than the image
/// written into it -- trailing bytes beyond the image are untouched and
/// irrelevant). On any failure once the backup has succeeded, attempts to
/// restore `target` from the just-taken backup before returning the
/// original error -- so a failed write never has a worse outcome than a
/// successful no-op, other than the two errors this returns if the
/// restore attempt itself also fails.
pub fn backup_write_verify(
    target: &Path,
    backup_path: &Path,
    new_content: &[u8],
) -> Result<WriteReport, PartitionIoError> {
    let current =
        fs::read(target).map_err(|err| PartitionIoError::Io(target.display().to_string(), err))?;
    atomic_write(backup_path, &current)
        .map_err(|err| PartitionIoError::Io(backup_path.display().to_string(), err))?;

    match write_and_verify(target, new_content) {
        Ok(()) => Ok(WriteReport {
            backed_up_bytes: current.len() as u64,
            written_bytes: new_content.len() as u64,
        }),
        Err(write_err) => match write_and_verify(target, &current) {
            Ok(()) => Err(write_err),
            Err(restore_err) => Err(PartitionIoError::RestoreAlsoFailed {
                write_error: write_err.to_string(),
                restore_error: restore_err.to_string(),
            }),
        },
    }
}

/// The manual rollback path: writes `backup_path`'s content straight back
/// to `target`, verifying it. No further backup is taken of whatever is
/// currently on `target` -- if this is being called, `target` is already
/// assumed to be in an unwanted state.
pub fn restore_from_backup(
    target: &Path,
    backup_path: &Path,
) -> Result<WriteReport, PartitionIoError> {
    let backup = fs::read(backup_path)
        .map_err(|err| PartitionIoError::Io(backup_path.display().to_string(), err))?;
    write_and_verify(target, &backup)?;
    Ok(WriteReport {
        backed_up_bytes: 0,
        written_bytes: backup.len() as u64,
    })
}

fn write_and_verify(target: &Path, content: &[u8]) -> Result<(), PartitionIoError> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .open(target)
        .map_err(|err| PartitionIoError::Io(target.display().to_string(), err))?;
    file.write_all(content)
        .map_err(|err| PartitionIoError::Io(target.display().to_string(), err))?;
    file.sync_all()
        .map_err(|err| PartitionIoError::Io(target.display().to_string(), err))?;
    drop(file);

    let actual =
        fs::read(target).map_err(|err| PartitionIoError::Io(target.display().to_string(), err))?;
    let actual_prefix = &actual[..content.len().min(actual.len())];
    if actual_prefix != content {
        return Err(PartitionIoError::VerifyMismatch {
            expected: sha256_hex(content),
            expected_size: content.len() as u64,
            actual: sha256_hex(actual_prefix),
            actual_size: actual_prefix.len() as u64,
        });
    }
    Ok(())
}

fn atomic_write(path: &Path, content: &[u8]) -> std::io::Result<()> {
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, content)?;
    fs::rename(&tmp_path, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_uevent(dir: &Path, block_name: &str, partname: &str, major: u32, minor: u32) {
        let block_dir = dir.join(block_name);
        fs::create_dir_all(&block_dir).unwrap();
        fs::write(
            block_dir.join("uevent"),
            format!("MAJOR={major}\nMINOR={minor}\nDEVNAME={block_name}\nPARTNAME={partname}\n"),
        )
        .unwrap();
    }

    #[test]
    fn resolves_partition_by_exact_partname_match() {
        let sysfs = tempdir().unwrap();
        write_uevent(sysfs.path(), "sda1", "persist", 8, 1);
        write_uevent(sysfs.path(), "sda11", "init_boot_a", 8, 11);
        write_uevent(sysfs.path(), "sda12", "vendor_boot_a", 8, 12);

        // No real mknod capability is assumed in a test sandbox -- this
        // test only proves the *scan and match* logic finds the right
        // MAJOR/MINOR; it accepts either a successful mknod or a
        // permission/capability error from the mknod syscall itself, but
        // never `PartitionNotFound` or `MissingMajorMinor` for a
        // partition that is clearly present in the fixture.
        let device_node = sysfs.path().join("would-be-device-node");
        let result = resolve_partition_device_node(sysfs.path(), "init_boot_a", &device_node);
        assert!(!matches!(
            result,
            Err(PartitionIoError::PartitionNotFound(..))
                | Err(PartitionIoError::MissingMajorMinor(..))
        ));
    }

    #[test]
    fn does_not_match_partition_name_as_a_substring() {
        let sysfs = tempdir().unwrap();
        // "init_boot_a" must not match a uevent whose PARTNAME is
        // "init_boot_a_extra" or "init_boot" -- exact line match only.
        write_uevent(sysfs.path(), "sda11", "init_boot_a_extra", 8, 11);
        write_uevent(sysfs.path(), "sda10", "init_boot", 8, 10);

        let device_node = sysfs.path().join("would-be-device-node");
        let result = resolve_partition_device_node(sysfs.path(), "init_boot_a", &device_node);
        assert!(matches!(
            result,
            Err(PartitionIoError::PartitionNotFound(..))
        ));
    }

    #[test]
    fn missing_partition_is_reported_precisely() {
        let sysfs = tempdir().unwrap();
        write_uevent(sysfs.path(), "sda1", "persist", 8, 1);

        let device_node = sysfs.path().join("would-be-device-node");
        let result = resolve_partition_device_node(sysfs.path(), "init_boot_a", &device_node);
        assert!(matches!(
            result,
            Err(PartitionIoError::PartitionNotFound(..))
        ));
    }

    #[test]
    fn backup_write_verify_round_trip_succeeds() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("fake-partition.img");
        let backup = dir.path().join("fake-partition.img.bak");
        fs::write(&target, b"old content, same length!").unwrap();

        let report = backup_write_verify(&target, &backup, b"new content, same length!").unwrap();
        assert_eq!(
            report.backed_up_bytes,
            "old content, same length!".len() as u64
        );
        assert_eq!(
            report.written_bytes,
            "new content, same length!".len() as u64
        );
        assert_eq!(fs::read(&target).unwrap(), b"new content, same length!");
        assert_eq!(fs::read(&backup).unwrap(), b"old content, same length!");
    }

    #[test]
    fn backup_preserves_original_when_target_is_larger_than_new_content() {
        // Real partitions are usually larger than the image written into
        // them -- only the written prefix is checked, trailing bytes are
        // untouched and irrelevant.
        let dir = tempdir().unwrap();
        let target = dir.path().join("fake-partition.img");
        let backup = dir.path().join("fake-partition.img.bak");
        fs::write(&target, b"0123456789ABCDEF").unwrap(); // 16 bytes

        let report = backup_write_verify(&target, &backup, b"NEW!").unwrap();
        assert_eq!(report.written_bytes, 4);
        assert_eq!(&fs::read(&target).unwrap()[..4], b"NEW!");
    }

    #[test]
    fn restore_from_backup_writes_the_backup_content_back() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("fake-partition.img");
        let backup = dir.path().join("fake-partition.img.bak");
        fs::write(&target, b"corrupted-or-wrong").unwrap();
        fs::write(&backup, b"known-good-content").unwrap();

        restore_from_backup(&target, &backup).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"known-good-content");
    }

    #[test]
    fn write_failure_triggers_automatic_restore() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("readonly-partition.img");
        let backup = dir.path().join("readonly-partition.img.bak");
        fs::write(&target, b"original-good-content").unwrap();

        // Make the target read-only after the backup step would have
        // already read it, so the write attempt itself fails and the
        // automatic-restore path runs -- since restore writes back the
        // exact same bytes that are already there, it also fails against
        // a read-only file, exercising RestoreAlsoFailed honestly rather
        // than faking a partial-failure scenario.
        let mut perms = fs::metadata(&target).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&target, perms).unwrap();

        let result = backup_write_verify(&target, &backup, b"attempted-new-content");
        assert!(matches!(
            result,
            Err(PartitionIoError::RestoreAlsoFailed { .. })
        ));
        // Un-corrupted: the read-only file's content was never actually
        // changed by the failed write attempt.
        assert_eq!(fs::read(&target).unwrap(), b"original-good-content");

        // Restore normal permissions so the tempdir can clean itself up.
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&target, fs::Permissions::from_mode(0o644)).unwrap();
    }
}
