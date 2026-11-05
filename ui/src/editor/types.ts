/** Mirrors `EvalRequest`/`EvalResponse` in src-tauri/src/evaluator.rs. */
export interface EvalRequest {
  version: number;
  text: string;
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
