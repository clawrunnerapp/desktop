import type { UpdaterState } from "../hooks/useUpdater.ts";

interface UpdateNoticeProps {
  updater: UpdaterState;
}

export function UpdateNotice({ updater }: UpdateNoticeProps) {
  const { status, version, progress, error, downloadAndInstall, dismiss } = updater;

  if (status === "available" && version) {
    return (
      <span className="update-notice">
        <span>Update v{version} available</span>
        <button type="button" className="status-btn status-btn-primary" onClick={downloadAndInstall}>
          Update
        </button>
        <button type="button" className="status-btn" onClick={dismiss}>
          Later
        </button>
      </span>
    );
  }

  if (status === "downloading") {
    return (
      <span className="update-notice">
        <span>{progress < 0 ? "Downloading…" : `Downloading… ${progress}%`}</span>
        <span className={`update-progress-bar${progress < 0 ? " update-progress-indeterminate" : ""}`} aria-hidden="true">
          {progress >= 0 && (
            <span className="update-progress-fill" style={{ width: `${progress}%` }} />
          )}
        </span>
      </span>
    );
  }

  if (status === "error") {
    const canRetry = version !== null;
    return (
      <span className="update-notice">
        <span className="update-notice-error" title={canRetry ? `v${version} update failed` : error!}>
          {canRetry ? `v${version} update failed` : error!}
        </span>
        {canRetry && (
          <button type="button" className="status-btn status-btn-primary" onClick={downloadAndInstall}>
            Retry
          </button>
        )}
        <button type="button" className="status-btn" onClick={dismiss}>
          Dismiss
        </button>
      </span>
    );
  }

  return null;
}
