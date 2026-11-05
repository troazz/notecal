/** Mirrors `EvalRequest`/`EvalResponse` in src-tauri/src/evaluator.rs. */
export type Locale = "en" | "id";

export interface EvalRequest {
  version: number;
  text: string;
  locale: Locale;
}

export interface LineResult {
  display: string | null;
  copy: string | null;
  error: string | null;
}

export interface EvalResponse {
  version: number;
  lines: LineResult[];
}
