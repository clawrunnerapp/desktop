import { useState, useEffect, useCallback, useRef } from "react";
import { enable, disable, isEnabled } from "@tauri-apps/plugin-autostart";

export function useAutostart() {
  const [enabled, setEnabled] = useState<boolean | null>(null);
  const [loading, setLoading] = useState(true);
  const [toggling, setToggling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const togglingRef = useRef(false);
  const enabledRef = useRef<boolean | null>(null);

  useEffect(() => {
    let stale = false;
    isEnabled()
      .then((value) => {
        if (stale) return;
        enabledRef.current = value;
        setEnabled(value);
        setLoading(false);
      })
      .catch(() => {
        if (stale) return;
        enabledRef.current = false;
        setEnabled(false);
        setError("Failed to check autostart status");
        setLoading(false);
      });
    return () => {
      stale = true;
    };
  }, []);

  const toggle = useCallback(async () => {
    if (togglingRef.current || enabledRef.current === null) return;
    togglingRef.current = true;
    setToggling(true);
    setError(null);

    const previous = enabledRef.current;
    try {
      if (previous) {
        await disable();
      } else {
        await enable();
      }
    } catch {
      togglingRef.current = false;
      setToggling(false);
      setError(previous ? "Failed to disable autostart" : "Failed to enable autostart");
      return;
    }

    try {
      const confirmed = await isEnabled();
      enabledRef.current = confirmed;
      setEnabled(confirmed);
    } catch {
      enabledRef.current = !previous;
      setEnabled(!previous);
    } finally {
      togglingRef.current = false;
      setToggling(false);
    }
  }, []);

  return { enabled, loading, toggling, error, toggle };
}
