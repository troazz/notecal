import { describe, expect, it } from "vitest";
import { Scheduler } from "./scheduler";
import type { EvalRequest, EvalResponse } from "./types";

function setup() {
  const frames: (() => void)[] = [];
  const requests: EvalRequest[] = [];
  const resolvers: ((r: EvalResponse | null) => void)[] = [];
  const applied: EvalResponse[] = [];
  let text = "";
  const scheduler = new Scheduler({
    invoke: (request) => {
      requests.push(request);
      return new Promise((resolve) => resolvers.push(resolve));
    },
    apply: (response) => applied.push(response),
    getText: () => text,
    getLocale: () => "en",
    requestFrame: (cb) => frames.push(cb),
  });
  const flushFrame = () => frames.splice(0).forEach((cb) => cb());
  const respond = (i: number, version: number) =>
    resolvers[i]!({ version, lines: [{ display: String(version), copy: null, error: null }] });
  const tick = () => new Promise((r) => setTimeout(r, 0));
  const skip = (i: number) => resolvers[i]!(null);
  return {
    scheduler,
    requests,
    applied,
    flushFrame,
    respond,
    skip,
    tick,
    setText: (t: string) => (text = t),
  };
}

describe("Scheduler", () => {
  it("sends one request per frame with the latest text", () => {
    const s = setup();
    s.setText("1");
    s.scheduler.changed();
    s.setText("1+");
    s.scheduler.changed();
    s.setText("1+1");
    s.scheduler.changed();
    expect(s.requests).toHaveLength(0);
    s.flushFrame();
    expect(s.requests).toEqual([{ version: 3, text: "1+1", locale: "en" }]);
  });

  it("applies the response for the current version", async () => {
    const s = setup();
    s.scheduler.changed();
    s.flushFrame();
    s.respond(0, 1);
    await s.tick();
    expect(s.applied.map((r) => r.version)).toEqual([1]);
  });

  it("ignores a response older than the current version", async () => {
    const s = setup();
    s.scheduler.changed();
    s.flushFrame();
    s.scheduler.changed();
    s.flushFrame();
    s.respond(0, 1);
    s.respond(1, 2);
    await s.tick();
    expect(s.applied.map((r) => r.version)).toEqual([2]);
  });

  it("ignores a request the backend skipped", async () => {
    const s = setup();
    s.scheduler.changed();
    s.flushFrame();
    s.skip(0);
    await s.tick();
    expect(s.applied).toHaveLength(0);
  });
});
