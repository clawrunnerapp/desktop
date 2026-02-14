import { useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { PtyState, PtyStatus, Settings } from "../types/index.ts";

interface PtyDataEvent {
  sessionId: number;
  data: string;
}

interface PtyStatusEvent {
  sessionId: number;
  status: string;
  errorMessage?: string;
}

const VALID_PTY_STATUSES: ReadonlySet<string> = new Set<PtyStatus>(["starting", "running", "stopped", "error"]);

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
    const unlistenFns: Array<() => void> = [];
    const listenPromises: Array<Promise<() => void>> = [];
    const { cols, rows } = initialSize;

    // Buffer events received before session ID is known (fast-exit race).
    // Once the session ID is set, buffered events matching the ID are replayed.
    const pendingDataEvents: PtyDataEvent[] = [];
    const pendingStatusEvents: PtyStatusEvent[] = [];
    let sessionKnown = false;

    function handleDataEvent(payload: PtyDataEvent) {
      if (cancelled) return;
      if (!sessionKnown) {
        pendingDataEvents.push(payload);
        return;
      }
      if (payload.sessionId === sessionIdRef.current) {
        onDataRef.current(payload.data);
      }
    }

    function handleStatusEvent(payload: PtyStatusEvent) {
      if (cancelled) return;
      if (!sessionKnown) {
        pendingStatusEvents.push(payload);
        return;
      }
      if (payload.sessionId === sessionIdRef.current) {
        const status: PtyStatus = VALID_PTY_STATUSES.has(payload.status)
          ? (payload.status as PtyStatus)
          : "error";
        onStatusChangeRef.current({ status, errorMessage: payload.errorMessage });
      }
    }

    function drainPendingEvents(sid: number) {
      sessionKnown = true;
      for (const evt of pendingDataEvents) {
        if (!cancelled && evt.sessionId === sid) {
          onDataRef.current(evt.data);
        }
      }
      pendingDataEvents.length = 0;
      for (const evt of pendingStatusEvents) {
        if (!cancelled && evt.sessionId === sid) {
          const status: PtyStatus = VALID_PTY_STATUSES.has(evt.status)
            ? (evt.status as PtyStatus)
            : "error";
          onStatusChangeRef.current({ status, errorMessage: evt.errorMessage });
        }
      }
      pendingStatusEvents.length = 0;
    }

    async function setup() {
      // Register listeners FIRST so no events are lost between spawn and listen.
      // Events arriving before the session ID is known are buffered and replayed.
      const p1 = listen<PtyDataEvent>("pty:data", (event) => {
        handleDataEvent(event.payload);
      });
      listenPromises.push(p1);
      const unlisten1 = await p1;
      if (cancelled) { unlisten1(); return; }
      unlistenFns.push(unlisten1);

      const p2 = listen<PtyStatusEvent>("pty:status", (event) => {
        handleStatusEvent(event.payload);
      });
      listenPromises.push(p2);
      const unlisten2 = await p2;
      if (cancelled) { unlisten2(); unlisten1(); return; }
      unlistenFns.push(unlisten2);

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
        // Replay any events that arrived before the session ID was known
        drainPendingEvents(sid);
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
      // Unlisten already-resolved listeners
      const alreadyCalled = new Set(unlistenFns);
      unlistenFns.forEach((fn) => fn());
      // Clean up any listen promises still pending (skip already-called ones)
      listenPromises.forEach((p) => p.then((fn) => {
        if (!alreadyCalled.has(fn)) fn();
      }).catch(() => {}));
      if (sessionIdRef.current > 0) {
        invoke("pty_kill", { sessionId: sessionIdRef.current }).catch(() => {});
      }
    };
  }, [initialSize]); // eslint-disable-line react-hooks/exhaustive-deps

  const write = useCallback(async (data: string) => {
    const sid = sessionIdRef.current;
    if (sid === 0) return;
    try {
      await invoke("pty_write", { sessionId: sid, data });
    } catch {
      // PTY may be dead; status event will handle it
    }
  }, []);

  const resize = useCallback(async (cols: number, rows: number) => {
    const sid = sessionIdRef.current;
    if (sid === 0) return;
    try {
      await invoke("pty_resize", { sessionId: sid, cols, rows });
    } catch {
      // Ignore resize errors
    }
  }, []);

  return { write, resize };
}
