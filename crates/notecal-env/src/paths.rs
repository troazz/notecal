//! Where notecal keeps its files. The app gets the base folders from Tauri,
//! the CLI from [`system_dirs`]; both come from the `dirs` crate, so they
//! agree.

use std::path::{Path, PathBuf};

use crate::settings::AppSettings;

/// The user's config, data and cache folders (`~/.config`, `~/.local/share`,
/// `~/.cache` on Linux; `~/Library/...` on macOS).
pub struct SystemDirs {
    pub config: PathBuf,
    pub data: PathBuf,
    pub cache: PathBuf,
}

/// The base folders for this user, if the system has them.
#[must_use]
pub fn system_dirs() -> Option<SystemDirs> {
    Some(SystemDirs {
        config: dirs::config_dir()?,
        data: dirs::data_dir()?,
        cache: dirs::cache_dir()?,
    })
}

#[must_use]
pub fn notes_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("notecal").join("notes")
}

#[must_use]
pub fn state_path(config_dir: &Path) -> PathBuf {
    config_dir.join("notecal").join("state.json")
}

#[must_use]
pub fn settings_path(config_dir: &Path) -> PathBuf {
    config_dir.join("notecal").join("settings.toml")
}

/// The notes folder: the `notes_dir` setting, or the default under `data_dir`.
#[must_use]
pub fn resolve_notes_dir(settings: &AppSettings, data_dir: &Path) -> PathBuf {
    if settings.notes_dir.is_empty() {
        notes_dir(data_dir)
    } else {
        PathBuf::from(&settings.notes_dir)
    }
}

#[must_use]
pub fn rates_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join("notecal").join("rates.json")
}

/// The definitions file, next to settings.toml.
#[must_use]
pub fn definitions_path(config_dir: &Path) -> PathBuf {
    settings_path(config_dir).with_file_name(crate::settings::DEFINITIONS_FILE)
}
