// Update prompt (PRD A9): what the user runs to upgrade, by install method.

/** Mirrors `UpdateInfo` in src-tauri/src/updates.rs. */
export interface UpdateInfo {
  version: string;
  install: "homebrew" | "pacman" | "deb" | "updater" | "download";
  /** `updater` only: downloaded and verified, ready to install. */
  ready: boolean;
}

/** The command to copy, or null when there is none (the updater, or a plain download). */
export function upgradeCommand(install: UpdateInfo["install"], version: string): string | null {
  switch (install) {
    case "homebrew":
      return "brew upgrade --cask notecal";
    case "pacman":
      // Built from the repository (README › Arch Linux); run in the clone.
      return "git pull && cd packaging/aur && makepkg -si";
    case "deb":
      // After downloading it from the release page (the Download button).
      return `sudo apt install ./notecal_${version}_amd64.deb`;
    default:
      return null;
  }
}
