import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { TabBar } from "../TabBar";
import type { Tab } from "../../state/tabs";

const tabs: Tab[] = [
  { path: "C:\\p\\a.ts", name: "a.ts", dirty: false },
  { path: "C:\\p\\b.ts", name: "b.ts", dirty: true },
];

function setup(activePath = tabs[0].path) {
  const onActivate = vi.fn();
  const onCloseRequest = vi.fn();
  const onCloseAllRequest = vi.fn();
  render(
    <TabBar
      tabs={tabs}
      activePath={activePath}
      onActivate={onActivate}
      onCloseRequest={onCloseRequest}
      onCloseAllRequest={onCloseAllRequest}
    />,
  );
  return { onActivate, onCloseRequest, onCloseAllRequest };
}

describe("TabBar", () => {
  it("marks the active tab as selected", () => {
    setup();
    const tabButtons = screen.getAllByRole("tab");
    expect(tabButtons[0]).toHaveAttribute("aria-selected", "true");
    expect(tabButtons[1]).toHaveAttribute("aria-selected", "false");
  });

  it("shows a dirty indicator only on tabs with unsaved changes", () => {
    setup();
    const dirtyDots = screen.getAllByTitle("Unsaved changes");
    expect(dirtyDots).toHaveLength(1);
    expect(screen.getByRole("tab", { name: /b\.ts/ })).toContainElement(dirtyDots[0]);
  });

  it("activates a tab on click and requests close from the close button", () => {
    const { onActivate, onCloseRequest } = setup();
    fireEvent.click(screen.getByRole("tab", { name: /b\.ts/ }));
    expect(onActivate).toHaveBeenCalledWith(tabs[1].path);
    fireEvent.click(screen.getByRole("button", { name: "Close a.ts" }));
    expect(onCloseRequest).toHaveBeenCalledWith(tabs[0].path);
  });

  it("requests close on middle-click", () => {
    const { onCloseRequest } = setup();
    fireEvent(
      screen.getByRole("tab", { name: /b\.ts/ }),
      new MouseEvent("auxclick", { bubbles: true, button: 1 }),
    );
    expect(onCloseRequest).toHaveBeenCalledWith(tabs[1].path);
  });

  it("supports keyboard navigation between tabs", () => {
    setup();
    const tabButtons = screen.getAllByRole("tab");
    tabButtons[0].focus();
    fireEvent.keyDown(tabButtons[0], { key: "ArrowRight" });
    expect(tabButtons[1]).toHaveFocus();
    fireEvent.keyDown(tabButtons[1], { key: "ArrowLeft" });
    expect(tabButtons[0]).toHaveFocus();
  });

  it("offers close-all when several tabs are open", () => {
    const { onCloseAllRequest } = setup();
    fireEvent.click(screen.getByRole("button", { name: "Close all" }));
    expect(onCloseAllRequest).toHaveBeenCalled();
  });
});
