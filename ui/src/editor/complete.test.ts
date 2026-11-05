import { CompletionContext } from "@codemirror/autocomplete";
import { EditorState } from "@codemirror/state";
import { describe, expect, it } from "vitest";
import { completeNote } from "./complete";

/** Labels offered at the end of `doc` (or at `|`). */
function labels(doc: string, explicit = false): string[] | null {
  const pos = doc.includes("|") ? doc.indexOf("|") : doc.length;
  const text = doc.replace("|", "");
  const state = EditorState.create({ doc: text });
  const result = completeNote(() => ["IDR", "USD"])(new CompletionContext(state, pos, explicit));
  return result ? result.options.map((o) => o.label) : null;
}

describe("completeNote", () => {
  it("offers variables assigned on earlier lines", () => {
    const offered = labels("electricity = 350,000\nel");
    expect(offered).toContain("electricity");
  });

  it("does not offer a variable before its assignment line", () => {
    const offered = labels("el|\nelectricity = 1");
    expect(offered).not.toContain("electricity");
  });

  it("waits for two letters unless asked", () => {
    expect(labels("e")).toBeNull();
    expect(labels("e", true)).not.toBeNull();
  });

  it("offers units and currencies after a number", () => {
    const offered = labels("5 ki")!;
    expect(offered).toContain("km");
    expect(offered).toContain("IDR");
    const glued = labels("5km")!;
    expect(glued).not.toContain("sqrt");
  });

  it("stays quiet in numbers and comments", () => {
    expect(labels("1234")).toBeNull();
    expect(labels("# Trip pl")).toBeNull();
    expect(labels("5 km // wal")).toBeNull();
  });
});
