import { useRef } from "react";
import type { Tab } from "../state/tabs";

interface TabBarProps {
  tabs: Tab[];
  activePath: string | null;
  onActivate: (path: string) => void;
  /** Close request — the parent decides whether confirmation is needed. */
  onCloseRequest: (path: string) => void;
  onCloseAllRequest: () => void;
}

/**
 * Accessible tab strip: roving arrow-key focus, Enter/Space activate,
 * Delete and middle-click close, per-tab dirty indicator and close button.
 */
export function TabBar({
  tabs,
  activePath,
  onActivate,
  onCloseRequest,
  onCloseAllRequest,
}: TabBarProps) {
  const listRef = useRef<HTMLDivElement>(null);

  if (tabs.length === 0) {
    return null;
  }

  const focusSibling = (current: HTMLElement, direction: 1 | -1) => {
    const buttons = Array.from(
      listRef.current?.querySelectorAll<HTMLElement>("[role='tab']") ?? [],
    );
    const index = buttons.indexOf(current);
    const next = buttons[(index + direction + buttons.length) % buttons.length];
    next?.focus();
  };

  return (
    <div className="tab-bar">
      <div className="tab-list" role="tablist" aria-label="Open files" ref={listRef}>
        {tabs.map((tab) => {
          const isActive = tab.path === activePath;
          return (
            <div key={tab.path} className={`editor-tab${isActive ? " tab-current" : ""}`}>
              <button
                type="button"
                role="tab"
                aria-selected={isActive}
                tabIndex={isActive ? 0 : -1}
                className="tab-label"
                title={tab.path}
                onClick={() => onActivate(tab.path)}
                onAuxClick={(e) => {
                  if (e.button === 1) {
                    e.preventDefault();
                    onCloseRequest(tab.path);
                  }
                }}
                onKeyDown={(e) => {
                  if (e.key === "ArrowRight") {
                    e.preventDefault();
                    focusSibling(e.currentTarget, 1);
                  } else if (e.key === "ArrowLeft") {
                    e.preventDefault();
                    focusSibling(e.currentTarget, -1);
                  } else if (e.key === "Delete") {
                    e.preventDefault();
                    onCloseRequest(tab.path);
                  }
                }}
              >
                {tab.dirty && (
                  <span className="tab-dirty-dot" aria-label="Unsaved changes" title="Unsaved changes">
                    ●
                  </span>
                )}
                {tab.name}
              </button>
              <button
                type="button"
                className="tab-close"
                aria-label={`Close ${tab.name}`}
                onClick={() => onCloseRequest(tab.path)}
              >
                ×
              </button>
            </div>
          );
        })}
      </div>
      {tabs.length > 1 && (
        <button
          type="button"
          className="toggle-btn tab-close-all"
          onClick={onCloseAllRequest}
        >
          Close all
        </button>
      )}
    </div>
  );
}
