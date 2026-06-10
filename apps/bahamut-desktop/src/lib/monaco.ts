/**
 * Monaco is bundled locally by Vite (no CDN: local-first + CSP `script-src
 * 'self'`). Web workers are emitted as separate chunks and loaded via
 * `worker-src 'self' blob:`.
 */
import * as monaco from "monaco-editor";
import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import jsonWorker from "monaco-editor/esm/vs/language/json/json.worker?worker";
import cssWorker from "monaco-editor/esm/vs/language/css/css.worker?worker";
import htmlWorker from "monaco-editor/esm/vs/language/html/html.worker?worker";
import tsWorker from "monaco-editor/esm/vs/language/typescript/ts.worker?worker";

self.MonacoEnvironment = {
  getWorker(_workerId: string, label: string): Worker {
    switch (label) {
      case "json":
        return new jsonWorker();
      case "css":
      case "scss":
      case "less":
        return new cssWorker();
      case "html":
      case "handlebars":
      case "razor":
        return new htmlWorker();
      case "typescript":
      case "javascript":
        return new tsWorker();
      default:
        return new editorWorker();
    }
  },
};

monaco.editor.defineTheme("bahamut-dark", {
  base: "vs-dark",
  inherit: true,
  rules: [],
  colors: {
    "editor.background": "#0B0B0A",
    "editor.foreground": "#F5F5F4",
    "editorLineNumber.foreground": "#62625D",
    "editorCursor.foreground": "#B98A84",
    "editor.selectionBackground": "#6F744855",
    "editor.lineHighlightBackground": "#16161480",
  },
});

export { monaco };
