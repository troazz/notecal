import { describe, expect, it } from "vitest";
import { describeVim, isVimBusy, vimBusyIn, type VimInputState } from "./vimstate";

const idle: VimInputState = {
  insertMode: false,
  visualMode: false,
  inputState: { keyBuffer: [], operator: null, prefixRepeat: [], motionRepeat: [] },
};

describe("describeVim", () => {
  it("labels every mode the library reports", () => {
    expect(describeVim("normal", idle).mode).toBe("NORMAL");
    expect(describeVim("insert", { ...idle, insertMode: true }).mode).toBe("INSERT");
    expect(describeVim("replace", idle).mode).toBe("REPLACE");
    expect(describeVim("visual", idle).mode).toBe("VISUAL");
    expect(describeVim("visual linewise", idle).mode).toBe("V-LINE");
    expect(describeVim("visual blockwise", idle).mode).toBe("V-BLOCK");
  });

  it("shows keys typed towards a command", () => {
    const pending = {
      ...idle,
      inputState: { ...idle.inputState, prefixRepeat: ["2"], operator: "delete" },
    };
    expect(describeVim("normal", pending)).toEqual({ mode: "NORMAL", pending: "2d" });
    const indent = { ...idle, inputState: { operator: "indent", operatorArgs: { indentRight: true } } };
    expect(describeVim("normal", indent).pending).toBe(">");
    const upper = { ...idle, inputState: { operator: "changeCase", operatorShortcut: "gU", motionRepeat: ["3"] } };
    expect(describeVim("normal", upper).pending).toBe("gU3");
    const register = { ...idle, inputState: { registerName: "+", keyBuffer: ["g"] } };
    expect(describeVim("normal", register).pending).toBe('"+g');
  });

  it("shows no pending keys once a command finished or while typing text", () => {
    expect(describeVim("normal", idle).pending).toBe("");
    const typing = { ...idle, insertMode: true, inputState: { keyBuffer: ["a"] } };
    expect(describeVim("insert", typing).pending).toBe("");
  });
});

describe("isVimBusy (Esc goes to vim, not to hiding the window)", () => {
  it("is busy in insert and visual mode", () => {
    expect(isVimBusy({ ...idle, insertMode: true }, false)).toBe(true);
    expect(isVimBusy({ ...idle, visualMode: true }, false)).toBe(true);
  });

  it("is busy with an operator, a count or a register pending", () => {
    expect(isVimBusy({ ...idle, inputState: { operator: "delete" } }, false)).toBe(true);
    expect(isVimBusy({ ...idle, inputState: { prefixRepeat: ["2"] } }, false)).toBe(true);
    expect(isVimBusy({ ...idle, inputState: { registerName: "a" } }, false)).toBe(true);
  });

  it("is busy while the : prompt is open", () => {
    expect(isVimBusy(idle, true)).toBe(true);
  });

  it("is idle in normal mode with nothing pending, and without vim", () => {
    expect(isVimBusy(idle, false)).toBe(false);
    expect(isVimBusy(null, false)).toBe(false);
    expect(vimBusyIn({})).toBe(false);
    expect(vimBusyIn(null)).toBe(false);
  });

  it("reads the library's state from the view", () => {
    expect(vimBusyIn({ cm: { state: { vim: { ...idle, insertMode: true } } } })).toBe(true);
    expect(vimBusyIn({ cm: { state: { vim: idle } } })).toBe(false);
  });
});
