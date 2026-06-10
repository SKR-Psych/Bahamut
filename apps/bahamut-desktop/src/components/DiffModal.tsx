import { useEffect, useRef } from "react";
import { monaco } from "../lib/monaco";
import { languageForFile } from "../lib/language";

interface DiffModalProps {
  fileName: string;
  /** Left side: the selected snapshot. */
  snapshotLabel: string;
  snapshotContent: string;
  /** Right side: the current on-disk/editor content. */
  currentContent: string;
  onClose: () => void;
}

/** Read-only Monaco diff between a snapshot and the current content. */
export function DiffModal({
  fileName,
  snapshotLabel,
  snapshotContent,
  currentContent,
  onClose,
}: DiffModalProps) {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!containerRef.current) {
      return;
    }
    const language = languageForFile(fileName);
    const original = monaco.editor.createModel(snapshotContent, language);
    const modified = monaco.editor.createModel(currentContent, language);
    const diff = monaco.editor.createDiffEditor(containerRef.current, {
      theme: "bahamut-dark",
      readOnly: true,
      renderSideBySide: true,
      automaticLayout: true,
      minimap: { enabled: false },
    });
    diff.setModel({ original, modified });

    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      diff.dispose();
      original.dispose();
      modified.dispose();
    };
  }, [fileName, snapshotContent, currentContent, onClose]);

  return (
    <div className="modal-backdrop" role="presentation" onClick={onClose}>
      <div
        className="modal-card diff-modal"
        role="dialog"
        aria-modal="true"
        aria-label={`Diff of ${fileName}`}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="diff-header">
          <span>
            <strong>{snapshotLabel}</strong> (left) vs <strong>current content</strong> (right) —{" "}
            {fileName}
          </span>
          <button type="button" className="secondary-btn btn-compact" onClick={onClose}>
            Close
          </button>
        </div>
        <div ref={containerRef} className="diff-host" />
      </div>
    </div>
  );
}
