import { useTerminal } from "../hooks/useTerminal.ts";
import { usePtySession } from "../hooks/usePtySession.ts";
import type { PtyState, Settings } from "../types/index.ts";

interface TerminalViewProps {
  onStatusChange: (state: PtyState) => void;
  settings: Settings;
  args: string[];
  active: boolean;
}

export function TerminalView({ onStatusChange, settings, args, active }: TerminalViewProps) {
  const { containerRef, writeToTerminal, initialSize } = useTerminal({
    onData: handleUserInput,
    onResize: handleResize,
    active,
  });

  const { write, resize } = usePtySession({
    onData: writeToTerminal,
    onStatusChange,
    settings,
    args,
    initialSize,
  });

  function handleUserInput(data: string) {
    write(data);
  }

  function handleResize(cols: number, rows: number) {
    resize(cols, rows);
  }

  return <div ref={containerRef} className="terminal-container" />;
}
