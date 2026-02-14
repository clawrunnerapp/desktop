import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { WelcomePage } from "./components/WelcomePage.tsx";
import { TerminalView } from "./components/TerminalView.tsx";
import { StatusBar } from "./components/StatusBar.tsx";
import { SettingsPanel } from "./components/SettingsPanel.tsx";
import type { PtyState, Settings, AppMode } from "./types/index.ts";

function App() {
  const [mode, setMode] = useState<AppMode>("welcome");
  const [isConfigured, setIsConfigured] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [ptyState, setPtyState] = useState<PtyState>({ status: "starting" });
  const [settings, setSettings] = useState<Settings>({ apiKeys: {} });
  const [restartKey, setRestartKey] = useState(0);

  useEffect(() => {
    invoke<boolean>("check_openclaw_configured").then(setIsConfigured).catch(() => {});
    invoke<Settings>("load_settings_cmd").then(setSettings).catch(() => {});
  }, []);

  // Handle process exit transitions
  useEffect(() => {
    if (ptyState.status !== "stopped" && ptyState.status !== "error") return;
    if (mode !== "onboard") return;

    let cancelled = false;
    invoke<boolean>("check_openclaw_configured").then((configured) => {
      if (cancelled) return;
      if (configured) {
        setIsConfigured(true);
        setMode("gateway");
        setPtyState({ status: "starting" });
      }
    }).catch(() => {});

    return () => { cancelled = true; };
  }, [ptyState.status, mode]);

  const handleStart = useCallback(() => {
    setPtyState({ status: "starting" });
    if (isConfigured) {
      setMode("gateway");
    } else {
      setMode("onboard");
    }
  }, [isConfigured]);

  const handleRestart = useCallback(() => {
    setPtyState({ status: "starting" });
    setRestartKey((k) => k + 1);
  }, []);

  const handleBackToWelcome = useCallback(() => {
    setPtyState({ status: "starting" });
    setMode("welcome");
    invoke<boolean>("check_openclaw_configured").then(setIsConfigured).catch(() => {});
  }, []);

  const handleSettingsSave = useCallback((newSettings: Settings) => {
    setSettings(newSettings);
    setShowSettings(false);
  }, []);

  const openclawArgs =
    mode === "onboard"
      ? ["onboard", "--skip-daemon"]
      : ["gateway"];

  if (mode === "welcome") {
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
        key={`${mode}-${restartKey}`}
        onStatusChange={setPtyState}
        settings={settings}
        args={openclawArgs}
      />

      <StatusBar
        status={ptyState}
        mode={mode}
        onRestart={handleRestart}
        onBackToWelcome={handleBackToWelcome}
      />

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
