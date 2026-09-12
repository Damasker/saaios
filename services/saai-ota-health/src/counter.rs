//! S12 Change 5: persisted boot-attempt counter. A plain text integer
//! file -- deliberately simple, matching this Change's "prove the
//! mechanism manually first" scope (no `native-init.c` wiring yet).

use std::path::Path;

pub fn read(path: &Path) -> u32 {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or(0)
}

pub fn write(path: &Path, value: u32) -> std::io::Result<()> {
    std::fs::write(path, format!("{value}\n"))
}

pub fn increment(path: &Path) -> std::io::Result<u32> {
    let next = read(path) + 1;
    write(path, next)?;
    Ok(next)
}

pub fn reset(path: &Path) -> std::io::Result<()> {
    write(path, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn missing_file_reads_as_zero() {
        let dir = tempdir().unwrap();
        assert_eq!(read(&dir.path().join("no-such-file")), 0);
    }

    #[test]
    fn increment_persists_across_reads() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("counter");
        assert_eq!(increment(&path).unwrap(), 1);
        assert_eq!(increment(&path).unwrap(), 2);
        assert_eq!(increment(&path).unwrap(), 3);
        assert_eq!(read(&path), 3);
    }

    #[test]
    fn reset_zeroes_the_counter() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("counter");
        increment(&path).unwrap();
        increment(&path).unwrap();
        reset(&path).unwrap();
        assert_eq!(read(&path), 0);
    }

    #[test]
    fn malformed_content_reads_as_zero_rather_than_panicking() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("counter");
        std::fs::write(&path, "not-a-number\n").unwrap();
        assert_eq!(read(&path), 0);
    }
}
