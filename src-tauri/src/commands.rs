//! Note and UI-state commands, plus the bridge from file-watch events to the
//! UI. Storage rules live in `notes.rs`; this layer only locks and routes.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::notes::{DiskChange, NoteInfo, NoteStore, SaveOutcome};
use crate::state::{self, UiState};
use crate::theme::{self, Theme};

/// Shown in the first note on a fresh install (the PRD's example note).
const SCRATCH_NAME: &str = "Scratch";
const SCRATCH_TEXT: &str = "\
# Monthly budget
rent = 4,500,000
electricity = 350,000
internet = 450,000
total
20% of prev

# Conversions
5 km in miles
3 days 4 hours in minutes
1.5 GB in MB
0xff in binary
";

/// Emitted when the open note's file was changed or deleted by something
/// other than this app.
pub const NOTE_CHANGED_ON_DISK: &str = "note-changed-on-disk";
/// Emitted when any note file was added, removed or changed.
pub const NOTES_LIST_CHANGED: &str = "notes-list-changed";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskChangeEvent {
    pub name: String,
    /// `"changed"` or `"deleted"`.
    pub kind: &'static str,
}

struct Inner {
    store: NoteStore,
    /// The note the editor shows, for routing watch events.
    open: Option<String>,
}

pub struct Notes {
    inner: Mutex<Inner>,
    state_path: PathBuf,
}

impl Notes {
    /// Opens the notes folder and seeds `Scratch` when it holds no notes.
    pub fn new(notes_dir: PathBuf, state_path: PathBuf) -> Result<Self, String> {
        let mut store = NoteStore::new(notes_dir)?;
        if store.list()?.is_empty() {
            store.create(SCRATCH_NAME)?;
            store.save(SCRATCH_NAME, SCRATCH_TEXT, "")?;
        }
        Ok(Self {
            inner: Mutex::new(Inner { store, open: None }),
            state_path,
        })
    }

    pub fn dir(&self) -> PathBuf {
        self.lock().store.dir().to_path_buf()
    }

    /// A panic while holding the lock cannot leave the store inconsistent
    /// (each operation updates its record after the file write), so a
    /// poisoned lock is recovered rather than failing every later command.
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Handles watch events for the notes folder.
    pub fn on_paths_changed(&self, app: &AppHandle, paths: &[PathBuf]) {
        let mut inner = self.lock();
        let names: Vec<String> = paths
            .iter()
            .filter_map(|p| inner.store.name_of(p))
            .collect();
        if names.is_empty() {
            return;
        }
        if let Some(open) = inner.open.clone().filter(|o| names.contains(o)) {
            let kind = match inner.store.disk_changed(&open) {
                Ok(DiskChange::Changed(_)) => Some("changed"),
                Ok(DiskChange::Deleted) => {
                    inner.store.mark_missing(&open);
                    Some("deleted")
                }
                Ok(DiskChange::Unchanged) => None,
                Err(e) => {
                    eprintln!("notecal: {e}");
                    None
                }
            };
            if let Some(kind) = kind {
                emit(
                    app,
                    NOTE_CHANGED_ON_DISK,
                    DiskChangeEvent { name: open, kind },
                );
            }
        }
        drop(inner);
        emit(app, NOTES_LIST_CHANGED, ());
    }
}

fn emit(app: &AppHandle, event: &str, payload: impl Serialize + Clone) {
    if let Err(e) = app.emit(event, payload) {
        eprintln!("notecal: could not send {event}: {e}");
    }
}

pub fn notes_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("notecal").join("notes")
}

pub fn state_path(config_dir: &Path) -> PathBuf {
    config_dir.join("notecal").join("state.json")
}

#[tauri::command]
pub fn list_notes(notes: State<'_, Notes>) -> Result<Vec<NoteInfo>, String> {
    notes.lock().store.list()
}

#[tauri::command]
pub fn open_note(name: String, notes: State<'_, Notes>) -> Result<String, String> {
    let mut inner = notes.lock();
    let text = inner.store.open(&name)?;
    inner.open = Some(name);
    Ok(text)
}

#[tauri::command]
pub fn save_note(
    name: String,
    text: String,
    stamp: String,
    notes: State<'_, Notes>,
) -> Result<SaveOutcome, String> {
    notes.lock().store.save(&name, &text, &stamp)
}

#[tauri::command]
pub fn create_note(name: String, notes: State<'_, Notes>) -> Result<(), String> {
    notes.lock().store.create(&name)
}

#[tauri::command]
pub fn rename_note(from: String, to: String, notes: State<'_, Notes>) -> Result<(), String> {
    let mut inner = notes.lock();
    inner.store.rename(&from, &to)?;
    if inner.open.as_deref() == Some(from.as_str()) {
        inner.open = Some(to);
    }
    Ok(())
}

#[tauri::command]
pub fn delete_note(name: String, notes: State<'_, Notes>) -> Result<(), String> {
    let mut inner = notes.lock();
    inner.store.delete(&name)?;
    if inner.open.as_deref() == Some(name.as_str()) {
        inner.open = None;
    }
    Ok(())
}

/// Where "auto" looks for Omarchy's current theme.
pub struct ThemeSource {
    pub omarchy_current: PathBuf,
}

impl ThemeSource {
    /// Handles watch events in Omarchy's current folder.
    pub fn on_paths_changed(&self, app: &AppHandle, paths: &[PathBuf]) {
        if paths
            .iter()
            .any(|p| theme::is_theme_swap(&self.omarchy_current, p))
        {
            emit(
                app,
                theme::THEME_CHANGED,
                theme::load_auto(&self.omarchy_current),
            );
        }
    }
}

/// The palette to use, or `None` for the built-in one.
#[tauri::command]
pub fn get_theme(source: State<'_, ThemeSource>) -> Option<Theme> {
    theme::load_auto(&source.omarchy_current)
}

#[tauri::command]
pub fn load_state(notes: State<'_, Notes>) -> UiState {
    state::load(&notes.state_path)
}

#[tauri::command]
pub fn save_state(ui_state: UiState, notes: State<'_, Notes>) -> Result<(), String> {
    state::save(&notes.state_path, &ui_state)
}
