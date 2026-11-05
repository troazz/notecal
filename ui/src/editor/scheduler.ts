import type { EvalRequest, EvalResponse, Locale } from "./types";

export interface SchedulerDeps {
  /** Sends a request; resolves to `null` when the backend skipped it. */
  invoke: (request: EvalRequest) => Promise<EvalResponse | null>;
  /** Applies results for the current document. */
  apply: (response: EvalResponse) => void;
  getText: () => string;
  getLocale: () => Locale;
  /** `requestAnimationFrame` in the app; injectable for tests. */
  requestFrame: (callback: () => void) => void;
  onError?: (error: unknown) => void;
}

/**
 * Coalesces edits into at most one evaluation request per animation frame
 * and applies only the response for the latest document version
 * (PRD "Evaluation flow").
 */
export class Scheduler {
  private version = 0;
  private frameQueued = false;

  constructor(private readonly deps: SchedulerDeps) {}

  /** The document version the next applied response must match. */
  get currentVersion(): number {
    return this.version;
  }

  /** Call after every document change. */
  changed(): void {
    this.version += 1;
    if (this.frameQueued) return;
    this.frameQueued = true;
    this.deps.requestFrame(() => {
      this.frameQueued = false;
      void this.send();
    });
  }

  private async send(): Promise<void> {
    const request: EvalRequest = {
      version: this.version,
      text: this.deps.getText(),
      locale: this.deps.getLocale(),
    };
    try {
      const response = await this.deps.invoke(request);
      if (response !== null && response.version === this.version) {
        this.deps.apply(response);
      }
    } catch (error) {
      this.deps.onError?.(error);
    }
  }
}
