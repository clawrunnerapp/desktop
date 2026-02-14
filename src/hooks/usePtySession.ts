import { useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { PtyState, Settings } from "../types/index.ts";

interface UsePtySessionOptions {
  onData: (data: string) => void;
  onStatusChange: (state: PtyState) => void;
  settings: Settings;
}

export function usePtySession({ onData, onStatusChange, settings }: UsePtySessionOptions) {
  const settingsRef = useRef(settings);
  settingsRef.current = settings;

  useEffect(() => {
    const unlisteners: Array<() => void> = [];

    async function setup() {
      const unlisten1 = await listen<string>("pty:data", (event) => {
        onData(event.payload);
      });
      unlisteners.push(unlisten1);

      const unlisten2 = await listen<PtyState>("pty:status", (event) => {
        onStatusChange(event.payload);
      });
      unlisteners.push(unlisten2);

      try {
        await invoke("pty_spawn", { settings: settingsRef.current });
        onStatusChange({ status: "running" });
      } catch (err) {
        onStatusChange({
          status: "error",
          errorMessage: String(err),
        });
      }
    }

    setup();

    return () => {
      unlisteners.forEach((fn) => fn());
      invoke("pty_kill").catch(() => {});
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const write = useCallback(async (data: string) => {
    try {
      await invoke("pty_write", { data });
    } catch {
      // PTY may be dead; status event will handle it
    }
  }, []);

  const resize = useCallback(async (cols: number, rows: number) => {
    try {
      await invoke("pty_resize", { cols, rows });
    } catch {
      // Ignore resize errors
    }
  }, []);

  const restart = useCallback(async () => {
    onStatusChange({ status: "starting" });
    try {
      await invoke("pty_kill");
    } catch {
      // May already be dead
    }
    try {
      await invoke("pty_spawn", { settings: settingsRef.current });
      onStatusChange({ status: "running" });
    } catch (err) {
      onStatusChange({ status: "error", errorMessage: String(err) });
    }
  }, [onStatusChange]);

  return { write, resize, restart };
}
