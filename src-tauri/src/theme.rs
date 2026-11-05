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
    fn theme_swaps_are_recognised() {
        let current = Path::new("/h/.local/state/omarchy/current");
        assert!(is_theme_swap(current, &current.join("theme")));
        assert!(is_theme_swap(current, &current.join("theme.name")));
        assert!(!is_theme_swap(current, &current.join("background")));
        assert!(!is_theme_swap(current, &current.join("next-theme")));
    }
}
