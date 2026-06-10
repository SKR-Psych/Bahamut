/** Maps a file name to a Monaco language id (best effort; defaults to plaintext). */

const EXTENSION_LANGUAGES: Record<string, string> = {
  ts: "typescript",
  tsx: "typescript",
  mts: "typescript",
  cts: "typescript",
  js: "javascript",
  jsx: "javascript",
  mjs: "javascript",
  cjs: "javascript",
  json: "json",
  html: "html",
  htm: "html",
  css: "css",
  scss: "scss",
  less: "less",
  md: "markdown",
  markdown: "markdown",
  rs: "rust",
  py: "python",
  rb: "ruby",
  go: "go",
  java: "java",
  c: "c",
  h: "c",
  cpp: "cpp",
  cc: "cpp",
  hpp: "cpp",
  cs: "csharp",
  php: "php",
  sql: "sql",
  sh: "shell",
  bash: "shell",
  ps1: "powershell",
  psm1: "powershell",
  yaml: "yaml",
  yml: "yaml",
  toml: "ini",
  ini: "ini",
  xml: "xml",
  svg: "xml",
  dockerfile: "dockerfile",
  graphql: "graphql",
  lua: "lua",
  swift: "swift",
  kt: "kotlin",
};

const SPECIAL_FILENAMES: Record<string, string> = {
  dockerfile: "dockerfile",
  makefile: "makefile",
  "cargo.lock": "ini",
};

export function languageForFile(fileName: string): string {
  const lower = fileName.toLowerCase();
  if (SPECIAL_FILENAMES[lower]) {
    return SPECIAL_FILENAMES[lower];
  }
  const dot = lower.lastIndexOf(".");
  if (dot < 0 || dot === lower.length - 1) {
    return "plaintext";
  }
  const ext = lower.slice(dot + 1);
  return EXTENSION_LANGUAGES[ext] ?? "plaintext";
}
