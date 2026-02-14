import type { PtyState } from "../types/index.ts";

interface StatusBarProps {
  status: PtyState;
}

const STATUS_LABELS: Record<string, string> = {
  starting: "Starting CLI...",
  running: "CLI Running",
  stopped: "CLI Stopped",
  error: "Error",
};

export function StatusBar({ status }: StatusBarProps) {
  return (
    <div className="status-bar">
      <div className="status-indicator">
        <span className={`status-dot ${status.status}`} />
        <span>{STATUS_LABELS[status.status]}</span>
        {status.errorMessage && (
          <span style={{ color: "#ef4444", marginLeft: 8 }}>
            {status.errorMessage}
          </span>
        )}
      </div>
      <span>OpenClaw Desktop v0.1.0</span>
    </div>
  );
}
