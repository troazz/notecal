import { describe, expect, it } from "vitest";
import { conflictStamp, NoteSession, SAVE_DELAY_MS, type SaveOutcome } from "./session";

function setup(files: Record<string, string> = { A: "a", B: "b" }) {
  const disk = { ...files };
  const saves: { name: string; text: string; stamp: string }[] = [];
  const messages: string[] = [];
  const timers = new Map<number, { cb: () => void; ms: number }>();
  let nextTimer = 1;
  let text = "";
  let conflictNext: string | null = null;
  const session = new NoteSession({
    open: async (name) => {
      const content = disk[name];
      if (content === undefined) throw new Error(`no ${name}`);
      return content;
    },
    save: async (name, saved, stamp): Promise<SaveOutcome> => {
      saves.push({ name, text: saved, stamp });
      if (conflictNext !== null) {
        const diskText = conflictNext;
        conflictNext = null;
        return { kind: "conflict", copyName: `${name} (conflict ${stamp})`, diskText };
      }
      disk[name] = saved;
      return { kind: "saved" };
    },
    getText: () => text,
    setText: (t) => {
      text = t;
      // The editor reports its own change, as CodeMirror's listener does.
      session.edited();
    },
    notify: (m) => messages.push(m),
    setTimer: (cb, ms) => {
      const id = nextTimer++;
      timers.set(id, { cb, ms });
      return id;
    },
    clearTimer: (id) => timers.delete(id as number),
    now: () => new Date(2026, 10, 5, 18, 7),
  });
  const type = (t: string) => {
    text = t;
    session.edited();
  };
  const fireTimers = async () => {
    const pending = [...timers.values()];
    timers.clear();
    pending.forEach((t) => t.cb());
    await session.flush();
  };
  return {
    session,
    disk,
    saves,
    messages,
    timers,
    type,
    fireTimers,
    text: () => text,
    conflictOnNextSave: (diskText: string) => (conflictNext = diskText),
  };
}

describe("NoteSession", () => {
  it("opening a note shows it without counting as an edit", async () => {
    const s = setup();
    await s.session.open("A");
    expect(s.text()).toBe("a");
    expect(s.session.hasUnsavedEdits).toBe(false);
    expect(s.timers.size).toBe(0);
  });

  it("saves once, 500 ms after a burst of edits", async () => {
    const s = setup();
    await s.session.open("A");
    s.type("a1");
    s.type("a12");
    s.type("a123");
    expect(s.timers.size).toBe(1);
    expect([...s.timers.values()][0]!.ms).toBe(SAVE_DELAY_MS);
    await s.fireTimers();
    expect(s.saves.map((x) => x.text)).toEqual(["a123"]);
    expect(s.session.hasUnsavedEdits).toBe(false);
  });

  it("switching notes saves the current one first", async () => {
    const s = setup();
    await s.session.open("A");
    s.type("edited");
    await s.session.open("B");
    expect(s.disk["A"]).toBe("edited");
    expect(s.text()).toBe("b");
    expect(s.timers.size).toBe(0);
  });

  it("flush without edits does not save", async () => {
    const s = setup();
    await s.session.open("A");
    await s.session.flush();
    expect(s.saves).toEqual([]);
  });

  it("reloads an external change when there are no unsaved edits", async () => {
    const s = setup();
    await s.session.open("A");
    s.disk["A"] = "from sync";
    await s.session.diskChanged({ name: "A", kind: "changed" });
    expect(s.text()).toBe("from sync");
    expect(s.saves).toEqual([]);
  });

  it("with unsaved edits, saves (Rust makes the conflict copy) and shows the disk text", async () => {
    const s = setup();
    await s.session.open("A");
    s.type("my edit");
    s.conflictOnNextSave("from sync");
    await s.session.diskChanged({ name: "A", kind: "changed" });
    expect(s.saves).toEqual([{ name: "A", text: "my edit", stamp: "2026-11-05 1807" }]);
    expect(s.text()).toBe("from sync");
    expect(s.messages[0]).toContain("A (conflict 2026-11-05 1807)");
    expect(s.session.hasUnsavedEdits).toBe(false);
  });

  it("keeps the buffer when the note is deleted on disk", async () => {
    const s = setup();
    await s.session.open("A");
    s.type("keep me");
    await s.session.diskChanged({ name: "A", kind: "deleted" });
    expect(s.text()).toBe("keep me");
    await s.fireTimers();
    expect(s.disk["A"]).toBe("keep me");
  });

  it("ignores changes to other notes", async () => {
    const s = setup();
    await s.session.open("A");
    s.disk["B"] = "changed";
    await s.session.diskChanged({ name: "B", kind: "changed" });
    expect(s.text()).toBe("a");
  });

  it("a failed save keeps the edits unsaved and reports it", async () => {
    const s = setup();
    await s.session.open("A");
    s.type("x");
    s.disk["A"] = "a";
    const failing = new NoteSession({
      open: async () => "a",
      save: async () => {
        throw "disk full";
      },
      getText: () => "x",
      setText: () => {},
      notify: (m) => s.messages.push(m),
      setTimer: () => 0,
      clearTimer: () => {},
      now: () => new Date(),
    });
    await failing.open("A");
    failing.edited();
    await failing.flush();
    expect(failing.hasUnsavedEdits).toBe(true);
    expect(s.messages.at(-1)).toContain("disk full");
  });

  it("follows a rename of the open note", async () => {
    const s = setup();
    await s.session.open("A");
    s.session.renamed("A", "C");
    s.type("x");
    await s.fireTimers();
    expect(s.saves.at(-1)?.name).toBe("C");
  });
});

