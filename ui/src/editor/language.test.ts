import { Text } from "@codemirror/state";
import { describe, expect, it } from "vitest";
import { firstAssignments, tokenizeLine } from "./language";

/** `[text, class]` pairs for one line. */
function highlight(line: string, vars: string[] = []): [string, string][] {
  return tokenizeLine(line, (name) => vars.includes(name)).map((t) => [line.slice(t.from, t.to), t.cls]);
}

describe("tokenizeLine", () => {
  it("colours numbers, units, operators and keywords", () => {
    expect(highlight("1,500.5 km in miles")).toEqual([
      ["1,500.5", "number"],
      ["km", "unit"],
      ["in", "keyword"],
      ["miles", "unit"],
    ]);
    expect(highlight("2 * (3 + 0xff)")).toEqual([
      ["2", "number"],
      ["*", "operator"],
      ["(", "paren"],
      ["3", "number"],
      ["+", "operator"],
      ["0xff", "number"],
      [")", "paren"],
    ]);
  });

  it("marks an assignment target and known variables", () => {
    expect(highlight("rent = 4,500,000")[0]).toEqual(["rent", "definition"]);
    expect(highlight("rent * 12", ["rent"])[0]).toEqual(["rent", "variable"]);
    expect(highlight("rent * 12")[0]).toEqual(["rent", "unit"]);
    // `==` is not an assignment.
    expect(highlight("a == b")[0]).toEqual(["a", "unit"]);
  });

  it("headers and comments run to the end of the line", () => {
    expect(highlight("# Trip plan")).toEqual([["# Trip plan", "heading"]]);
    expect(highlight("5 km // walk")).toContainEqual(["// walk", "comment"]);
    expect(highlight("5 km # walk")).toContainEqual(["# walk", "comment"]);
  });

  it("references and functions", () => {
    expect(highlight("sqrt(2) + prev")).toEqual([
      ["sqrt", "function"],
      ["(", "paren"],
      ["2", "number"],
      [")", "paren"],
      ["+", "operator"],
      ["prev", "keyword"],
    ]);
  });
});

describe("firstAssignments", () => {
  it("records the first line each name is assigned on", () => {
    const doc = Text.of(["rate * 2", "rate = 3", "x = rate", "rate = 4"]);
    expect(firstAssignments(doc)).toEqual(
      new Map([
        ["rate", 2],
        ["x", 3],
      ]),
    );
  });
});
