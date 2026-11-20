// Update prompt (PRD A9): what the user runs to upgrade, by install method.

/** Mirrors `UpdateInfo` in src-tauri/src/updates.rs. */
export interface UpdateInfo {
  version: string;
  install: "homebrew" | "pacman" | "download";
}

/** The command to copy, or null when the new version is downloaded instead. */
export function upgradeCommand(install: UpdateInfo["install"]): string | null {
  switch (install) {
    case "homebrew":
      return "brew upgrade --cask notecal";
    case "pacman":
      // Built from the repository (README › Arch Linux); run in the clone.
      return "git pull && cd packaging/aur && makepkg -si";
    default:
      return null;
  }
}
