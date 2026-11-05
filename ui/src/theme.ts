import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/** Mirrors `Theme` in src-tauri/src/theme.rs. */
export interface Theme {
  name: string;
  mode: "light" | "dark";
  colors: Partial<Record<PaletteKey, string>>;
}

/** The palette keys of Omarchy's `colors.toml` that notecal uses. */
export type PaletteKey =
  | "background"
  | "lighter_background"
  | "foreground"
  | "accent"
  | "selection"
  | "muted"
  | "red"
  | "yellow"
  | "orange"
  | "green"
  | "cyan"
  | "blue"
  | "magenta";

export type Palette = Record<PaletteKey, string>;

/**
 * Built-in palettes: used without Omarchy, and for any key a theme leaves
 * out. Text colours meet WCAG 2.2 AA (4.5:1) on `background` (PRD E6).
 */
export const BUILTIN: Record<Theme["mode"], Palette> = {
  light: {
    background: "#fbfbfa",
    lighter_background: "#ecedef",
    foreground: "#1d1f23",
    accent: "#2457c5",
    selection: "#d7e3fb",
    muted: "#5b6270",
    red: "#b42318",
    yellow: "#8a4b00",
    orange: "#a3460a",
    green: "#2f6b1f",
    cyan: "#0b6b6b",
    blue: "#2457c5",
    magenta: "#8a2f8f",
  },
  dark: {
    background: "#1b1d22",
    lighter_background: "#2a2d34",
    foreground: "#e6e7ea",
    accent: "#8fb3ff",
    selection: "#2f3b55",
    muted: "#a0a7b4",
    red: "#ff8a80",
    yellow: "#f0c674",
    orange: "#f5a97f",
    green: "#a6d189",
    cyan: "#7fd4d4",
    blue: "#8fb3ff",
    magenta: "#d7a4e8",
  },
};

/** CSS variable → palette key. The editor and its parts read only these variables. */
const VARIABLES: Record<string, PaletteKey> = {
  "--bg-color": "background",
  "--panel-color": "lighter_background",
  "--text-color": "foreground",
  "--result-color": "muted",
  "--muted-color": "muted",
  "--accent-color": "accent",
  "--selection-color": "selection",
  "--error-color": "red",
  "--hl-heading": "foreground",
  "--hl-comment": "muted",
  "--hl-number": "yellow",
  "--hl-unit": "cyan",
  "--hl-variable": "foreground",
  "--hl-keyword": "magenta",
  "--hl-function": "blue",
  "--hl-operator": "blue",
};

/** The full palette for `theme`, or the built-in one for the system mode. */
export function resolvePalette(theme: Theme | null, systemDark: boolean): { mode: Theme["mode"]; palette: Palette } {
  const mode = theme?.mode ?? (systemDark ? "dark" : "light");
  return { mode, palette: { ...BUILTIN[mode], ...theme?.colors } };
}

/** CSS custom properties for a palette. */
export function cssVariables(palette: Palette): Record<string, string> {
  const vars: Record<string, string> = {};
  for (const [name, key] of Object.entries(VARIABLES)) vars[name] = palette[key];
  // Borders sit between background and text.
  vars["--border-color"] = `color-mix(in srgb, ${palette.foreground} 18%, ${palette.background})`;
  return vars;
}

function apply(theme: Theme | null, systemDark: boolean): void {
  const { mode, palette } = resolvePalette(theme, systemDark);
  const root = document.documentElement;
  for (const [name, value] of Object.entries(cssVariables(palette))) root.style.setProperty(name, value);
  root.style.colorScheme = mode;
}

/**
 * Applies the built-in palette at once (no flash), then the followed theme
 * (Omarchy) when the backend reports one, and keeps both up to date.
 */
export function startTheming(): void {
  const media = window.matchMedia("(prefers-color-scheme: dark)");
  let followed: Theme | null = null;
  const refresh = () => apply(followed, media.matches);
  refresh();
  media.addEventListener("change", refresh);
  const set = (theme: Theme | null) => {
    followed = theme;
    refresh();
  };
  invoke<Theme | null>("get_theme")
    .then(set)
    .catch((error: unknown) => console.error("get_theme failed", error));
  void listen<Theme | null>("theme-changed", (event) => set(event.payload));
}
