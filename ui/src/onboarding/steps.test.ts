import { describe, expect, it } from "vitest";
import { bindLines, welcomeNote } from "./steps";

describe("bindLines", () => {
  it("offers nothing off Wayland, where the shortcut setting works", () => {
    expect(bindLines({ wayland: false, hyprland: null, pin: "native" })).toBeNull();
  });

  it("gives Hyprland's Lua and classic lines, with the float rule pinning needs", () => {
    const lua = bindLines({ wayland: true, hyprland: "lua", pin: "hyprland" });
    expect(lua).toContain('hl.dsp.exec_cmd("notecal --toggle")');
    expect(lua).toContain("float = true");
    const classic = bindLines({ wayland: true, hyprland: "classic", pin: "hyprland" });
    expect(classic).toContain("bind = ALT, SPACE, exec, notecal --toggle");
    expect(classic).toContain("windowrule = float");
  });

  it("gives the command for other Wayland compositors", () => {
    expect(bindLines({ wayland: true, hyprland: null, pin: "none" })).toBe("notecal --toggle");
  });
});

describe("welcomeNote", () => {
  const sheet = [
    {
      id: "percentages",
      examples: [
        { id: "percent-of", text: "20% of 150", result: "30" },
        { id: "percent-add", text: "150 + 20%", result: "180" },
      ],
    },
    { id: "variables", examples: [{ id: "assign", text: "price = 25\nprice * 4", result: "100" }] },
    { id: "empty", examples: [] },
  ];
  const note = welcomeNote(sheet, {
    title: "Welcome to notecal",
    intro: "Results appear on the right.",
    section: (id) => `Section ${id}`,
    explain: (id) => `About ${id}`,
  });

  it("puts the first example of each section under its title and explanation", () => {
    expect(note).toBe(
      [
        "# Welcome to notecal",
        "// Results appear on the right.",
        "",
        "# Section percentages",
        "// About percent-of",
        "20% of 150",
        "",
        "# Section variables",
        "// About assign",
        "price = 25",
        "price * 4",
        "",
      ].join("\n"),
    );
  });
});
