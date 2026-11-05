import { describe, expect, it } from "vitest";
import { BUILTIN, cssVariables, resolvePalette, type Palette, type Theme } from "./theme";

/** WCAG 2.2 contrast ratio between two `#rrggbb` colours. */
function contrast(a: string, b: string): number {
  const luminance = (hex: string) => {
    const [r, g, bl] = [1, 3, 5].map((i) => {
      const v = parseInt(hex.slice(i, i + 2), 16) / 255;
      return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
    });
    return 0.2126 * r! + 0.7152 * g! + 0.0722 * bl!;
  };
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi! + 0.05) / (lo! + 0.05);
}

describe("resolvePalette", () => {
  it("uses the built-in palette for the system mode without a theme", () => {
    expect(resolvePalette(null, true)).toEqual({ mode: "dark", palette: BUILTIN.dark });
    expect(resolvePalette(null, false)).toEqual({ mode: "light", palette: BUILTIN.light });
  });

  it("a theme's mode wins over the system, and missing keys fall back", () => {
    const theme: Theme = { name: "t", mode: "light", colors: { background: "#ffffff", yellow: "#777700" } };
    const { mode, palette } = resolvePalette(theme, true);
    expect(mode).toBe("light");
    expect(palette.background).toBe("#ffffff");
    expect(palette.yellow).toBe("#777700");
    expect(palette.magenta).toBe(BUILTIN.light.magenta);
  });
});

describe("cssVariables", () => {
  it("maps syntax roles to palette colours", () => {
    const vars = cssVariables(BUILTIN.dark);
    expect(vars["--hl-number"]).toBe(BUILTIN.dark.yellow);
    expect(vars["--hl-unit"]).toBe(BUILTIN.dark.cyan);
    expect(vars["--hl-keyword"]).toBe(BUILTIN.dark.magenta);
    expect(vars["--error-color"]).toBe(BUILTIN.dark.red);
    expect(vars["--bg-color"]).toBe(BUILTIN.dark.background);
  });
});

describe("built-in palettes meet WCAG 2.2 AA (PRD E6)", () => {
  const textKeys: (keyof Palette)[] = [
    "foreground",
    "muted",
    "accent",
    "red",
    "yellow",
    "orange",
    "green",
    "cyan",
    "blue",
    "magenta",
  ];
  for (const mode of ["light", "dark"] as const) {
    const palette = BUILTIN[mode];
    for (const key of textKeys) {
      it(`${mode} ${key} on background ≥ 4.5:1`, () => {
        expect(contrast(palette[key], palette.background)).toBeGreaterThanOrEqual(4.5);
      });
      it(`${mode} ${key} on panel ≥ 4.5:1`, () => {
        expect(contrast(palette[key], palette.lighter_background)).toBeGreaterThanOrEqual(4.5);
      });
    }
    it(`${mode} foreground on selection ≥ 4.5:1`, () => {
      expect(contrast(palette.foreground, palette.selection)).toBeGreaterThanOrEqual(4.5);
    });
  }
});
