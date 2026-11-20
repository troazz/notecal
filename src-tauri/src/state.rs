//! UI state kept between launches (PRD N4): the last open note and its
//! cursor, and whether the first-run onboarding is done (A8). Stored as
//! `state.json` next to the settings file.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use notecal_env::fs::write_atomic;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UiState {
    pub last_note: Option<String>,
    /// Cursor offset in the last note, in UTF-16 code units (the editor's).
    pub cursor: usize,
    pub onboarding: Onboarding,
}

/// First-run onboarding (A8). A file without the field is from before it
/// existed, so that user is not new: the default is `Done`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Onboarding {
    /// A fresh install that hasn't finished or skipped the onboarding yet.
    Pending,
    #[default]
    Done,
}

/// Reads the state. No file means a fresh install (onboarding pending); an
/// unreadable one means defaults, without bringing the onboarding back.
pub fn load(path: &Path) -> UiState {
    match fs::read_to_string(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => UiState {
            onboarding: Onboarding::Pending,
            ..UiState::default()
        },
        result => result
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default(),
    }
}

/// Saves the note and cursor from the UI; the onboarding state on disk is
/// kept, since the UI doesn't send it.
pub fn save_position(path: &Path, from_ui: &UiState) -> Result<(), String> {
    let state = UiState {
        onboarding: load(path).onboarding,
        ..from_ui.clone()
    };
    save(path, &state)
}

/// Marks the onboarding done, keeping the rest.
pub fn finish_onboarding(path: &Path) -> Result<(), String> {
    let state = UiState {
        onboarding: Onboarding::Done,
        ..load(path)
    };
    save(path, &state)
}

pub fn save(path: &Path, state: &UiState) -> Result<(), String> {
    let dir = path
        .parent()
        .ok_or_else(|| format!("{} has no parent folder", path.display()))?;
    fs::create_dir_all(dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    let text = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    write_atomic(dir, path, &text).map_err(|e| format!("Could not save the app state: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn round_trips() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("sub/state.json");
        let state = UiState {
            last_note: Some("Budget".into()),
            cursor: 42,
            onboarding: Onboarding::Done,
        };
        save(&path, &state).unwrap();
        assert_eq!(load(&path), state);
    }

    #[test]
    fn missing_or_corrupt_file_gives_defaults() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.json");
        assert_eq!(load(&path).onboarding, Onboarding::Pending);
        fs::write(&path, "{ not json").unwrap();
        assert_eq!(load(&path), UiState::default());
        fs::write(&path, r#"{"lastNote":"A","extra":1}"#).unwrap();
        assert_eq!(load(&path).last_note.as_deref(), Some("A"));
    }

    #[test]
    fn onboarding_is_pending_only_on_a_fresh_install() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.json");
        assert_eq!(load(&path).onboarding, Onboarding::Pending);
        // A state file from before onboarding existed: an existing user.
        fs::write(&path, r#"{"lastNote":"A","cursor":3}"#).unwrap();
        assert_eq!(load(&path).onboarding, Onboarding::Done);
    }

    #[test]
    fn saving_the_position_keeps_the_onboarding_state() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.json");
        // The UI sends no onboarding field, which reads as Done.
        let from_ui = UiState {
            last_note: Some("Scratch".into()),
            cursor: 5,
            ..UiState::default()
        };
        save_position(&path, &from_ui).unwrap();
        assert_eq!(
            load(&path).onboarding,
            Onboarding::Pending,
            "quit part-way: shown again"
        );
        assert_eq!(load(&path).last_note.as_deref(), Some("Scratch"));

        finish_onboarding(&path).unwrap();
        save_position(&path, &from_ui).unwrap();
        let state = load(&path);
        assert_eq!(state.onboarding, Onboarding::Done);
        assert_eq!(state.cursor, 5);
    }
}
