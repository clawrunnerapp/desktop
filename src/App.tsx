import { useState, useCallback } from "react";
import { TerminalView } from "./components/TerminalView.tsx";
import { StatusBar } from "./components/StatusBar.tsx";
import { SettingsPanel } from "./components/SettingsPanel.tsx";
import type { PtyState, Settings } from "./types/index.ts";

function App() {
  const [showSettings, setShowSettings] = useState(false);
  const [ptyState, setPtyState] = useState<PtyState>({ status: "starting" });
  const [settings, setSettings] = useState<Settings>({ apiKeys: {} });

  const handleSettingsSave = useCallback((newSettings: Settings) => {
    setSettings(newSettings);
    setShowSettings(false);
  }, []);

  return (
    <div className="app">
      <div className="app-header">
        <h1>OpenClaw Desktop</h1>
        <div className="app-header-actions">
          <button onClick={() => setShowSettings(true)}>Settings</button>
        </div>
      </div>

      <TerminalView onStatusChange={setPtyState} settings={settings} />

      <StatusBar status={ptyState} />

      {showSettings && (
        <SettingsPanel
          settings={settings}
          onSave={handleSettingsSave}
          onClose={() => setShowSettings(false)}
        />
      )}
    </div>
  );
}

export default App;
