//! Notes as `.txt` files in one directory (PRD N1, N2, N5 and the storage
//! rules). Plain Rust with no Tauri types, so every rule is unit-tested.
//!
//! The PRD's "content hash" check is done by remembering the text last read
//! or written and comparing it with the file: notes are small, and the
//! comparison is exact.

use notecal_env::fs::write_atomic;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Serialize;

const EXTENSION: &str = "txt";
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

    /// The note name for a path in the notes directory, if it is a note
    /// file or an iCloud Drive placeholder for one (`.Budget.txt.icloud`,
    /// left when macOS evicts a synced file to save space).
    pub fn name_of(&self, path: &Path) -> Option<String> {
        if path.parent()? != self.dir {
            return None;
        }
        let file_name = path.file_name()?.to_str()?;
        let name = match file_name.strip_suffix(ICLOUD_SUFFIX) {
            Some(rest) => rest.strip_prefix('.')?.strip_suffix(".txt")?,
            None if path.extension()? == EXTENSION => path.file_stem()?.to_str()?,
            None => return None,
        };
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
        // A note being downloaded from iCloud has both files for a moment.
        notes.sort_by(|a, b| a.name.cmp(&b.name).then(b.modified_ms.cmp(&a.modified_ms)));
        notes.dedup_by(|a, b| a.name == b.name);
        notes.sort_by(|a, b| b.modified_ms.cmp(&a.modified_ms).then(a.name.cmp(&b.name)));
        Ok(notes)
    }

    pub fn open(&mut self, name: &str) -> Result<String, String> {
        validate_name(name)?;
        if !self.path(name).exists() && self.icloud_placeholder(name).exists() {
            download_from_icloud(&self.path(name))
                .map_err(|e| format!("'{name}' is in iCloud Drive and not downloaded yet: {e}"))?;
        }
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
        // `disk` is `Some` here: a missing file counts as unchanged above.
        let copy_name = self.free_name(&conflict_base(name, stamp))?;
        write_atomic(&self.dir, &self.path(&copy_name), text)
            .map_err(|e| format!("Could not write the conflict copy '{copy_name}': {e}"))?;
        // Known, so the UI can update the copy with keystrokes typed while
        // this save was in flight.
        self.known
            .insert(copy_name.clone(), Known::Content(text.to_string()));
        self.known
            .insert(name.to_string(), Known::Content(disk_text.clone()));
        Ok(SaveOutcome::Conflict {
            copy_name,
            disk_text,
        })
    }

    pub fn create(&mut self, name: &str) -> Result<(), String> {
        validate_new_name(name)?;
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
        if from == to {
            return Ok(());
        }
        validate_new_name(to)?;
        // `to` may "exist" only because it is `from` itself on a
        // case-insensitive filesystem (changing only the letter case).
        // Anything else would be replaced by the rename, so refuse.
        let target = self.path(to);
        if target.exists() && !same_file(&self.path(from), &target) {
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

    fn icloud_placeholder(&self, name: &str) -> PathBuf {
        self.dir.join(format!(".{name}.{EXTENSION}{ICLOUD_SUFFIX}"))
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

/// Room `free_name` may add for a numeric suffix (` 999`).
const SUFFIX_ROOM: usize = 4;

/// `<name> (conflict <stamp>)`, shortened to stay a valid name. The stamp
/// comes from the UI, so only digits, spaces and `-` are kept.
fn conflict_base(name: &str, stamp: &str) -> String {
    let stamp: String = stamp
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == ' ' || *c == '-')
        .take(16)
        .collect();
    let suffix = format!(" (conflict {})", stamp.trim());
    let budget = MAX_NAME_BYTES - SUFFIX_ROOM - suffix.len();
    let mut end = name.len().min(budget);
    while !name.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{suffix}", name[..end].trim_end())
}

/// Whether two paths are the same file (for case-only renames).
fn same_file(a: &Path, b: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        match (fs::metadata(a), fs::metadata(b)) {
            (Ok(a), Ok(b)) => a.dev() == b.dev() && a.ino() == b.ino(),
            _ => false,
        }
    }
    #[cfg(not(unix))]
    {
        match (fs::canonicalize(a), fs::canonicalize(b)) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        }
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

/// Suffix of iCloud Drive placeholders for files evicted from this Mac.
const ICLOUD_SUFFIX: &str = ".icloud";

/// Asks iCloud Drive to download `path` and waits up to 10 s for it.
#[cfg(target_os = "macos")]
fn download_from_icloud(path: &Path) -> Result<(), String> {
    let status = std::process::Command::new("brctl")
        .arg("download")
        .arg(path)
        .status()
        .map_err(|e| format!("could not run brctl: {e}"))?;
    if !status.success() {
        return Err(format!("brctl download failed ({status})"));
    }
    for _ in 0..100 {
        if path.exists() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    Err("the download did not finish within 10 seconds; try again".into())
}

#[cfg(not(target_os = "macos"))]
fn download_from_icloud(_path: &Path) -> Result<(), String> {
    Err("download it on a Mac first".into())
}

/// Checks a name for a new or renamed note: [`validate_name`], plus the
/// rules of Windows file names, so a note made on one system opens on every
/// system the notes folder is synced to. Existing notes are not held to these
/// rules; they can still be opened, saved and renamed.
pub fn validate_new_name(name: &str) -> Result<(), String> {
    validate_name(name)?;
    const FORBIDDEN: [char; 7] = ['<', '>', ':', '"', '|', '?', '*'];
    if name.contains(FORBIDDEN) {
        return Err(
            "A note name cannot contain < > : \" | ? * (Windows does not allow them).".into(),
        );
    }
    if name.ends_with('.') {
        return Err("A note name cannot end with '.'.".into());
    }
    // Windows device names, whatever the case and even with an extension.
    let stem = name
        .split('.')
        .next()
        .unwrap_or(name)
        .trim_end()
        .to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.len() == 4
            && stem.as_bytes()[3].is_ascii_digit()
            && stem.as_bytes()[3] != b'0');
    if reserved {
        return Err(format!("'{name}' is a reserved name on Windows."));
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

#[cfg(test)]
mod tests {
    use super::*;
    use notecal_env::fs::TEMP_PREFIX;
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
    fn rename_never_replaces_a_note_that_differs_only_in_case() {
        let (_dir, mut store) = store();
        store.create("budget").unwrap();
        store.save("budget", "lower", STAMP).unwrap();
        // On a case-insensitive filesystem (Windows, default macOS) they are
        // the same file, so creating the second one is refused.
        if store.create("Budget").is_err() {
            assert_eq!(read(&store, "Budget"), "lower");
            return;
        }
        store.save("Budget", "upper", STAMP).unwrap();
        // On a case-sensitive filesystem these are two notes; renaming one
        // onto the other must not destroy it.
        if store.path("budget") != store.path("Budget")
            && fs::read_to_string(store.path("Budget")).unwrap() == "upper"
        {
            assert!(store.rename("budget", "Budget").is_err());
            assert_eq!(read(&store, "Budget"), "upper");
            assert_eq!(read(&store, "budget"), "lower");
        }
    }

    #[test]
    fn rename_can_change_only_the_case() {
        let (_dir, mut store) = store();
        store.create("budget").unwrap();
        store.save("budget", "x", STAMP).unwrap();
        store.rename("budget", "Budget").unwrap();
        assert_eq!(read(&store, "Budget"), "x");
    }

    #[test]
    fn conflict_copy_fits_long_note_names() {
        let (_dir, mut store) = store();
        let name = "n".repeat(MAX_NAME_BYTES);
        store.create(&name).unwrap();
        write_externally(&store, &name, "sync");
        let SaveOutcome::Conflict { copy_name, .. } = store.save(&name, "mine", STAMP).unwrap()
        else {
            panic!("expected a conflict");
        };
        assert!(copy_name.len() <= MAX_NAME_BYTES);
        assert!(copy_name.ends_with("(conflict 2026-11-05 1830)"));
        assert_eq!(read(&store, &copy_name), "mine");
    }

    #[test]
    fn conflict_stamp_from_the_ui_is_sanitised() {
        let (_dir, mut store) = store();
        store.create("A").unwrap();
        write_externally(&store, "A", "sync");
        let outcome = store.save("A", "mine", "../../etc/x\n\0 2026").unwrap();
        let SaveOutcome::Conflict { copy_name, .. } = outcome else {
            panic!("expected a conflict");
        };
        assert!(validate_name(&copy_name).is_ok(), "{copy_name}");
        assert_eq!(read(&store, &copy_name), "mine");
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
    fn new_names_follow_windows_rules_but_old_notes_still_open() {
        for name in [
            "a:b", "what?", "x*y", "a<b", "q\"", "pipe|", "end.", "CON", "nul", "com1", "Lpt9.x",
        ] {
            assert!(
                validate_new_name(name).is_err(),
                "{name:?} should be refused"
            );
        }
        for name in [
            "Budget",
            "Trip 2026",
            "a.b",
            "COM0",
            "Console",
            "COM10",
            "budget (conflict 2026-11-18 1030)",
        ] {
            assert!(
                validate_new_name(name).is_ok(),
                "{name:?} should be allowed"
            );
        }
        let (dir, mut store) = store();
        assert!(store.create("11:00 standup").is_err());
        // A note named with ':' can only exist where the filesystem allows it
        // (made on Linux or macOS); it still opens and can be renamed away.
        #[cfg(unix)]
        {
            std::fs::write(dir.path().join("10:30 standup.txt"), "1 + 1").unwrap();
            assert_eq!(store.open("10:30 standup").unwrap(), "1 + 1");
            assert!(store.rename("10:30 standup", "10:30 standup").is_ok());
            assert!(store.rename("10:30 standup", "1030 standup").is_ok());
        }
        #[cfg(not(unix))]
        let _ = dir;
    }

    #[test]
    fn icloud_placeholders_are_listed_as_their_notes() {
        let (dir, mut store) = store();
        std::fs::write(dir.path().join(".Budget.txt.icloud"), "plist").unwrap();
        std::fs::write(dir.path().join("Trip.txt"), "").unwrap();
        std::fs::write(dir.path().join(".Trip.txt.icloud"), "plist").unwrap();
        std::fs::write(dir.path().join(".other.icloud"), "").unwrap();
        let names: Vec<String> = store.list().unwrap().into_iter().map(|n| n.name).collect();
        assert_eq!(names.len(), 2, "{names:?}");
        assert!(names.contains(&"Budget".to_string()) && names.contains(&"Trip".to_string()));
        #[cfg(not(target_os = "macos"))]
        assert!(store.open("Budget").unwrap_err().contains("iCloud Drive"));
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
