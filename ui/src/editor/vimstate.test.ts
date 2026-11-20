import { describe, expect, it } from "vitest";
import { describeVim, type VimInputState } from "./vimstate";

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