describe("NoteSession under races and failures", () => {
  /** A session whose saves wait until released, to model a save in flight. */
  function slowSetup() {
    const disk: Record<string, string> = { A: "a" };
    const order: string[] = [];
    const messages: string[] = [];
    let release: (() => void) | null = null;
    let conflict: string | null = null;
    let text = "";
    const session = new NoteSession({
      open: async (name) => {
        order.push(`open:${disk[name]}`);
        return disk[name]!;
      },
      save: async (name, saved) => {
        order.push(`save:${name}=${saved}`);
        if (name.includes("conflict")) {
          disk[name] = saved;
          return { kind: "saved" };
        }
        await new Promise<void>((resolve) => (release = resolve));
        if (conflict !== null) {
          const diskText = conflict;
          conflict = null;
          disk[`${name} (conflict x)`] = saved;
          return { kind: "conflict", copyName: `${name} (conflict x)`, diskText };
        }
        disk[name] = saved;
        return { kind: "saved" };
      },
      getText: () => text,
      setText: (t) => {
        text = t;
        session.edited();
      },
      notify: (m) => messages.push(m),
      setTimer: () => 0,
      clearTimer: () => {},
      now: () => new Date(2026, 10, 5, 18, 7),
    });
    const type = (t: string) => {
      text = t;
      session.edited();
    };
    const tick = () => new Promise((r) => setTimeout(r, 0));
    return {
      session,
      disk,
      order,
      messages,
      type,
      tick,
      text: () => text,
      release: () => release?.(),
      conflictOnNextSave: (d: string) => (conflict = d),
    };
  }

  it("reloads after a save in flight lands, not before", async () => {
    const s = slowSetup();
    await s.session.open("A");
    s.type("mine");
    const saving = s.session.flush();
    await s.tick();
    const reload = s.session.diskChanged({ name: "A", kind: "changed" });
    await s.tick();
    expect(s.order).toEqual(["open:a", "save:A=mine"]);
    s.release();
    await saving;
    await reload;
    // The reload read the file after the save, so it sees "mine".
    expect(s.order.at(-1)).toBe("open:mine");
    expect(s.text()).toBe("mine");
  });

  it("keystrokes typed during a conflicting save end up in the conflict copy", async () => {
    const s = slowSetup();
    await s.session.open("A");
    s.type("mine");
    s.conflictOnNextSave("from sync");
    const saving = s.session.flush();
    await s.tick();
    s.type("mine plus more");
    s.release();
    await saving;
    expect(s.text()).toBe("from sync");
    expect(s.disk["A (conflict x)"]).toBe("mine plus more");
  });

  it("an error inside a save does not stop later saves", async () => {
    let calls = 0;
    let text = "x";
    const messages: string[] = [];
    const session = new NoteSession({
      open: async () => "a",
      save: async () => {
        calls += 1;
        return { kind: "saved" };
      },
      getText: () => {
        if (text === "boom") throw new Error("editor gone");
        return text;
      },
      setText: () => {},
      notify: (m) => messages.push(m),
      setTimer: () => 0,
      clearTimer: () => {},
      now: () => new Date(),
    });
    await session.open("A");
    text = "boom";
    session.edited();
    await session.flush();
    expect(messages.at(-1)).toContain("editor gone");
    text = "fine";
    session.edited();
    await session.flush();
    expect(calls).toBe(1);
  });

  it("does not switch notes when the current edits cannot be saved", async () => {
    const session = new NoteSession({
      open: async (name) => name.toLowerCase(),
      save: async () => {
        throw "disk full";
      },
      getText: () => "edited",
      setText: () => {},
      notify: () => {},
      setTimer: () => 0,
      clearTimer: () => {},
      now: () => new Date(),
    });
    await session.open("A");
    session.edited();
    await expect(session.open("B")).rejects.toThrow("unsaved edits");
    expect(session.name).toBe("A");
    expect(session.hasUnsavedEdits).toBe(true);
  });
});

describe("conflictStamp", () => {
  it("uses local time as YYYY-MM-DD HHMM", () => {
    expect(conflictStamp(new Date(2026, 0, 2, 3, 4))).toBe("2026-01-02 0304");
  });
});
