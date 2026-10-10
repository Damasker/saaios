use std::fs;
use std::path::{Path, PathBuf};

pub use crate::redact::REDACTED_FILES;

use crate::redact::redact_text;

/// Copy the allowlisted snapshot files into `dest`, then redact them.
///
/// Files outside [`REDACTED_FILES`] are left behind. `dest` is created.
pub fn normalize(raw: &Path, dest: &Path) -> std::io::Result<PathBuf> {
    fs::create_dir_all(dest)?;
    for name in REDACTED_FILES {
        let source = raw.join(name);
        if !source.is_file() {
            continue;
        }
        let text = fs::read_to_string(&source)?;
        fs::write(dest.join(name), redact_text(&text))?;
    }
    Ok(dest.to_path_buf())
}
