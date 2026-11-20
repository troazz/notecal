// First-run onboarding (PRD A8): the pure parts, kept out of the component so
// they can be tested.
import type { CheatSection } from "../notes/cheatsearch";

export type StepId = "welcome" | "look" | "language" | "keys" | "data" | "ready";

export const STEPS: readonly StepId[] = ["welcome", "look", "language", "keys", "data", "ready"];

/** Mirrors `DesktopInfo` in src-tauri/src/pin.rs. */
export interface DesktopInfo {
  wayland: boolean;
  hyprland: "lua" | "classic" | null;
  pin: "native" | "hyprland" | "none";
}

/** Font size presets; the Settings screen still takes any size from 8 to 40. */
export const FONT_SIZES = [13, 15, 17, 20] as const;

/** Decimal place presets. */
export const PRECISIONS = [2, 4, 6, 10] as const;

/**
 * Lines for the user's compositor config: a key that runs `notecal --toggle`,
 * and on Hyprland the float rule that pinning needs. Null off Wayland, where
 * the app registers its own shortcut.
 */
export function bindLines(desktop: DesktopInfo): string | null {
  if (!desktop.wayland) return null;
  switch (desktop.hyprland) {
    case "lua":
      return [
        'hl.bind("ALT + SPACE", hl.dsp.exec_cmd("notecal --toggle"), { description = "notecal" })',
        'hl.window_rule({ match = { class = "^notecal$" }, float = true, center = true, size = { 720, 520 } })',
      ].join("\n");
    case "classic":
      return ["bind = ALT, SPACE, exec, notecal --toggle", "windowrule = float, size 720 520, center, class:^(notecal)$"].join(
        "\n",
      );
    default:
      return "notecal --toggle";
  }
}

/** The welcome note's name. */
export const WELCOME_NOTE = "Welcome";

/**
 * The welcome note: a short intro, then the first example of every
 * cheatsheet section under its title, with its explanation as a comment.
 * Headers and comments are plain note syntax, so every line still evaluates
 * (src-tauri checks the same layout in `welcome_layout_evaluates`).
 */
export function welcomeNote(
  sheet: readonly CheatSection[],
  text: { title: string; intro: string; section: (id: string) => string; explain: (id: string) => string },
): string {
  const lines = [`# ${text.title}`, `// ${text.intro}`];
  for (const section of sheet) {
    const example = section.examples[0];
    if (!example) continue;
    lines.push("", `# ${text.section(section.id)}`, `// ${text.explain(example.id)}`, example.text);
  }
  return `${lines.join("\n")}\n`;
}
