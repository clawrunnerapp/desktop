import { useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { PtyState, Settings } from "../types/index.ts";

interface PtyDataEvent {
  sessionId: number;
  data: string;
}

interface PtyStatusEvent {
  sessionId: number;
  status: string;
  errorMessage?: string;
}

interface UsePtySessionOptions {
  onData: (data: string) => void;
  onStatusChange: (state: PtyState) => void;
  settings: Settings;
  args: string[];
  initialSize: { cols: number; rows: number } | null;
}

export function usePtySession({ onData, onStatusChange, settings, args, initialSize }: UsePtySessionOptions) {
  const settingsRef = useRef(settings);
  settingsRef.current = settings;

  const argsRef = useRef(args);
  argsRef.current = args;

  const onDataRef = useRef(onData);
  onDataRef.current = onData;

  const onStatusChangeRef = useRef(onStatusChange);
  onStatusChangeRef.current = onStatusChange;

  const spawnedRef = useRef(false);
  const sessionIdRef = useRef<number>(0);

  useEffect(() => {
    if (spawnedRef.current || !initialSize) return;
    spawnedRef.current = true;

    let cancelled = false;
    const unlisteners: Array<() => void> = [];
    const { cols, rows } = initialSize;

    async function setup() {
      // Register listeners FIRST so no events are lost between spawn and listen.
      // Session ID filtering prevents stale events from prior sessions.
      // Accept events when sessionIdRef is 0 (before spawn returns) because
      // the Rust spawn() joins the old reader thread before starting the new one,
      // so only new-session events can arrive during this window.
      const matchesSession = (id: number) =>
        sessionIdRef.current === 0 || id === sessionIdRef.current;

      const unlisten1 = await listen<PtyDataEvent>("pty:data", (event) => {
        if (!cancelled && matchesSession(event.payload.sessionId)) {
          onDataRef.current(event.payload.data);
        }
      });
      if (cancelled) { unlisten1(); return; }
      unlisteners.push(unlisten1);

      const unlisten2 = await listen<PtyStatusEvent>("pty:status", (event) => {
        if (!cancelled && matchesSession(event.payload.sessionId)) {
          onStatusChangeRef.current({
            status: event.payload.status as PtyState["status"],
            errorMessage: event.payload.errorMessage,
          });
        }
      });
      if (cancelled) { unlisten2(); unlisten1(); return; }
      unlisteners.push(unlisten2);

      // Now spawn - events emitted after this will be caught by listeners above.
      if (cancelled) return;
      try {
        const sid = await invoke<number>("pty_spawn", {
          settings: settingsRef.current,
          args: argsRef.current,
          cols,
          rows,
        });
        if (cancelled) {
          invoke("pty_kill", { sessionId: sid }).catch(() => {});
          return;
        }
        sessionIdRef.current = sid;
        onStatusChangeRef.current({ status: "running" });
      } catch (err) {
        if (cancelled) return;
        console.error("[pty] Spawn failed:", err);
        onStatusChangeRef.current({
          status: "error",
          errorMessage: String(err),
        });
      }
    }

    setup();

    return () => {
      cancelled = true;
      unlisteners.forEach((fn) => fn());
      if (sessionIdRef.current > 0) {
        invoke("pty_kill", { sessionId: sessionIdRef.current }).catch(() => {});
      }
    };
  }, [initialSize]); // eslint-disable-line react-hooks/exhaustive-deps

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

  return { write, resize };
}
