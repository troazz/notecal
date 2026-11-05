//! Notes as `.txt` files in one directory (PRD N1, N2, N5 and the storage
//! rules). Plain Rust with no Tauri types, so every rule is unit-tested.
//!
//! The PRD's "content hash" check is done by remembering the text last read
//! or written and comparing it with the file: notes are small, and the
//! comparison is exact.

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::UNIX_EPOCH;

use serde::Serialize;

const EXTENSION: &str = "txt";
const TEMP_PREFIX: &str = ".notecal-tmp-";
const MAX_NAME_BYTES: usize = 120;

/// One entry in the note list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteInfo {
    pub name: String,
    /// Last modification, in milliseconds since the Unix epoch.
    pub modified_ms: u64,
}

/// What `save` did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SaveOutcome {
    Saved,
    /// The file changed on disk since it was last read or written. The local
    /// text went to `copy_name` and `disk_text` is what the file holds now.
    #[serde(rename_all = "camelCase")]
    Conflict {
        copy_name: String,
        disk_text: String,
    },
}

/// How the file of an open note compares with what the app last saw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiskChange {
    Unchanged,
    Changed(String),
    Deleted,
}

/// What the app last read from or wrote to a note's file.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Known {
    Content(String),
    Missing,
}

pub struct NoteStore {
    dir: PathBuf,
    known: HashMap<String, Known>,
}

impl NoteStore {
    /// Opens the notes directory, creating it if needed.
    pub fn new(dir: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&dir)
            .map_err(|e| format!("Could not create the notes folder {}: {e}", dir.display()))?;
        Ok(Self {
            dir,
            known: HashMap::new(),
        })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The note name for a path in the notes directory, if it is a note file.
    pub fn name_of(&self, path: &Path) -> Option<String> {
        if path.parent()? != self.dir || path.extension()? != EXTENSION {
            return None;
        }
        let name = path.file_stem()?.to_str()?;
        validate_name(name).ok()?;
        Some(name.to_string())
    }

    /// All notes, most recently modified first.
    pub fn list(&self) -> Result<Vec<NoteInfo>, String> {
        let entries = fs::read_dir(&self.dir).map_err(|e| format!("Could not list notes: {e}"))?;
        let mut notes: Vec<NoteInfo> = entries
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_ok_and(|t| t.is_file()))
            .filter_map(|entry| {
                let name = self.name_of(&entry.path())?;
                let modified_ms = entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
                Some(NoteInfo { name, modified_ms })
            })
            .collect();
        notes.sort_by(|a, b| b.modified_ms.cmp(&a.modified_ms).then(a.name.cmp(&b.name)));
        Ok(notes)
    }

    pub fn open(&mut self, name: &str) -> Result<String, String> {
        validate_name(name)?;
        let text = fs::read_to_string(self.path(name))
            .map_err(|e| format!("Could not open '{name}': {e}"))?;
        self.known
            .insert(name.to_string(), Known::Content(text.clone()));
        Ok(text)
    }

    /// Saves `text`, unless the file changed on disk since it was last read
    /// or written: then `text` goes to a conflict copy named with `stamp`
    /// (`YYYY-MM-DD HHMM`, local time, supplied by the caller) and the file
    /// is left as it is.
    pub fn save(&mut self, name: &str, text: &str, stamp: &str) -> Result<SaveOutcome, String> {
        validate_name(name)?;
        let path = self.path(name);
        let disk = read_if_exists(&path).map_err(|e| format!("Could not check '{name}': {e}"))?;
        let unchanged = match (&disk, self.known.get(name)) {
            // Deleted elsewhere: keep the buffer and write it again.
            (None, _) => true,
            (Some(disk), Some(Known::Content(known))) => disk == known,
            (Some(_), Some(Known::Missing) | None) => false,
        };
        if unchanged {
            write_atomic(&self.dir, &path, text)
                .map_err(|e| format!("Could not save '{name}': {e}"))?;
            self.known
                .insert(name.to_string(), Known::Content(text.to_string()));
            return Ok(SaveOutcome::Saved);
        }
        let disk_text = disk.unwrap_or_default();
        let copy_name = self.free_name(&format!("{name} (conflict {stamp})"))?;
        write_atomic(&self.dir, &self.path(&copy_name), text)
            .map_err(|e| format!("Could not write the conflict copy '{copy_name}': {e}"))?;
        self.known
            .insert(name.to_string(), Known::Content(disk_text.clone()));
        Ok(SaveOutcome::Conflict {
            copy_name,
            disk_text,
        })
    }

    pub fn create(&mut self, name: &str) -> Result<(), String> {
        validate_name(name)?;
        let path = self.path(name);
        if path.exists() {
            return Err(format!("A note named '{name}' already exists."));
        }
        write_atomic(&self.dir, &path, "")
            .map_err(|e| format!("Could not create '{name}': {e}"))?;
        self.known
            .insert(name.to_string(), Known::Content(String::new()));
        Ok(())
    }

    pub fn rename(&mut self, from: &str, to: &str) -> Result<(), String> {
        validate_name(from)?;
        validate_name(to)?;
        if from == to {
            return Ok(());
        }
        // Allow changing only the letter case on case-insensitive filesystems.
        if self.path(to).exists() && !from.eq_ignore_ascii_case(to) {
            return Err(format!("A note named '{to}' already exists."));
        }
        fs::rename(self.path(from), self.path(to))
            .map_err(|e| format!("Could not rename '{from}': {e}"))?;
        if let Some(known) = self.known.remove(from) {
            self.known.insert(to.to_string(), known);
        }
        Ok(())
    }

    pub fn delete(&mut self, name: &str) -> Result<(), String> {
        validate_name(name)?;
        fs::remove_file(self.path(name)).map_err(|e| format!("Could not delete '{name}': {e}"))?;
        self.known.remove(name);
        Ok(())
    }

    /// Compares an open note's file with what the app last read or wrote.
    /// Does not update that record: the caller either reloads (`open`) or
    /// saves its own edits (`save`, which then makes a conflict copy).
    pub fn disk_changed(&self, name: &str) -> Result<DiskChange, String> {
        let Some(known) = self.known.get(name) else {
            return Ok(DiskChange::Unchanged);
        };
        let disk = read_if_exists(&self.path(name))
            .map_err(|e| format!("Could not check '{name}': {e}"))?;
        Ok(match (known, disk) {
            (Known::Content(known), Some(disk)) if *known == disk => DiskChange::Unchanged,
            (Known::Missing, None) => DiskChange::Unchanged,
            (_, Some(disk)) => DiskChange::Changed(disk),
            (Known::Content(_), None) => DiskChange::Deleted,
        })
    }

    /// Records that the app saw `name`'s file deleted, so a later appearance
    /// of the file counts as an external change.
    pub fn mark_missing(&mut self, name: &str) {
        self.known.insert(name.to_string(), Known::Missing);
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{name}.{EXTENSION}"))
    }

    /// `base`, or `base 2`, `base 3`… if a note already has that name.
    fn free_name(&self, base: &str) -> Result<String, String> {
        validate_name(base)?;
        if !self.path(base).exists() {
            return Ok(base.to_string());
        }
        (2..1000)
            .map(|n| format!("{base} {n}"))
            .find(|candidate| !self.path(candidate).exists())
            .ok_or_else(|| format!("Too many notes named '{base}'."))
    }
}

