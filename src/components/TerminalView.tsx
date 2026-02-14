import { useTerminal } from "../hooks/useTerminal.ts";
import { usePtySession } from "../hooks/usePtySession.ts";
import type { PtyState, Settings } from "../types/index.ts";

interface TerminalViewProps {
  onStatusChange: (state: PtyState) => void;
  settings: Settings;
}

export function TerminalView({ onStatusChange, settings }: TerminalViewProps) {
  const { containerRef, writeToTerminal } = useTerminal({
    onData: handleUserInput,
    onResize: handleResize,
  });

  const { write, resize } = usePtySession({
    onData: writeToTerminal,
    onStatusChange,
    settings,
  });

  function handleUserInput(data: string) {
    write(data);
  }

  function handleResize(cols: number, rows: number) {
    resize(cols, rows);
  }

  return <div ref={containerRef} className="terminal-container" />;
}
