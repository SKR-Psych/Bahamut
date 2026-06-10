import type { SnapshotMeta } from "../lib/types";
import { formatBytes, shortHash } from "../lib/format";

interface SnapshotsPanelProps {
  fileName: string;
  /** Hash of the file content currently on disk (from the last read/save). */
  currentHash: string | null;
  snapshots: SnapshotMeta[];
  onRestoreRequest: (snapshot: SnapshotMeta) => void;
  onDiffRequest: (snapshot: SnapshotMeta) => void;
}

const OPERATION_LABELS: Record<string, string> = {
  save: "Before save",
  "pre-rollback": "Before restore",
  "pre-delete": "Before delete",
};

function operationLabel(op: string): string {
  return OPERATION_LABELS[op] ?? op;
}

export function SnapshotsPanel({
  fileName,
  currentHash,
  snapshots,
  onRestoreRequest,
  onDiffRequest,
}: SnapshotsPanelProps) {
  if (snapshots.length === 0) {
    return (
      <p className="small-text">
        No snapshots yet for {fileName}. A snapshot of the previous content is stored on every
        save, and restores are themselves undoable.
      </p>
    );
  }
  return (
    <div>
      <p className="small-text">
        Snapshots of <strong>{fileName}</strong>, newest first. Restoring creates a new snapshot
        of the current content, so a restore can always be undone.
      </p>
      <div className="table-container">
        <table className="audit-table">
          <thead>
            <tr>
              <th>Captured</th>
              <th>Operation</th>
              <th>Content</th>
              <th>Size</th>
              <th>Vs. current</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {snapshots.map((snap) => {
              const matchesCurrent = currentHash !== null && snap.content_hash === currentHash;
              return (
                <tr key={snap.id}>
                  <td>{snap.created_at}</td>
                  <td>{operationLabel(snap.operation)}</td>
                  <td>
                    <code>{shortHash(snap.content_hash)}</code>
                  </td>
                  <td>{formatBytes(snap.size_bytes)}</td>
                  <td>
                    {matchesCurrent ? (
                      <span className="badge-success">same as current</span>
                    ) : (
                      <span className="badge-muted">differs</span>
                    )}
                  </td>
                  <td className="snapshot-actions">
                    <button
                      type="button"
                      className="secondary-btn btn-compact"
                      onClick={() => onDiffRequest(snap)}
                    >
                      Diff
                    </button>
                    <button
                      type="button"
                      className="secondary-btn btn-compact"
                      disabled={matchesCurrent}
                      title={matchesCurrent ? "Identical to the current content" : undefined}
                      onClick={() => onRestoreRequest(snap)}
                    >
                      Restore
                    </button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}
