import type { AuditLogEntry, ChainVerification } from "../lib/types";

interface AuditPanelProps {
  logs: AuditLogEntry[];
  chain: ChainVerification | null;
}

export function AuditPanel({ logs, chain }: AuditPanelProps) {
  return (
    <div>
      {chain && (
        <p className={chain.valid ? "status-success" : "status-error"}>
          {chain.valid
            ? `● Audit chain verified (${chain.entries_checked} entries)`
            : `✕ Audit chain BROKEN at seq ${chain.first_broken_seq}: ${chain.detail ?? ""}`}
        </p>
      )}
      <div className="table-container">
        <table className="audit-table">
          <thead>
            <tr>
              <th>Seq</th>
              <th>Timestamp</th>
              <th>Action</th>
              <th>Details</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            {logs.length === 0 ? (
              <tr>
                <td colSpan={5} style={{ textAlign: "center" }}>
                  No logs recorded yet.
                </td>
              </tr>
            ) : (
              logs.map((log) => (
                <tr key={log.id}>
                  <td>{log.seq}</td>
                  <td>{log.timestamp}</td>
                  <td>
                    <code>{log.action_type}</code>
                  </td>
                  <td className="audit-details">{log.details || "-"}</td>
                  <td>
                    <span className={log.status === "success" ? "badge-success" : "badge-error"}>
                      {log.status}
                    </span>
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
