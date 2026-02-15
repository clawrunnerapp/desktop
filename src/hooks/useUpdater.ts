import { useState, useEffect, useCallback, useRef } from "react";
import { check } from "@tauri-apps/plugin-updater";
import type { Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type UpdateStatus =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "error";

export interface UpdaterState {
  status: UpdateStatus;
  version: string | null;
  progress: number;
  error: string | null;
  downloadAndInstall: () => void;
  dismiss: () => void;
}

const CHECK_DELAY_MS = 5_000;
const CHECK_INTERVAL_MS = 60 * 60 * 1000;
/** Sentinel value for `progress` indicating unknown total size. */
const PROGRESS_INDETERMINATE = -1;

export function useUpdater(): UpdaterState {
  const [status, setStatus] = useState<UpdateStatus>("idle");
  const [version, setVersion] = useState<string | null>(null);
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState<string | null>(null);

  const updateRef = useRef<Update | null>(null);
  const statusRef = useRef<UpdateStatus>("idle");
  const dismissedVersionRef = useRef<string | null>(null);
  const unmountedRef = useRef(false);

  useEffect(() => {
    return () => { unmountedRef.current = true; };
  }, []);

  const doCheck = useCallback(async () => {
    if (statusRef.current !== "idle" && statusRef.current !== "error") return;
    statusRef.current = "checking";
    setStatus("checking");
    setVersion(null);
    setError(null);

    try {
      const update = await check();
      if (unmountedRef.current) return;
      if (update) {
        if (dismissedVersionRef.current === update.version) {
          statusRef.current = "idle";
          setStatus("idle");
          return;
        }
        updateRef.current = update;
        setVersion(update.version);
        statusRef.current = "available";
        setStatus("available");
      } else {
        statusRef.current = "idle";
        setStatus("idle");
      }
    } catch (e) {
      if (unmountedRef.current) return;
      statusRef.current = "error";
      setStatus("error");
      setError(e instanceof Error ? e.message : "Update check failed");
    }
  }, []);

  useEffect(() => {
    const timeout = setTimeout(doCheck, CHECK_DELAY_MS);
    const interval = setInterval(doCheck, CHECK_INTERVAL_MS);
    return () => {
      clearTimeout(timeout);
      clearInterval(interval);
    };
  }, [doCheck]);

  const downloadAndInstall = useCallback(async () => {
    if (statusRef.current !== "available" && statusRef.current !== "error") return;
    const update = updateRef.current;
    if (!update) return;

    statusRef.current = "downloading";
    setStatus("downloading");
    setProgress(0);
    setError(null);

    try {
      let totalLength = 0;
      let downloaded = 0;

      await update.downloadAndInstall((event) => {
        if (unmountedRef.current) return;
        if (event.event === "Started") {
          totalLength = event.data.contentLength ?? 0;
          if (totalLength === 0) {
            setProgress(PROGRESS_INDETERMINATE);
          }
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          if (totalLength > 0) {
            setProgress(Math.min(Math.round((downloaded / totalLength) * 100), 100));
          }
        } else if (event.event === "Finished") {
          setProgress(100);
        }
      });

      if (unmountedRef.current) return;
      await relaunch();
    } catch (e) {
      if (unmountedRef.current) return;
      statusRef.current = "error";
      setStatus("error");
      setError(e instanceof Error ? e.message : "Update download failed");
    }
  }, []);

  const dismiss = useCallback(() => {
    if (statusRef.current === "available") {
      const update = updateRef.current;
      if (update) {
        dismissedVersionRef.current = update.version;
      }
    }
    updateRef.current = null;
    statusRef.current = "idle";
    setStatus("idle");
    setVersion(null);
    setError(null);
  }, []);

  return { status, version, progress, error, downloadAndInstall, dismiss };
}
