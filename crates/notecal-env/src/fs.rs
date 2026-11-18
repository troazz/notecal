//! Writing files safely.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

/// Prefix of `write_atomic`'s temp files; listings skip names starting with it.
pub const TEMP_PREFIX: &str = ".notecal-tmp-";

/// Writes through a hidden temp file in the same directory and renames it
/// over `path`, so a crash never leaves a half-written note.
pub fn write_atomic(dir: &Path, path: &Path, text: &str) -> io::Result<()> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let temp = dir.join(format!(
        "{TEMP_PREFIX}{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}
