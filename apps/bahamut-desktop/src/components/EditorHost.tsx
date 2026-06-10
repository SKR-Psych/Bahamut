import { forwardRef, useEffect, useImperativeHandle, useRef } from "react";
import { monaco } from "../lib/monaco";
import { languageForFile } from "../lib/language";

export interface EditorFile {
  path: string;
  name: string;
  /** Content as last read from disk. */
  initialContent: string;
  /** Bump to force the buffer to reset to initialContent (reload/restore). */
  version: number;
}

export interface EditorHostHandle {
  /** Marks a file's buffer as saved: its dirty baseline becomes `text`. */
  markSaved(path: string, text: string): void;
  /** Current buffer text (unsaved edits included). */
  getText(path: string): string | null;
  /** Scrolls the active editor to a line (used by search results). */
  revealLine(path: string, line: number): void;
}

interface EditorHostProps {
  files: EditorFile[];
  activePath: string | null;
  onDirtyChange: (path: string, dirty: boolean) => void;
  onRequestSave: (path: string, currentText: string) => void;
}

interface ModelEntry {
  model: monaco.editor.ITextModel;
  baseline: string;
  version: number;
  viewState: monaco.editor.ICodeEditorViewState | null;
  listener: { dispose(): void };
}

/**
 * One Monaco editor instance hosting one model per open tab. Models keep
 * their own undo stacks and unsaved buffers; closing a tab disposes its
 * model (the Workspace confirms dirty closes first).
 */
export const EditorHost = forwardRef<EditorHostHandle, EditorHostProps>(function EditorHost(
  { files, activePath, onDirtyChange, onRequestSave },
  ref,
) {
  const containerRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null);
  const modelsRef = useRef<Map<string, ModelEntry>>(new Map());
  const activePathRef = useRef<string | null>(null);
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
    const save = () => {
      const path = activePathRef.current;
      if (path) {
        callbacksRef.current.onRequestSave(path, editor.getValue());
      }
    };
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, save);
    window.addEventListener("bahamut:request-save", save);
    editorRef.current = editor;
    const models = modelsRef.current;
    return () => {
      window.removeEventListener("bahamut:request-save", save);
      editor.dispose();
      editorRef.current = null;
      for (const entry of models.values()) {
        entry.listener.dispose();
        entry.model.dispose();
      }
      models.clear();
    };
  }, []);

  // Sync models with the open-file list.
  useEffect(() => {
    const models = modelsRef.current;
    const open = new Set(files.map((f) => f.path));

    for (const [path, entry] of models) {
      if (!open.has(path)) {
        entry.listener.dispose();
        entry.model.dispose();
        models.delete(path);
      }
    }
    for (const file of files) {
      const existing = models.get(file.path);
      if (!existing) {
        const model = monaco.editor.createModel(
          file.initialContent,
          languageForFile(file.name),
        );
        const entry: ModelEntry = {
          model,
          baseline: file.initialContent,
          version: file.version,
          viewState: null,
          listener: { dispose: () => undefined },
        };
        entry.listener = model.onDidChangeContent(() => {
          callbacksRef.current.onDirtyChange(file.path, model.getValue() !== entry.baseline);
        });
        models.set(file.path, entry);
      } else if (file.version > existing.version) {
        // Reload / snapshot restore: replace the buffer with disk content.
        existing.model.setValue(file.initialContent);
        existing.baseline = file.initialContent;
        existing.version = file.version;
        callbacksRef.current.onDirtyChange(file.path, false);
      }
    }
  }, [files]);

  // Switch the visible model when the active tab changes.
  useEffect(() => {
    const editor = editorRef.current;
    if (!editor) {
      return;
    }
    const previous = activePathRef.current;
    if (previous) {
      const entry = modelsRef.current.get(previous);
      if (entry) {
        entry.viewState = editor.saveViewState();
      }
    }
    activePathRef.current = activePath;
    if (activePath) {
      const entry = modelsRef.current.get(activePath);
      if (entry) {
        editor.setModel(entry.model);
        if (entry.viewState) {
          editor.restoreViewState(entry.viewState);
        }
        editor.focus();
      }
    } else {
      editor.setModel(null);
    }
  }, [activePath, files]);

  useImperativeHandle(ref, () => ({
    markSaved(path: string, text: string) {
      const entry = modelsRef.current.get(path);
      if (entry) {
        entry.baseline = text;
        callbacksRef.current.onDirtyChange(path, entry.model.getValue() !== text);
      }
    },
    getText(path: string) {
      return modelsRef.current.get(path)?.model.getValue() ?? null;
    },
    revealLine(path: string, line: number) {
      const editor = editorRef.current;
      if (editor && activePathRef.current === path) {
        editor.revealLineInCenter(line);
        editor.setPosition({ lineNumber: line, column: 1 });
        editor.focus();
      }
    },
  }));

  return <div ref={containerRef} className="editor-host" />;
});
