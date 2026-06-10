import type { SnapshotMeta } from "../lib/types";
import { formatBytes, shortHash } from "../lib/format";

interface SnapshotsPanelProps {
  snapshots: SnapshotMeta[];
  onRestore: (snapshotId: number) => void;
}

export function SnapshotsPanel({ snapshots, onRestore }: SnapshotsPanelProps) {
  if (snapshots.length === 0) {
    return (
      <p className="small-text">
        No snapshots yet for this file. A pre-change snapshot is stored on every save.
      </p>
    );
  }
  return (
    <div className="table-container">
      <table className="audit-table">
        <thead>
          <tr>
            <th>Created</th>
            <th>Content hash</th>
            <th>Size</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {snapshots.map((snap) => (
            <tr key={snap.id}>
              <td>{snap.created_at}</td>
              <td>
                <code>{shortHash(snap.content_hash)}</code>
              </td>
              <td>{formatBytes(snap.size_bytes)}</td>
              <td>
                <button
                  type="button"
                  className="secondary-btn btn-compact"
                  onClick={() => onRestore(snap.id)}
                >
                  Restore
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
