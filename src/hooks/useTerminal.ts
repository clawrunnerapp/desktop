import { useEffect, useRef, useCallback, useState } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import "@xterm/xterm/css/xterm.css";

interface UseTerminalOptions {
  onData: (data: string) => void;
  onResize: (cols: number, rows: number) => void;
}

export function useTerminal({ onData, onResize }: UseTerminalOptions) {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const resizeObserverRef = useRef<ResizeObserver | null>(null);
  const [initialSize, setInitialSize] = useState<{ cols: number; rows: number } | null>(null);

  // Keep callbacks in refs so the xterm onData handler always calls the latest version
  const onDataRef = useRef(onData);
  onDataRef.current = onData;
  const onResizeRef = useRef(onResize);
  onResizeRef.current = onResize;

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const term = new Terminal({
      cursorBlink: true,
      fontSize: 14,
      fontFamily: 'Menlo, Monaco, "Courier New", monospace',
      theme: {
        background: "#1a1a2e",
        foreground: "#e0e0e0",
        cursor: "#e94560",
        selectionBackground: "#0f346080",
      },
      allowProposedApi: true,
    });

    const fitAddon = new FitAddon();
    const webLinksAddon = new WebLinksAddon();

    term.loadAddon(fitAddon);
    term.loadAddon(webLinksAddon);
    term.open(container);

    termRef.current = term;
    fitAddonRef.current = fitAddon;

    // Initial fit - report dimensions for PTY spawn
    let initialRafId = requestAnimationFrame(() => {
      initialRafId = 0;
      fitAddon.fit();
      setInitialSize({ cols: term.cols, rows: term.rows });
      onResizeRef.current(term.cols, term.rows);
    });

    // Handle user input via ref to always use latest callback
    const dataDisposable = term.onData((data) => onDataRef.current(data));

    // Auto-focus the terminal
    term.focus();

    // Handle container resize with debounce
    let disposed = false;
    let resizeTimer: ReturnType<typeof setTimeout>;
    let resizeRafId = 0;
    const observer = new ResizeObserver(() => {
      clearTimeout(resizeTimer);
      resizeTimer = setTimeout(() => {
        if (disposed) return;
        resizeRafId = requestAnimationFrame(() => {
          resizeRafId = 0;
          if (disposed) return;
          fitAddon.fit();
          onResizeRef.current(term.cols, term.rows);
        });
      }, 50);
    });
    observer.observe(container);
    resizeObserverRef.current = observer;

    return () => {
      disposed = true;
      cancelAnimationFrame(initialRafId);
      cancelAnimationFrame(resizeRafId);
      clearTimeout(resizeTimer);
      dataDisposable.dispose();
      observer.disconnect();
      term.dispose();
      termRef.current = null;
      fitAddonRef.current = null;
      resizeObserverRef.current = null;
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const writeToTerminal = useCallback((data: string) => {
    termRef.current?.write(data);
  }, []);

  return { containerRef, writeToTerminal, initialSize };
}
