//! User settings in `settings.toml` (PRD A6). Every key is optional; a bad
//! value is reported and replaced by its default, never fatal, and reading
//! never rewrites the user's file.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::notes::write_atomic;

/// Emitted with the new `SettingsView` whenever settings change.
pub const SETTINGS_CHANGED: &str = "settings-changed";

const HEADER: &str = "\
# notecal settings. Every key is optional; see docs/linux-setup.md.
# Saving from the Settings screen rewrites this file (comments are not kept).
";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    En,
    Id,
}

impl From<Locale> for notecal_core::Locale {
    fn from(locale: Locale) -> Self {
        match locale {
            Locale::En => Self::En,
            Locale::Id => Self::Id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSettings {
    /// Global shortcut (A1); not used on Wayland, where `notecal --toggle` is bound instead.
    pub shortcut: String,
    /// `auto`, `light`, `dark`, or the name of a file in the themes folder.
    pub theme: String,
    /// `auto` = the system monospace font (follows Omarchy's font).
    pub font_family: String,
    pub font_size: u8,
    /// Decimal places shown for non-currency results.
    pub precision: u8,
    pub locale: Locale,
    /// Empty = the default notes folder.
    pub notes_dir: String,
    /// False = never fetch exchange rates; use the cached ones only.
    pub currency_refresh: bool,
    pub always_on_top: bool,
    /// Closing the window hides it; Quit from the tray.
    pub background: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            shortcut: "Alt+Space".into(),
            theme: "auto".into(),
            font_family: "auto".into(),
            font_size: 15,
            precision: 6,
            locale: Locale::En,
            notes_dir: String::new(),
            currency_refresh: true,
            always_on_top: false,
            background: true,
        }
    }
}

pub const FONT_SIZES: std::ops::RangeInclusive<u8> = 8..=40;
pub const PRECISIONS: std::ops::RangeInclusive<u8> = 0..=20;

impl AppSettings {
    /// Engine settings derived from these.
    pub fn engine(&self) -> notecal_core::Settings {
        notecal_core::Settings {
            locale: self.locale.into(),
            precision: self.precision,
        }
    }
}

/// Settings plus any problems found while reading them, for the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SettingsView {
    pub settings: AppSettings,
    pub problems: Vec<String>,
}

/// Parses settings, taking each valid key and reporting the rest.
pub fn parse(text: &str) -> SettingsView {
    let mut settings = AppSettings::default();
    let mut problems = Vec::new();
    let table: toml::Table = match text.parse() {
        Ok(table) => table,
        Err(e) => {
            problems.push(format!(
                "settings.toml is not valid TOML, so defaults are used: {e}"
            ));
            return SettingsView { settings, problems };
        }
    };
    for (key, value) in &table {
        let bad = |problems: &mut Vec<String>, expected: &str| {
            problems.push(format!(
                "'{key}' should be {expected}; the default is used."
            ));
        };
        match key.as_str() {
            "shortcut" => match value.as_str().filter(|s| !s.trim().is_empty()) {
                Some(s) => settings.shortcut = s.trim().to_string(),
                None => bad(&mut problems, "a shortcut such as \"Alt+Space\""),
            },
            // A theme name becomes a file name in the themes folder.
            "theme" => match value
                .as_str()
                .map(str::trim)
                .filter(|s| !s.is_empty() && !s.starts_with('.') && !s.contains(['/', '\\']))
            {
                Some(s) => settings.theme = s.to_string(),
                None => bad(&mut problems, "auto, light, dark or a theme file name"),
            },
            "font_family" => match value.as_str().filter(|s| !s.trim().is_empty()) {
                Some(s) => settings.font_family = s.trim().to_string(),
                None => bad(&mut problems, "a font name or \"auto\""),
            },
            "font_size" => match small_int(value, &FONT_SIZES) {
                Some(n) => settings.font_size = n,
                None => bad(&mut problems, "a number from 8 to 40"),
            },
            "precision" => match small_int(value, &PRECISIONS) {
                Some(n) => settings.precision = n,
                None => bad(&mut problems, "a number from 0 to 20"),
            },
            "locale" => match value.as_str() {
                Some("en") => settings.locale = Locale::En,
                Some("id") => settings.locale = Locale::Id,
                _ => bad(&mut problems, "\"en\" or \"id\""),
            },
            // Absolute only: a relative path would depend on where notecal was started.
            "notes_dir" => match value
                .as_str()
                .map(str::trim)
                .filter(|s| s.is_empty() || Path::new(s).is_absolute())
            {
                Some(s) => settings.notes_dir = s.to_string(),
                None => bad(
                    &mut problems,
                    "an absolute folder path such as \"/home/you/Notes\", or \"\" for the default",
                ),
            },
            "currency_refresh" | "always_on_top" | "background" => match value.as_bool() {
                Some(b) => match key.as_str() {
                    "currency_refresh" => settings.currency_refresh = b,
                    "always_on_top" => settings.always_on_top = b,
                    _ => settings.background = b,
                },
                None => bad(&mut problems, "true or false"),
            },
            _ => problems.push(format!("'{key}' is not a notecal setting and is ignored.")),
        }
    }
    SettingsView { settings, problems }
}

