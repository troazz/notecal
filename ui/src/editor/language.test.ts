import { ensureSyntaxTree } from "@codemirror/language";
import { EditorState } from "@codemirror/state";
import { highlightTree, tagHighlighter, tags } from "@lezer/highlight";
import { describe, expect, it } from "vitest";
import { notecalLanguage } from "./language";

/** One class per tag the highlight style colours, most specific first. */
const highlighter = tagHighlighter([
  { tag: tags.heading, class: "heading" },
  { tag: tags.comment, class: "comment" },
  { tag: tags.number, class: "number" },
  { tag: tags.keyword, class: "keyword" },
  { tag: tags.definition(tags.variableName), class: "definition" },
  { tag: tags.function(tags.variableName), class: "function" },
  { tag: tags.variableName, class: "variable" },
  { tag: tags.unit, class: "unit" },
  { tag: tags.operator, class: "operator" },
  { tag: tags.paren, class: "paren" },
]);

/** `[text, class]` pairs for every highlighted span of `doc`. */
function highlight(doc: string): [string, string][] {
  const state = EditorState.create({ doc, extensions: [notecalLanguage] });
  const tree = ensureSyntaxTree(state, doc.length, 5000);
  if (!tree) throw new Error("no syntax tree");
  const spans: [string, string][] = [];
  highlightTree(tree, highlighter, (from, to, cls) => spans.push([doc.slice(from, to), cls]));
  return spans;
}

describe("notecal highlighting", () => {
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

  it("marks assignments and later uses as variables", () => {
    const spans = highlight("rent = 4,500,000\nrent * 12");
    expect(spans[0]).toEqual(["rent", "definition"]);
    expect(spans).toContainEqual(["rent", "variable"]);
  });

  it("a name before its assignment is not yet a variable", () => {
    expect(highlight("rate * 2\nrate = 3")[0]).toEqual(["rate", "unit"]);
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
