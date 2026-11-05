import {
  acceptCompletion,
  autocompletion,
  closeCompletion,
  moveCompletionSelection,
  type Completion,
  type CompletionContext,
  type CompletionResult,
} from "@codemirror/autocomplete";
import { Prec, type Extension } from "@codemirror/state";
import { keymap } from "@codemirror/view";
import { firstAssignments } from "./language";

/** Common units and words of the calculation language (PRD C3–C8). */
const WORDS: Completion[] = [
  ...["in", "to", "of", "as", "prev", "sum", "total", "avg", "mod"].map((label) => ({
    label,
    type: "keyword",
  })),
  ...["sqrt", "round", "floor", "ceil", "abs", "log", "ln", "sin", "cos", "tan"].map((label) => ({
    label,
    type: "function",
    apply: `${label}(`,
  })),
  ...[
    "km",
    "metres",
    "cm",
    "mm",
    "miles",
    "feet",
    "inches",
    "yards",
    "kg",
    "grams",
    "pounds",
    "ounces",
    "litres",
    "mL",
    "gallons",
    "seconds",
    "minutes",
    "hours",
    "days",
    "weeks",
    "months",
    "years",
    "bytes",
    "bits",
    "KB",
    "MB",
    "GB",
    "TB",
    "KiB",
    "MiB",
    "GiB",
    "TiB",
    "celsius",
    "fahrenheit",
    "kelvin",
    "mph",
    "hex",
    "binary",
    "octal",
  ].map((label) => ({ label, type: "unit" })),
];

/**
 * Completion source (PRD E5): variables assigned on earlier lines, common
 * units and keywords, and currency codes. Starts after two letters; never
 * inside a number or a comment.
 */
export function completeNote(currencies: () => readonly string[]) {
  return (context: CompletionContext): CompletionResult | null => {
    const currencyOptions: Completion[] = currencies().map((label) => ({ label, type: "currency" }));
    const word = context.matchBefore(/[A-Za-z_][A-Za-z_0-9]*/);
    if (!word || (word.text.length < 2 && !context.explicit)) return null;
    const line = context.state.doc.lineAt(context.pos);
    const before = line.text.slice(0, context.pos - line.from);
    if (/(^\s*#|\/\/|\s#)/.test(before)) return null;
    // A letter glued to a number is a unit (`5km`) or hex digits; still offer units only then.
    const afterDigit = /\d$/.test(before.slice(0, before.length - word.text.length));
    const variables: Completion[] = [];
    for (const [name, assignedOn] of firstAssignments(context.state.doc)) {
      if (assignedOn < line.number) variables.push({ label: name, type: "variable", boost: 10 });
    }
    const options = afterDigit
      ? WORDS.filter((w) => w.type === "unit").concat(currencyOptions)
      : [...variables, ...WORDS, ...currencyOptions];
    return { from: word.from, options, validFor: /^[A-Za-z_][A-Za-z_0-9]*$/ };
  };
}

/**
 * Autocomplete with Tab to accept: Enter always starts a new line in a
 * calculator note, so it never accepts a suggestion.
 */
export function noteCompletion(currencies: () => readonly string[]): Extension {
  return [
    autocompletion({ override: [completeNote(currencies)], defaultKeymap: false, icons: false }),
    Prec.high(
      keymap.of([
        { key: "Tab", run: acceptCompletion },
        { key: "ArrowDown", run: moveCompletionSelection(true) },
        { key: "ArrowUp", run: moveCompletionSelection(false) },
        { key: "Escape", run: closeCompletion },
      ]),
    ),
  ];
}
