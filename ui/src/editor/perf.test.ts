import { describe, expect, it } from "vitest";
import { KeystrokeTimer, percentile } from "./perf";

describe("percentile", () => {
  it("uses nearest rank", () => {
    const values = Array.from({ length: 100 }, (_, i) => i + 1);
    expect(percentile(values, 50)).toBe(50);
    expect(percentile(values, 95)).toBe(95);
    expect(percentile([], 95)).toBe(0);
  });
});

describe("KeystrokeTimer", () => {
  it("closes every keystroke up to the painted version", () => {
    const timer = new KeystrokeTimer();
    timer.keystroke(1, 0);
    timer.keystroke(2, 5);
    timer.keystroke(3, 8);
    timer.painted(2, 20);
    expect(timer.stats()).toMatchObject({ count: 2, max: 20 });
    timer.painted(3, 30);
    expect(timer.stats().count).toBe(3);
  });

  it("keeps only the most recent samples", () => {
    const timer = new KeystrokeTimer(3);
    for (let v = 1; v <= 5; v++) {
      timer.keystroke(v, 0);
      timer.painted(v, v);
    }
    expect(timer.stats()).toMatchObject({ count: 3, p50: 4, max: 5 });
  });
});
