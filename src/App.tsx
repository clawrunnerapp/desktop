import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { WelcomePage } from "./components/WelcomePage.tsx";
import { TerminalView } from "./components/TerminalView.tsx";
import { StatusBar } from "./components/StatusBar.tsx";
import { SettingsPanel } from "./components/SettingsPanel.tsx";
import type { PtyState, Settings } from "./types/index.ts";

type AppScreen = "welcome" | "terminal";

function App() {
  const [screen, setScreen] = useState<AppScreen>("welcome");
  const [isConfigured, setIsConfigured] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [ptyState, setPtyState] = useState<PtyState>({ status: "starting" });
  const [settings, setSettings] = useState<Settings>({ apiKeys: {} });
  const [openclawArgs, setOpenclawArgs] = useState<string[]>([]);

  useEffect(() => {
    invoke<boolean>("check_openclaw_configured").then(setIsConfigured).catch(() => {});
    invoke<Settings>("load_settings_cmd").then(setSettings).catch(() => {});
  }, []);

  const handleStart = useCallback(() => {
    const args = isConfigured ? ["gateway"] : ["onboard"];
    setOpenclawArgs(args);
    setPtyState({ status: "starting" });
    setScreen("terminal");
  }, [isConfigured]);

  const handleSettingsSave = useCallback((newSettings: Settings) => {
    setSettings(newSettings);
    setShowSettings(false);
  }, []);

  if (screen === "welcome") {
    return <WelcomePage isConfigured={isConfigured} onStart={handleStart} />;
  }

  return (
    <div className="app">
      <div className="app-header">
        <h1>OpenClaw Desktop</h1>
        <div className="app-header-actions">
          <button onClick={() => setShowSettings(true)}>Settings</button>
        </div>
      </div>

      <TerminalView
        onStatusChange={setPtyState}
        settings={settings}
        args={openclawArgs}
      />

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
