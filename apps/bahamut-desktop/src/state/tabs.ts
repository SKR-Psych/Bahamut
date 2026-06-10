/**
 * Pure tab-state transitions, kept free of React and Monaco so the open /
 * focus / close / dirty rules are unit-testable. The Workspace component owns
 * the state; EditorHost owns the corresponding Monaco models.
 */

export interface Tab {
  path: string;
  name: string;
  dirty: boolean;
}

export interface TabsState {
  tabs: Tab[];
  activePath: string | null;
}

export const emptyTabs: TabsState = { tabs: [], activePath: null };

/** Opens a file: reopening an already-open path focuses its existing tab. */
export function openTab(state: TabsState, file: { path: string; name: string }): TabsState {
  const existing = state.tabs.find((t) => t.path === file.path);
  if (existing) {
    return { ...state, activePath: existing.path };
  }
  return {
    tabs: [...state.tabs, { path: file.path, name: file.name, dirty: false }],
    activePath: file.path,
  };
}

/** Closes a tab; if it was active, activates the nearest neighbour. */
export function closeTab(state: TabsState, path: string): TabsState {
  const index = state.tabs.findIndex((t) => t.path === path);
  if (index < 0) {
    return state;
  }
  const tabs = state.tabs.filter((t) => t.path !== path);
  let activePath = state.activePath;
  if (state.activePath === path) {
    const neighbour = tabs[Math.min(index, tabs.length - 1)];
    activePath = neighbour ? neighbour.path : null;
  }
  return { tabs, activePath };
}

export function closeAllTabs(_state: TabsState): TabsState {
  return emptyTabs;
}

export function focusTab(state: TabsState, path: string): TabsState {
  if (!state.tabs.some((t) => t.path === path)) {
    return state;
  }
  return { ...state, activePath: path };
}

export function setTabDirty(state: TabsState, path: string, dirty: boolean): TabsState {
  const tab = state.tabs.find((t) => t.path === path);
  if (!tab || tab.dirty === dirty) {
    return state;
  }
  return {
    ...state,
    tabs: state.tabs.map((t) => (t.path === path ? { ...t, dirty } : t)),
  };
}

/** Cycles focus forward (+1) or backward (-1) through the tab strip. */
export function cycleTab(state: TabsState, direction: 1 | -1): TabsState {
  if (state.tabs.length === 0 || state.activePath === null) {
    return state;
  }
  const index = state.tabs.findIndex((t) => t.path === state.activePath);
  const next = (index + direction + state.tabs.length) % state.tabs.length;
  return { ...state, activePath: state.tabs[next].path };
}

/** Returns the open tabs located at or under `path` (file or folder). */
export function tabsUnderPath(state: TabsState, path: string): Tab[] {
  const sep = path.includes("/") ? "/" : "\\";
  const prefix = path.endsWith(sep) ? path : path + sep;
  return state.tabs.filter((t) => t.path === path || t.path.startsWith(prefix));
}
