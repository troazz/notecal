import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** Mirrors `AppSettings` in src-tauri/src/settings.rs (TOML keys, snake_case). */
export interface AppSettings {
  /** Interface language; `auto` follows the system. */
  language: "auto" | "en" | "id";
  shortcut: string;
  theme: string;
  font_family: string;
  font_size: number;
  precision: number;
  locale: "en" | "id";
  notes_dir: string;
  currency_refresh: boolean;
  always_on_top: boolean;
  background: boolean;
  /** Pixels per inch for `px`. */
  css_ppi: number;
  /** Pixels per `em` and `rem`. */
  css_em_px: number;
  /** Vim keys in the editor. */
  vim_mode: boolean;
  /** Look for a newer release once a day. */
  check_updates: boolean;
}

/** Mirrors `SettingsView`: settings plus problems found reading the file. */
export interface SettingsView {
  settings: AppSettings;
  problems: string[];
}

export const settingsApi = {
  get: () => invoke<SettingsView>("get_settings"),
  save: (settings: AppSettings) => invoke<void>("save_settings", { settings }),
  onChanged: (handler: (view: SettingsView) => void): Promise<UnlistenFn> =>
    listen<SettingsView>("settings-changed", (e) => handler(e.payload)),
};
