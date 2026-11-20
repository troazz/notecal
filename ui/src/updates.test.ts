import { describe, expect, it } from "vitest";
import { upgradeCommand } from "./updates";

describe("upgradeCommand", () => {
  it("gives the package manager's command", () => {
    expect(upgradeCommand("homebrew", "1.0.1")).toBe("brew upgrade --cask notecal");
    expect(upgradeCommand("pacman", "1.0.1")).toContain("makepkg -si");
    expect(upgradeCommand("deb", "1.0.1")).toBe("sudo apt install ./notecal_1.0.1_amd64.deb");
  });

  it("has none for a downloaded app", () => {
    expect(upgradeCommand("download", "1.0.1")).toBeNull();
    expect(upgradeCommand("updater", "1.0.1")).toBeNull();
  });
});