/// Checks a note name: it becomes a file name, so it must be one safe path
/// component.
pub fn validate_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("A note name cannot be empty.".into());
    }
    if name != name.trim() {
        return Err("A note name cannot start or end with a space.".into());
    }
    if name.starts_with('.') {
        return Err("A note name cannot start with '.'.".into());
    }
    if name.contains(['/', '\\', '\0']) || name.chars().any(char::is_control) {
        return Err("A note name cannot contain '/', '\\' or control characters.".into());
    }
    if name.len() > MAX_NAME_BYTES {
        return Err(format!(
            "A note name can be at most {MAX_NAME_BYTES} bytes long."
        ));
    }
    Ok(())
}

fn read_if_exists(path: &Path) -> io::Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const STAMP: &str = "2026-11-05 1830";

    fn store() -> (TempDir, NoteStore) {
        let dir = TempDir::new().unwrap();
        let store = NoteStore::new(dir.path().to_path_buf()).unwrap();
        (dir, store)
    }

    fn write_externally(store: &NoteStore, name: &str, text: &str) {
        fs::write(store.path(name), text).unwrap();
    }

    fn read(store: &NoteStore, name: &str) -> String {
        fs::read_to_string(store.path(name)).unwrap()
    }

    #[test]
    fn saves_and_reopens() {
        let (_dir, mut store) = store();
        store.create("Budget").unwrap();
        assert_eq!(store.save("Budget", "1 + 1", STAMP), Ok(SaveOutcome::Saved));
        assert_eq!(store.save("Budget", "1 + 2", STAMP), Ok(SaveOutcome::Saved));
        assert_eq!(store.open("Budget").unwrap(), "1 + 2");
    }

    #[test]
    fn external_change_without_local_edits_is_reported_for_reload() {
        let (_dir, mut store) = store();
        store.create("Budget").unwrap();
        store.save("Budget", "a", STAMP).unwrap();
        write_externally(&store, "Budget", "b");
        assert_eq!(
            store.disk_changed("Budget"),
            Ok(DiskChange::Changed("b".into()))
        );
        assert_eq!(store.open("Budget").unwrap(), "b");
        assert_eq!(store.disk_changed("Budget"), Ok(DiskChange::Unchanged));
    }

    #[test]
    fn own_writes_are_not_external_changes() {
        let (_dir, mut store) = store();
        store.create("Budget").unwrap();
        store.save("Budget", "a", STAMP).unwrap();
        assert_eq!(store.disk_changed("Budget"), Ok(DiskChange::Unchanged));
    }

    #[test]
    fn external_change_with_local_edits_writes_a_conflict_copy() {
        let (_dir, mut store) = store();
        store.create("Budget").unwrap();
        store.save("Budget", "a", STAMP).unwrap();
        write_externally(&store, "Budget", "from sync");

        let outcome = store.save("Budget", "my edit", STAMP).unwrap();

        let copy = "Budget (conflict 2026-11-05 1830)".to_string();
        assert_eq!(
            outcome,
            SaveOutcome::Conflict {
                copy_name: copy.clone(),
                disk_text: "from sync".into()
            }
        );
        assert_eq!(read(&store, "Budget"), "from sync");
        assert_eq!(read(&store, &copy), "my edit");
        // The disk version is now the known one, so the next save goes through.
        assert_eq!(store.save("Budget", "next", STAMP), Ok(SaveOutcome::Saved));
    }

    #[test]
    fn conflict_copies_never_overwrite_each_other() {
        let (_dir, mut store) = store();
        store.create("Budget").unwrap();
        for text in ["one", "two"] {
            write_externally(&store, "Budget", &format!("sync {text}"));
            store.save("Budget", text, STAMP).unwrap();
        }
        assert_eq!(read(&store, "Budget (conflict 2026-11-05 1830)"), "one");
        assert_eq!(read(&store, "Budget (conflict 2026-11-05 1830) 2"), "two");
    }

    #[test]
    fn deleted_note_is_reported_and_saved_again() {
        let (_dir, mut store) = store();
        store.create("Budget").unwrap();
        store.save("Budget", "a", STAMP).unwrap();
        fs::remove_file(store.path("Budget")).unwrap();
        assert_eq!(store.disk_changed("Budget"), Ok(DiskChange::Deleted));
        assert_eq!(store.save("Budget", "a2", STAMP), Ok(SaveOutcome::Saved));
        assert_eq!(read(&store, "Budget"), "a2");
    }

    #[test]
    fn file_reappearing_after_delete_counts_as_external() {
        let (_dir, mut store) = store();
        store.create("Budget").unwrap();
        fs::remove_file(store.path("Budget")).unwrap();
        store.mark_missing("Budget");
        assert_eq!(store.disk_changed("Budget"), Ok(DiskChange::Unchanged));
        write_externally(&store, "Budget", "restored");
        assert_eq!(
            store.disk_changed("Budget"),
            Ok(DiskChange::Changed("restored".into()))
        );
    }

    #[test]
    fn list_skips_temp_hidden_and_other_files() {
        let (dir, mut store) = store();
        store.create("Budget").unwrap();
        fs::write(dir.path().join(".notecal-tmp-1-0"), "x").unwrap();
        fs::write(dir.path().join(".syncthing.Budget.txt.tmp"), "x").unwrap();
        fs::write(dir.path().join(".hidden.txt"), "x").unwrap();
        fs::write(dir.path().join("image.png"), "x").unwrap();
        fs::create_dir(dir.path().join("folder.txt")).unwrap();
        let names: Vec<String> = store.list().unwrap().into_iter().map(|n| n.name).collect();
        assert_eq!(names, ["Budget"]);
    }

    #[test]
    fn atomic_write_leaves_no_temp_files() {
        let (dir, mut store) = store();
        store.create("Budget").unwrap();
        store.save("Budget", "a", STAMP).unwrap();
        let leftovers = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().starts_with(TEMP_PREFIX))
            .count();
        assert_eq!(leftovers, 0);
    }

    #[test]
    fn create_and_rename_refuse_existing_names() {
        let (_dir, mut store) = store();
        store.create("A").unwrap();
        store.create("B").unwrap();
        assert!(store.create("A").is_err());
        assert!(store.rename("A", "B").is_err());
        store.rename("A", "C").unwrap();
        assert_eq!(store.open("C").unwrap(), "");
        assert!(store.open("A").is_err());
    }

    #[test]
    fn rename_keeps_the_conflict_check() {
        let (_dir, mut store) = store();
        store.create("A").unwrap();
        store.save("A", "a", STAMP).unwrap();
        store.rename("A", "B").unwrap();
        assert_eq!(store.save("B", "b", STAMP), Ok(SaveOutcome::Saved));
    }

    #[test]
    fn delete_removes_the_file() {
        let (_dir, mut store) = store();
        store.create("A").unwrap();
        store.delete("A").unwrap();
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn invalid_names_are_refused() {
        for name in ["", "  ", " a", "a ", ".a", "a/b", "a\\b", "a\0b", "a\nb"] {
            assert!(validate_name(name).is_err(), "{name:?} should be refused");
        }
        assert!(validate_name(&"x".repeat(MAX_NAME_BYTES + 1)).is_err());
        for name in ["Budget", "Trip 2026", "Catatan belanja", "a.b", "日本"] {
            assert!(validate_name(name).is_ok(), "{name:?} should be allowed");
        }
    }

    #[test]
    fn name_of_accepts_only_note_files_in_the_folder() {
        let (dir, store) = store();
        assert_eq!(
            store.name_of(&dir.path().join("Budget.txt")),
            Some("Budget".into())
        );
        assert_eq!(store.name_of(&dir.path().join(".notecal-tmp-1-0")), None);
        assert_eq!(store.name_of(&dir.path().join("a.md")), None);
        assert_eq!(store.name_of(Path::new("/elsewhere/Budget.txt")), None);
    }
}
