import { afterEach, describe, expect, it } from "vitest";
import { setLanguage } from "../i18n.svelte";
import { cheatRows, fold, queryWords, type CheatSection } from "./cheatsearch";

afterEach(() => setLanguage("en"));

const SHEET: CheatSection[] = [
  {
    id: "percentages",
    examples: [
      { id: "percent-of", text: "20% of 150", result: "30" },
      { id: "percent-subtract", text: "150 - 20%", result: "120" },
    ],
  },
  {
    id: "dates",
    examples: [
      { id: "date-add", text: "today + 2 weeks", result: "Wed, 2 Dec 2026" },
      { id: "time-zone", text: "now in Tokyo", result: "12:00" },
    ],
  },
];

const texts = (query: string, vim = false) =>
  cheatRows(SHEET, query, vim).map((row) => (row.kind === "example" ? row.text : row.keys));

describe("cheatRows", () => {
  it("shows everything for an empty query, shortcuts last", () => {
    const rows = cheatRows(SHEET, "", false);
    expect(rows.slice(0, 4).every((r) => r.kind === "example")).toBe(true);
    expect(rows.at(-1)?.kind).toBe("shortcut");
    expect(rows.some((r) => r.kind === "shortcut" && r.keys === ":w")).toBe(false);
  });

  it("finds examples through their explanation, in either language", () => {
    expect(texts("subtract")).toEqual(["150 - 20%"]);
    expect(texts("subtr")).toEqual(["150 - 20%"]);
    setLanguage("id");
    expect(texts("subtract")).toEqual(["150 - 20%"]);
    expect(texts("kurangi")).toEqual(["150 - 20%"]);
  });

  it("matches the example itself by any part", () => {
    expect(texts("% of")).toEqual(["20% of 150"]);
    expect(texts("okyo")).toEqual(["now in Tokyo"]);
  });

  it("matches a section by keyword, title or id as a whole word", () => {
    expect(texts("discount")).toEqual(["20% of 150", "150 - 20%"]);
    expect(texts("diskon")).toEqual(["20% of 150", "150 - 20%"]);
    expect(texts("date")).toEqual(["today + 2 weeks", "now in Tokyo"]);
    expect(texts("tanggal")).toEqual(["today + 2 weeks", "now in Tokyo"]);
  });

  it("needs every word to match", () => {
    expect(texts("tz tokyo")).toEqual(["now in Tokyo"]);
    expect(texts("tz london")).toEqual([]);
  });

  it("ignores case and accents", () => {
    expect(texts("TÓKYO")).toEqual(["now in Tokyo"]);
    expect(fold("Été")).toBe("ete");
    expect(queryWords("  a   B ")).toEqual(["a", "b"]);
  });

  it("finds a shortcut by its own words, not through the section title", () => {
    expect(texts("pin")).toEqual(["Ctrl+Shift+T"]);
    setLanguage("id");
    expect(texts("pin")).toEqual(["Ctrl+Shift+T"]);
    expect(texts("keys").length).toBeGreaterThan(5);
  });

  it("lists vim keys only while vim mode is on", () => {
    expect(texts("vim")).toEqual([]);
    expect(texts("vim", true)).toEqual(["Esc", ":w", ":q  :x  :wq", '"+y  "+p']);
  });
});
