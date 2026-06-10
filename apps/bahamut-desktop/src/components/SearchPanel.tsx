import { useRef, useState } from "react";
import type { SearchResponse } from "../lib/types";
import { cancelProjectSearch, searchProject } from "../lib/api";

interface SearchPanelProps {
  onOpenResult: (path: string, name: string, line: number) => void;
}

type Status = "idle" | "searching" | "done" | "error";

export function SearchPanel({ onOpenResult }: SearchPanelProps) {
  const [query, setQuery] = useState("");
  const [caseSensitive, setCaseSensitive] = useState(false);
  const [wholeWord, setWholeWord] = useState(false);
  const [regexMode, setRegexMode] = useState(false);
  const [status, setStatus] = useState<Status>("idle");
  const [error, setError] = useState<string | null>(null);
  const [results, setResults] = useState<SearchResponse | null>(null);
  // Stale-response guard: only the latest request may update the UI.
  const requestSeq = useRef(0);

  const runSearch = async () => {
    if (!query.trim() || status === "searching") {
      return;
    }
    const seq = ++requestSeq.current;
    setStatus("searching");
    setError(null);
    try {
      const resp = await searchProject({
        query,
        case_sensitive: caseSensitive,
        whole_word: wholeWord,
        regex: regexMode,
      });
      if (requestSeq.current !== seq) {
        return; // superseded
      }
      setResults(resp);
      setStatus("done");
    } catch (e) {
      if (requestSeq.current !== seq) {
        return;
      }
      setError(String(e));
      setStatus("error");
    }
  };

  const cancel = async () => {
    requestSeq.current++;
    try {
      await cancelProjectSearch();
    } catch {
      // Cancellation is best-effort; the stale-response guard covers the UI.
    }
    setStatus(results ? "done" : "idle");
  };

  return (
    <div className="search-panel">
      <form
        className="search-form"
        onSubmit={(e) => {
          e.preventDefault();
          void runSearch();
        }}
      >
        <input
          className="input-field input-compact search-input"
          placeholder="Search in project…"
          aria-label="Search query"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <div className="search-toggles" role="group" aria-label="Search options">
          <label className="toggle-chip" title="Case sensitive">
            <input
              type="checkbox"
              aria-label="Case sensitive"
              checked={caseSensitive}
              onChange={(e) => setCaseSensitive(e.target.checked)}
            />
            Aa
          </label>
          <label className="toggle-chip" title="Whole word">
            <input
              type="checkbox"
              aria-label="Whole word"
              checked={wholeWord}
              onChange={(e) => setWholeWord(e.target.checked)}
            />
            ⌊ab⌋
          </label>
          <label className="toggle-chip" title="Regular expression">
            <input
              type="checkbox"
              aria-label="Regular expression"
              checked={regexMode}
              onChange={(e) => setRegexMode(e.target.checked)}
            />
            .*
          </label>
          {status === "searching" ? (
            <button type="button" className="secondary-btn btn-compact" onClick={() => void cancel()}>
              Cancel
            </button>
          ) : (
            <button type="submit" className="primary-btn btn-compact" disabled={!query.trim()}>
              Search
            </button>
          )}
        </div>
      </form>

      {status === "searching" && <p className="small-text search-status">Searching…</p>}
      {status === "error" && <p className="status-error search-status">{error}</p>}

      {status === "done" && results && (
        <div className="search-results">
          <p className="small-text search-status">
            {results.total_matches} match{results.total_matches === 1 ? "" : "es"} in{" "}
            {results.files.length} file{results.files.length === 1 ? "" : "s"} (
            {results.files_scanned} scanned)
            {results.truncated && " — results truncated"}
            {results.timed_out && " — search timed out"}
            {results.cancelled && " — cancelled"}
          </p>
          {results.files.map((file) => (
            <div key={file.path} className="search-file-group">
              <p className="search-file-name" title={file.path}>
                {file.name}
              </p>
              {file.matches.map((m) => (
                <button
                  key={`${file.path}:${m.line}:${m.column}`}
                  type="button"
                  className="search-result-row"
                  onClick={() => onOpenResult(file.path, file.name, m.line)}
                >
                  <span className="search-line-no">{m.line}</span>
                  <span className="search-preview">{m.preview}</span>
                </button>
              ))}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
