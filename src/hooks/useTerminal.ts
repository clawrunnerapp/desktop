import { useEffect, useRef, useCallback } from "react";
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

    // Initial fit
    requestAnimationFrame(() => {
      fitAddon.fit();
      onResize(term.cols, term.rows);
    });

    // Handle user input
    const dataDisposable = term.onData(onData);

    // Handle container resize
    const observer = new ResizeObserver(() => {
      requestAnimationFrame(() => {
        fitAddon.fit();
        onResize(term.cols, term.rows);
      });
    });
    observer.observe(container);
    resizeObserverRef.current = observer;

    return () => {
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

  return { containerRef, writeToTerminal };
}