fn small_int(value: &toml::Value, range: &std::ops::RangeInclusive<u8>) -> Option<u8> {
    value
        .as_integer()
        .and_then(|n| u8::try_from(n).ok())
        .filter(|n| range.contains(n))
}

/// Reads the file; a missing file means defaults with no problems.
pub fn load(path: &Path) -> SettingsView {
    match fs::read_to_string(path) {
        Ok(text) => parse(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => SettingsView {
            settings: AppSettings::default(),
            problems: Vec::new(),
        },
        Err(e) => SettingsView {
            settings: AppSettings::default(),
            problems: vec![format!(
                "Could not read settings.toml, so defaults are used: {e}"
            )],
        },
    }
}

/// Checks settings coming from the UI with the same rules as the file.
pub fn validate(settings: &AppSettings) -> Result<(), String> {
    let text = toml::to_string(settings).map_err(|e| e.to_string())?;
    let view = parse(&text);
    match view.problems.first() {
        Some(problem) => Err(problem.clone()),
        None => Ok(()),
    }
}

pub fn save(path: &Path, settings: &AppSettings) -> Result<(), String> {
    validate(settings)?;
    let dir = path
        .parent()
        .ok_or_else(|| format!("{} has no parent folder", path.display()))?;
    fs::create_dir_all(dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    let body = toml::to_string(settings).map_err(|e| e.to_string())?;
    write_atomic(dir, path, &format!("{HEADER}{body}"))
        .map_err(|e| format!("Could not save settings: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn missing_file_gives_defaults_without_problems() {
        let dir = TempDir::new().unwrap();
        let view = load(&dir.path().join("settings.toml"));
        assert_eq!(view.settings, AppSettings::default());
        assert!(view.problems.is_empty());
    }

    #[test]
    fn partial_file_keeps_defaults_for_the_rest() {
        let view = parse("precision = 2\nlocale = \"id\"\n");
        assert_eq!(view.settings.precision, 2);
        assert_eq!(view.settings.locale, Locale::Id);
        assert_eq!(view.settings.font_size, 15);
        assert!(view.problems.is_empty());
    }

    #[test]
    fn invalid_values_fall_back_and_are_reported() {
        let view = parse(
            "precision = 99\nfont_size = \"big\"\nlocale = \"fr\"\nbackground = 1\ncolour = \"red\"\ntheme = \"\"\n",
        );
        assert_eq!(view.settings, AppSettings::default());
        assert_eq!(view.problems.len(), 6, "{:?}", view.problems);
        assert!(view.problems.iter().any(|p| p.contains("'colour'")));
    }

    #[test]
    fn notes_dir_must_be_absolute_and_theme_a_plain_name() {
        for bad in [
            "notes_dir = \"notes\"",
            "notes_dir = \"~/notes\"",
            "theme = \"../../etc/x\"",
            "theme = \"/etc/x\"",
            "theme = \".hidden\"",
        ] {
            let view = parse(bad);
            assert_eq!(view.settings, AppSettings::default(), "{bad}");
            assert_eq!(view.problems.len(), 1, "{bad}");
        }
        let ok = parse("notes_dir = \"/home/me/Notes\"\ntheme = \"my theme\"");
        assert!(ok.problems.is_empty());
    }

    #[test]
    fn broken_toml_gives_defaults_and_one_problem() {
        let view = parse("precision = = 3");
        assert_eq!(view.settings, AppSettings::default());
        assert_eq!(view.problems.len(), 1);
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("sub/settings.toml");
        let settings = AppSettings {
            theme: "my theme".into(),
            font_family: "JetBrains Mono".into(),
            precision: 3,
            locale: Locale::Id,
            notes_dir: "/tmp/notes".into(),
            currency_refresh: false,
            ..AppSettings::default()
        };
        save(&path, &settings).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("# notecal settings."));
        let view = load(&path);
        assert_eq!(view.settings, settings);
        assert!(view.problems.is_empty());
    }

    #[test]
    fn save_refuses_invalid_settings() {
        let dir = TempDir::new().unwrap();
        let settings = AppSettings {
            precision: 50,
            ..AppSettings::default()
        };
        assert!(save(&dir.path().join("settings.toml"), &settings).is_err());
        assert!(!dir.path().join("settings.toml").exists());
    }

    #[test]
    fn engine_settings_follow_locale_and_precision() {
        let settings = AppSettings {
            precision: 2,
            locale: Locale::Id,
            ..AppSettings::default()
        };
        let engine = settings.engine();
        assert_eq!(engine.precision, 2);
        assert_eq!(engine.locale, notecal_core::Locale::Id);
    }
}
