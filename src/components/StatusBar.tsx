import { APP_VERSION } from "../types/index.ts";
import type { PtyState, AppMode } from "../types/index.ts";
import type { UpdaterState } from "../hooks/useUpdater.ts";
import { UpdateNotice } from "./UpdateNotice.tsx";

interface StatusBarProps {
  status: PtyState;
  mode: AppMode;
  onRestart: () => void;
  onBackToWelcome: () => void;
  updater: UpdaterState;
}

const STATUS_LABELS: Record<string, string> = {
  starting: "Starting...",
  running: "Running",
  stopped: "Stopped",
  error: "Error",
};

export function StatusBar({ status, mode, onRestart, onBackToWelcome, updater }: StatusBarProps) {
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
          <button type="button" className="status-btn" onClick={onRestart}>
            {mode === "gateway" ? "Restart gateway" : "Retry setup"}
          </button>
        )}
        {showActions && (
          <button type="button" className="status-btn" onClick={onBackToWelcome}>
            Back
          </button>
        )}
        {!showActions && (
          <UpdateNotice updater={updater} />
        )}
        {!showActions && (updater.status === "idle" || updater.status === "checking") && (
          <span>OpenClaw Desktop v{APP_VERSION}</span>
        )}
      </div>
    </div>
  );
}
