import { useEffect, useRef } from "react";
import { monaco } from "../lib/monaco";
import { languageForFile } from "../lib/language";

interface EditorPaneProps {
  filePath: string;
  fileName: string;
  /** Content as read from disk; replaces the buffer when filePath changes. */
  content: string;
  /** Bumped by the parent to force a buffer reset (reload / rollback). */
  contentVersion: number;
  onDirtyChange: (dirty: boolean) => void;
  onRequestSave: (currentText: string) => void;
}

/**
 * Thin Monaco wrapper. The parent owns the file lifecycle (read/save); this
 * component owns only the buffer and dirty tracking.
 */
export function EditorPane({
  filePath,
  fileName,
  content,
  contentVersion,
  onDirtyChange,
  onRequestSave,
}: EditorPaneProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const baselineRef = useRef(content);
  const callbacksRef = useRef({ onDirtyChange, onRequestSave });
  callbacksRef.current = { onDirtyChange, onRequestSave };

  // Create the editor once.
  useEffect(() => {
    if (!containerRef.current) {
      return;
    }
    const editor = monaco.editor.create(containerRef.current, {
      value: "",
      theme: "bahamut-dark",
      automaticLayout: true,
      minimap: { enabled: false },
      fontSize: 13,
      scrollBeyondLastLine: false,
    });
    editor.onDidChangeModelContent(() => {
      callbacksRef.current.onDirtyChange(editor.getValue() !== baselineRef.current);
    });
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => {
      callbacksRef.current.onRequestSave(editor.getValue());
    });
    // The toolbar Save button triggers the same save path as Ctrl+S.
    const onExternalSaveRequest = () => {
      callbacksRef.current.onRequestSave(editor.getValue());
    };
    window.addEventListener("bahamut:request-save", onExternalSaveRequest);
    editorRef.current = editor;
    return () => {
      window.removeEventListener("bahamut:request-save", onExternalSaveRequest);
      editor.dispose();
      editorRef.current = null;
    };
  }, []);

  // Reset the buffer whenever a different file (or fresh content) arrives.
  useEffect(() => {
    const editor = editorRef.current;
    if (!editor) {
      return;
    }
    baselineRef.current = content;
    const model = editor.getModel();
    if (model) {
      monaco.editor.setModelLanguage(model, languageForFile(fileName));
    }
    editor.setValue(content);
    callbacksRef.current.onDirtyChange(false);
  }, [filePath, fileName, content, contentVersion]);

  return <div ref={containerRef} className="editor-host" data-path={filePath} />;
}
