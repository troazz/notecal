import { RangeSet, RangeSetBuilder, StateField, type Extension } from "@codemirror/state";
import { EditorView, GutterMarker, gutter } from "@codemirror/view";
import { setResults } from "./results";

/**
 * PRD errors rule: the result column never shows error text; a gutter
 * marker does, and hovering it shows the engine's reason.
 */
class ErrorMarker extends GutterMarker {
  constructor(readonly reason: string) {
    super();
  }

  override eq(other: GutterMarker): boolean {
    return other instanceof ErrorMarker && other.reason === this.reason;
  }

  override toDOM(): Node {
    const marker = document.createElement("span");
    marker.className = "cm-error-marker";
    marker.textContent = "●";
    marker.title = this.reason;
    marker.setAttribute("role", "img");
    marker.setAttribute("aria-label", `Error: ${this.reason}`);
    return marker;
  }
}

/** Error markers by line start, from the latest results. */
const errorsField = StateField.define<RangeSet<ErrorMarker>>({
  create: () => RangeSet.empty,
  update(markers, tr) {
    for (const effect of tr.effects) {
      if (!effect.is(setResults)) continue;
      const builder = new RangeSetBuilder<ErrorMarker>();
      const count = Math.min(tr.state.doc.lines, effect.value.length);
      for (let i = 0; i < count; i++) {
        const reason = effect.value[i]?.error;
        if (reason) {
          const from = tr.state.doc.line(i + 1).from;
          builder.add(from, from, new ErrorMarker(reason));
        }
      }
      return builder.finish();
    }
    return tr.docChanged ? markers.map(tr.changes) : markers;
  },
});

export const errorGutter: Extension = [
  errorsField,
  gutter({
    class: "cm-error-gutter",
    markers: (view) => {
      // The line being typed is usually incomplete; marking it would flicker.
      const cursorLine = view.state.doc.lineAt(view.state.selection.main.head).from;
      const all = view.state.field(errorsField);
      return all.update({ filter: (from) => from !== cursorLine });
    },
  }),
  EditorView.baseTheme({
    ".cm-error-gutter": { width: "1rem", textAlign: "center" },
    ".cm-error-marker": { color: "var(--error-color)", fontSize: "0.7em", cursor: "help" },
    ".cm-gutters": { background: "transparent", border: "none" },
  }),
];
