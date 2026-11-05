//! Colour themes in Omarchy's `colors.toml` format. In "auto" mode the app
//! follows Omarchy's current theme when it exists and repaints when
//! `omarchy theme set` swaps it; otherwise the UI uses its built-in light or
//! dark palette (`ui/src/theme.ts`).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// Palette keys the UI uses. Others in the file are ignored.
pub const KEYS: &[&str] = &[
    "background",
    "lighter_background",
    "foreground",
    "accent",
    "selection",
    "muted",
    "red",
    "yellow",
    "orange",
    "green",
    "cyan",
    "blue",
    "magenta",
];

/// Emitted with an `Option<Theme>` when the followed theme changes
/// (`None` means "use the built-in palette").
pub const THEME_CHANGED: &str = "theme-changed";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Light,
    Dark,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Theme {
    pub name: String,
    pub mode: Mode,
    /// Valid `#rrggbb` colours by key. Missing keys fall back to the
    /// built-in palette of the same mode, in the UI.
    pub colors: BTreeMap<String, String>,
}

/// Parses a `colors.toml`-style palette. Unknown keys are ignored and
/// invalid colours are dropped (so they fall back), but text that is not
/// TOML at all is an error.
pub fn parse(name: &str, text: &str) -> Result<Theme, String> {
    let table: toml::Table = text
        .parse()
        .map_err(|e| format!("The theme '{name}' is not valid TOML: {e}"))?;
    let colors: BTreeMap<String, String> = KEYS
        .iter()
        .filter_map(|key| {
            let value = table.get(*key)?.as_str()?;
            normalize_hex(value).map(|hex| ((*key).to_string(), hex))
        })
        .collect();
    let mode = match table.get("mode").and_then(toml::Value::as_str) {
        Some("light") => Mode::Light,
        Some("dark") => Mode::Dark,
        _ => match colors.get("background") {
            Some(bg) if luminance(bg) > 0.5 => Mode::Light,
            _ => Mode::Dark,
        },
    };
    Ok(Theme {
        name: name.to_string(),
        mode,
        colors,
    })
}

/// `#RGB`, `#RRGGBB` or `#RRGGBBAA` → lowercase `#rrggbb` (alpha dropped).
fn normalize_hex(value: &str) -> Option<String> {
    let hex = value.trim().strip_prefix('#')?;
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let six = match hex.len() {
        3 => hex.chars().flat_map(|c| [c, c]).collect(),
        6 | 8 => hex[..6].to_string(),
        _ => return None,
    };
    Some(format!("#{}", six.to_ascii_lowercase()))
}

/// Relative luminance (WCAG) of a normalized `#rrggbb` colour.
fn luminance(hex: &str) -> f64 {
    let channel = |i: usize| {
        let v = f64::from(u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0)) / 255.0;
        if v <= 0.039_28 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5)
}

/// Omarchy's "current" folder: holds `theme/` (swapped as a whole on
/// `omarchy theme set`) and `theme.name`.
pub fn omarchy_current_dir(home: &Path) -> PathBuf {
    home.join(".local/state/omarchy/current")
}

/// The theme "auto" resolves to: Omarchy's current palette if present.
/// `None` means the built-in palette; a broken Omarchy file is logged and
/// also falls back, so a bad theme never leaves the app unreadable.
pub fn load_auto(omarchy_current: &Path) -> Option<Theme> {
    let text = fs::read_to_string(omarchy_current.join("theme/colors.toml")).ok()?;
    let name = fs::read_to_string(omarchy_current.join("theme.name"))
        .map(|n| n.trim().to_string())
        .unwrap_or_else(|_| "omarchy".into());
    parse(&format!("Omarchy: {name}"), &text)
        .inspect_err(|e| eprintln!("notecal: {e}"))
        .ok()
}

/// The user's theme files (`<name>.toml` in `dir`), by name, sorted.
pub fn list_user_themes(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "toml"))
                .filter_map(|p| p.file_stem()?.to_str().map(str::to_string))
                .filter(|name| !name.starts_with('.'))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// The palette for a `theme` setting: `auto` (Omarchy, else `None` for the
/// built-in palette of the system mode), `light`/`dark` (built-in, forced
/// mode), or the name of a user theme file.
pub fn resolve(
    choice: &str,
    omarchy_current: &Path,
    themes_dir: &Path,
) -> Result<Option<Theme>, String> {
    let builtin = |mode, name: &str| Theme {
        name: name.into(),
        mode,
        colors: BTreeMap::new(),
    };
    match choice {
        "auto" => Ok(load_auto(omarchy_current)),
        "light" => Ok(Some(builtin(Mode::Light, "Built-in light"))),
        "dark" => Ok(Some(builtin(Mode::Dark, "Built-in dark"))),
        name => {
            let path = user_theme_path(themes_dir, name);
            let text = fs::read_to_string(&path).map_err(|e| {
                format!(
                    "Could not read the theme '{name}' ({}): {e}",
                    path.display()
                )
            })?;
            parse(name, &text).map(Some)
        }
    }
}

