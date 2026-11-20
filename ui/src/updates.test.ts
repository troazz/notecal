import { describe, expect, it } from "vitest";
import { upgradeCommand } from "./updates";

describe("upgradeCommand", () => {
  it("gives the package manager's command", () => {
    expect(upgradeCommand("homebrew")).toBe("brew upgrade --cask notecal");
    expect(upgradeCommand("pacman")).toContain("makepkg -si");
  });

  it("has none for a downloaded app", () => {
    expect(upgradeCommand("download")).toBeNull();
  });
});
