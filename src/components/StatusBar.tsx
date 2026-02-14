import { APP_VERSION } from "../types/index.ts";
import type { PtyState, AppMode } from "../types/index.ts";

interface StatusBarProps {
  status: PtyState;
  mode: AppMode;
  onRestart: () => void;
  onBackToWelcome: () => void;
}

const STATUS_LABELS: Record<string, string> = {
  starting: "Starting...",
  running: "Running",
  stopped: "Stopped",
  error: "Error",
};

export function StatusBar({ status, mode, onRestart, onBackToWelcome }: StatusBarProps) {
  const showActions = status.status === "stopped" || status.status === "error";

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
      <div className="status-actions">
        {showActions && (mode === "gateway" || mode === "onboard") && (
          <button className="status-btn" onClick={onRestart}>
            {mode === "gateway" ? "Restart gateway" : "Retry setup"}
          </button>
        )}
        {showActions && (
          <button className="status-btn" onClick={onBackToWelcome}>
            Back
          </button>
        )}
        {!showActions && (
          <span>OpenClaw Desktop v{APP_VERSION}</span>
        )}
      </div>
    </div>
  );
}