pub fn user_theme_path(themes_dir: &Path, name: &str) -> PathBuf {
    themes_dir.join(format!("{name}.toml"))
}

/// Whether a watch event in Omarchy's current folder may mean a new theme.
pub fn is_theme_swap(omarchy_current: &Path, path: &Path) -> bool {
    path == omarchy_current.join("theme") || path == omarchy_current.join("theme.name")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const ETHEREAL: &str = include_str!("../testdata/omarchy-ethereal.toml");

    #[test]
    fn parses_an_omarchy_palette() {
        let theme = parse("ethereal", ETHEREAL).unwrap();
        assert_eq!(theme.mode, Mode::Dark);
        assert_eq!(theme.colors["background"], "#060b1e");
        assert_eq!(theme.colors["yellow"], "#e9bb4f");
        assert_eq!(theme.colors.len(), KEYS.len());
        // Keys outside KEYS (bright_*, brown, …) are ignored.
        assert!(!theme.colors.contains_key("bright_red"));
    }

    #[test]
    fn invalid_colours_are_dropped_and_missing_keys_left_out() {
        let theme = parse(
            "t",
            "mode = \"light\"\nforeground = \"#ABC\"\nred = \"not a colour\"\nblue = \"#12345\"\naccent = \"#11223344\"",
        )
        .unwrap();
        assert_eq!(theme.mode, Mode::Light);
        assert_eq!(
            theme.colors,
            BTreeMap::from([
                ("accent".to_string(), "#112233".to_string()),
                ("foreground".to_string(), "#aabbcc".to_string()),
            ])
        );
    }

    #[test]
    fn mode_is_inferred_from_the_background_when_missing() {
        assert_eq!(
            parse("t", "background = \"#fafafa\"").unwrap().mode,
            Mode::Light
        );
        assert_eq!(
            parse("t", "background = \"#101010\"").unwrap().mode,
            Mode::Dark
        );
        assert_eq!(parse("t", "").unwrap().mode, Mode::Dark);
    }

    #[test]
    fn non_toml_is_an_error() {
        assert!(parse("t", "this is = = not toml").is_err());
    }

    #[test]
    fn auto_follows_omarchy_when_present() {
        let dir = TempDir::new().unwrap();
        let current = omarchy_current_dir(dir.path());
        assert_eq!(load_auto(&current), None);

        fs::create_dir_all(current.join("theme")).unwrap();
        fs::write(current.join("theme/colors.toml"), ETHEREAL).unwrap();
        fs::write(current.join("theme.name"), "ethereal\n").unwrap();
        let theme = load_auto(&current).unwrap();
        assert_eq!(theme.name, "Omarchy: ethereal");

        fs::write(current.join("theme/colors.toml"), "= broken").unwrap();
        assert_eq!(load_auto(&current), None);
    }

    #[test]
    fn lists_and_resolves_user_themes() {
        let dir = TempDir::new().unwrap();
        let themes = dir.path().join("themes");
        fs::create_dir_all(&themes).unwrap();
        fs::write(themes.join("ethereal.toml"), ETHEREAL).unwrap();
        fs::write(themes.join("broken.toml"), "= x").unwrap();
        fs::write(themes.join("notes.txt"), "x").unwrap();
        assert_eq!(list_user_themes(&themes), ["broken", "ethereal"]);
        assert_eq!(
            list_user_themes(&dir.path().join("missing")),
            Vec::<String>::new()
        );

        let omarchy = dir.path().join("no-omarchy");
        assert_eq!(resolve("auto", &omarchy, &themes), Ok(None));
        assert_eq!(
            resolve("light", &omarchy, &themes).unwrap().unwrap().mode,
            Mode::Light
        );
        let user = resolve("ethereal", &omarchy, &themes).unwrap().unwrap();
        assert_eq!(user.colors["background"], "#060b1e");
        assert!(resolve("broken", &omarchy, &themes).is_err());
        assert!(resolve("missing", &omarchy, &themes).is_err());
    }

    #[test]
    fn theme_swaps_are_recognised() {
        let current = Path::new("/h/.local/state/omarchy/current");
        assert!(is_theme_swap(current, &current.join("theme")));
        assert!(is_theme_swap(current, &current.join("theme.name")));
        assert!(!is_theme_swap(current, &current.join("background")));
        assert!(!is_theme_swap(current, &current.join("next-theme")));
    }
}
