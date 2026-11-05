//! UI state kept between launches (PRD N4): the last open note and its
//! cursor. Stored as `state.json` next to the settings file.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::notes::write_atomic;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UiState {
    pub last_note: Option<String>,
    /// Cursor offset in the last note, in UTF-16 code units (the editor's).
    pub cursor: usize,
}

/// Reads the state; a missing or unreadable file means a fresh start.
pub fn load(path: &Path) -> UiState {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
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
        };
        save(&path, &state).unwrap();
        assert_eq!(load(&path), state);
    }

    #[test]
    fn missing_or_corrupt_file_gives_defaults() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.json");
        assert_eq!(load(&path), UiState::default());
        fs::write(&path, "{ not json").unwrap();
        assert_eq!(load(&path), UiState::default());
        fs::write(&path, r#"{"lastNote":"A","extra":1}"#).unwrap();
        assert_eq!(load(&path).last_note.as_deref(), Some("A"));
    }
}
