/**
 * Keystroke-to-results latency for Gate 1: time from a keystroke to the
 * frame that shows results for that keystroke's document version.
 */
export class KeystrokeTimer {
  private readonly pending = new Map<number, number>();
  private readonly samples: number[] = [];

  constructor(private readonly capacity = 200) {}

  /** A keystroke produced document `version` at time `now` (ms). */
  keystroke(version: number, now: number): void {
    this.pending.set(version, now);
  }

  /** Results for `version` were painted at `now`; earlier keystrokes are done too. */
  painted(version: number, now: number): void {
    for (const [pendingVersion, start] of this.pending) {
      if (pendingVersion > version) continue;
      this.samples.push(now - start);
      this.pending.delete(pendingVersion);
    }
    if (this.samples.length > this.capacity) {
      this.samples.splice(0, this.samples.length - this.capacity);
    }
  }

  stats(): { count: number; p50: number; p95: number; max: number } {
    const sorted = [...this.samples].sort((a, b) => a - b);
    return {
      count: sorted.length,
      p50: percentile(sorted, 50),
      p95: percentile(sorted, 95),
      max: sorted.at(-1) ?? 0,
    };
  }
}

/** Nearest-rank percentile of an ascending array; 0 when empty. */
export function percentile(sorted: readonly number[], p: number): number {
  if (sorted.length === 0) return 0;
  const rank = Math.ceil((p / 100) * sorted.length);
  return sorted[Math.min(sorted.length, Math.max(1, rank)) - 1] ?? 0;
}
