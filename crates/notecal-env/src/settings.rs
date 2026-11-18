//! User settings in `settings.toml` (PRD A6). Every key is optional; a bad
//! value is reported and replaced by its default, never fatal, and reading
//! never rewrites the user's file.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::fs::write_atomic;

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

/// The interface language; `Auto` follows the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Auto,
    En,
    Id,
}

/// A language the interface is translated into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UiLanguage {
    En,
    Id,
}

impl From<UiLanguage> for notecal_core::Language {
    fn from(language: UiLanguage) -> Self {
        match language {
            UiLanguage::En => Self::En,
            UiLanguage::Id => Self::Id,
        }
    }
}

impl Language {
    /// The language to show, given the system locale (`id-ID`, `en_US.UTF-8`, …).
    #[must_use]
    pub fn resolve(self, system_locale: Option<&str>) -> UiLanguage {
        match self {
            Language::En => UiLanguage::En,
            Language::Id => UiLanguage::Id,
            Language::Auto => {
                let code = system_locale.unwrap_or("").to_ascii_lowercase();
                // `in` is the old code for Indonesian, still used by some systems.
                if code.starts_with("id") || code.starts_with("in_") || code.starts_with("in-") {
                    UiLanguage::Id
                } else {
                    UiLanguage::En
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSettings {
    /// Interface language (PRD M9).
    pub language: Language,
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
    /// Pixels per inch for `px`.
    pub css_ppi: u16,
    /// Pixels per `em` and `rem`.
    pub css_em_px: u16,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: Language::Auto,
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
            css_ppi: 96,
            css_em_px: 16,
        }
    }
}

pub const FONT_SIZES: std::ops::RangeInclusive<u8> = 8..=40;
pub const PRECISIONS: std::ops::RangeInclusive<u8> = 0..=20;
pub const CSS_PPIS: std::ops::RangeInclusive<u16> = 1..=1000;
pub const CSS_EM_PXS: std::ops::RangeInclusive<u16> = 1..=200;

/// The definitions file, next to settings.toml.
pub const DEFINITIONS_FILE: &str = "definitions.txt";

/// Reads the definitions file; missing or unreadable means none.
pub fn load_definitions(path: &Path) -> String {
    match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => {
            eprintln!("notecal: could not read {}: {e}", path.display());
            String::new()
        }
    }
}

/// Lines of the definitions file that could not be used, for the Settings screen.
pub fn definition_problems(settings: &AppSettings, definitions: &str) -> Vec<String> {
    notecal_core::check_definitions(&settings.engine(definitions, UiLanguage::En), None)
        .into_iter()
        .map(|e| format!("{DEFINITIONS_FILE} line {}: {}", e.line, e.message))
        .collect()
}

impl AppSettings {
    /// Engine settings derived from these, the definitions file's text and
    /// the resolved interface language (for weekday and month names).
    pub fn engine(&self, definitions: &str, language: UiLanguage) -> notecal_core::Settings {
        notecal_core::Settings {
            locale: self.locale.into(),
            precision: self.precision,
            language: language.into(),
            css_ppi: self.css_ppi,
            css_em_px: self.css_em_px,
            definitions: definitions.to_string(),
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
            "language" => match value.as_str() {
                Some("auto") => settings.language = Language::Auto,
                Some("en") => settings.language = Language::En,
                Some("id") => settings.language = Language::Id,
                _ => bad(&mut problems, "\"auto\", \"en\" or \"id\""),
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
            "css_ppi" => match medium_int(value, &CSS_PPIS) {
                Some(n) => settings.css_ppi = n,
                None => bad(&mut problems, "a number from 1 to 1000"),
            },
            "css_em_px" => match medium_int(value, &CSS_EM_PXS) {
                Some(n) => settings.css_em_px = n,
                None => bad(&mut problems, "a number from 1 to 200"),
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

fn medium_int(value: &toml::Value, range: &std::ops::RangeInclusive<u16>) -> Option<u16> {
    value
        .as_integer()
        .and_then(|n| u16::try_from(n).ok())
        .filter(|n| range.contains(n))
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
        // An absolute path on this platform (Windows needs a drive letter).
        let absolute = if cfg!(windows) {
            r"C:\Users\me\Notes"
        } else {
            "/home/me/Notes"
        };
        let ok = parse(&format!("notes_dir = '{absolute}'\ntheme = \"my theme\""));
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
            notes_dir: dir.path().join("notes").to_string_lossy().into_owned(),
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
        let engine = settings.engine("vat = 11%", UiLanguage::Id);
        assert_eq!(engine.language, notecal_core::Language::Id);
        assert_eq!(engine.precision, 2);
        assert_eq!(engine.locale, notecal_core::Locale::Id);
        assert_eq!(engine.css_ppi, 96);
        assert_eq!(engine.definitions, "vat = 11%");
    }

    #[test]
    fn css_settings_are_range_checked() {
        let view = parse("css_ppi = 72\ncss_em_px = 10\n");
        assert_eq!((view.settings.css_ppi, view.settings.css_em_px), (72, 10));
        assert!(view.problems.is_empty());
        let view = parse("css_ppi = 0\ncss_em_px = 500\n");
        assert_eq!((view.settings.css_ppi, view.settings.css_em_px), (96, 16));
        assert_eq!(view.problems.len(), 2);
    }

    #[test]
    fn definitions_are_optional_and_their_errors_are_listed() {
        let dir = TempDir::new().unwrap();
        assert_eq!(load_definitions(&dir.path().join(DEFINITIONS_FILE)), "");
        let problems = definition_problems(&AppSettings::default(), "vat = 11%\n1 + 1\n");
        assert_eq!(problems.len(), 1);
        assert!(
            problems[0].starts_with("definitions.txt line 2:"),
            "{problems:?}"
        );
    }

    #[test]
    fn language_follows_the_system_when_auto() {
        assert_eq!(Language::Auto.resolve(Some("id-ID")), UiLanguage::Id);
        assert_eq!(Language::Auto.resolve(Some("in_ID.UTF-8")), UiLanguage::Id);
        assert_eq!(Language::Auto.resolve(Some("en_US.UTF-8")), UiLanguage::En);
        assert_eq!(Language::Auto.resolve(Some("india")), UiLanguage::En);
        assert_eq!(Language::Auto.resolve(None), UiLanguage::En);
        assert_eq!(Language::Id.resolve(Some("en-US")), UiLanguage::Id);
        let view = parse("language = \"id\"\n");
        assert_eq!(view.settings.language, Language::Id);
        assert!(parse("language = \"fr\"\n").problems.len() == 1);
    }
}
