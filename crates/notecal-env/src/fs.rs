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
        drop(file);
        rename_with_retry(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

/// Renames `from` over `to`. On Windows, replacing a file fails while another
/// program (a sync client, an antivirus scanner) briefly has it open, so a
/// refused rename is retried for about half a second.
fn rename_with_retry(from: &Path, to: &Path) -> io::Result<()> {
    let mut delay_ms = 10;
    loop {
        match fs::rename(from, to) {
            Err(e)
                if cfg!(windows)
                    && e.kind() == io::ErrorKind::PermissionDenied
                    && delay_ms <= 160 =>
            {
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                delay_ms *= 2;
            }
            result => return result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_and_replaces_without_leaving_temp_files() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("note.txt");
        write_atomic(dir.path(), &path, "one").unwrap();
        write_atomic(dir.path(), &path, "two").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "two");
        let names: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["note.txt"]);
    }
}
