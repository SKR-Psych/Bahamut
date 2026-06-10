import { describe, expect, it } from "vitest";
import {
  closeAllTabs,
  closeTab,
  cycleTab,
  emptyTabs,
  focusTab,
  openTab,
  setTabDirty,
  tabsUnderPath,
} from "../tabs";

const fileA = { path: "C:\\proj\\a.ts", name: "a.ts" };
const fileB = { path: "C:\\proj\\b.ts", name: "b.ts" };
const fileC = { path: "C:\\proj\\src\\c.ts", name: "c.ts" };

describe("tabs state", () => {
  it("opens multiple tabs and activates the newest", () => {
    let s = openTab(emptyTabs, fileA);
    s = openTab(s, fileB);
    expect(s.tabs.map((t) => t.name)).toEqual(["a.ts", "b.ts"]);
    expect(s.activePath).toBe(fileB.path);
  });

  it("reopening an open file focuses the existing tab without duplicating", () => {
    let s = openTab(openTab(emptyTabs, fileA), fileB);
    s = openTab(s, fileA);
    expect(s.tabs).toHaveLength(2);
    expect(s.activePath).toBe(fileA.path);
  });

  it("tracks dirty state per tab", () => {
    let s = openTab(openTab(emptyTabs, fileA), fileB);
    s = setTabDirty(s, fileA.path, true);
    expect(s.tabs.find((t) => t.path === fileA.path)?.dirty).toBe(true);
    expect(s.tabs.find((t) => t.path === fileB.path)?.dirty).toBe(false);
  });

  it("closing the active tab activates a neighbour", () => {
    let s = openTab(openTab(openTab(emptyTabs, fileA), fileB), fileC);
    s = focusTab(s, fileB.path);
    s = closeTab(s, fileB.path);
    expect(s.tabs).toHaveLength(2);
    expect(s.activePath).toBe(fileC.path);
  });

  it("closing the last tab clears the active path", () => {
    let s = openTab(emptyTabs, fileA);
    s = closeTab(s, fileA.path);
    expect(s.tabs).toHaveLength(0);
    expect(s.activePath).toBeNull();
  });

  it("closes all tabs", () => {
    const s = openTab(openTab(emptyTabs, fileA), fileB);
    expect(closeAllTabs(s)).toEqual(emptyTabs);
  });

  it("cycles forward and backward with wrap-around", () => {
    let s = openTab(openTab(openTab(emptyTabs, fileA), fileB), fileC);
    expect(cycleTab(s, 1).activePath).toBe(fileA.path); // wraps from c -> a
    s = focusTab(s, fileA.path);
    expect(cycleTab(s, -1).activePath).toBe(fileC.path);
  });

  it("finds tabs under a folder path for rename/delete handling", () => {
    const s = openTab(openTab(emptyTabs, fileA), fileC);
    const under = tabsUnderPath(s, "C:\\proj\\src");
    expect(under.map((t) => t.name)).toEqual(["c.ts"]);
    const exact = tabsUnderPath(s, fileA.path);
    expect(exact.map((t) => t.name)).toEqual(["a.ts"]);
  });
});
