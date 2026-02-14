import { useState, useCallback, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { WelcomePage } from "./components/WelcomePage.tsx";
import { TerminalView } from "./components/TerminalView.tsx";
import { StatusBar } from "./components/StatusBar.tsx";
import { SettingsPanel } from "./components/SettingsPanel.tsx";
import { TabBar } from "./components/TabBar.tsx";
import { WebUIView } from "./components/WebUIView.tsx";
import type { PtyState, Settings, AppMode, TabId } from "./types/index.ts";

const CHAT_ARGS = ["tui"];

function resetChatState(
  setChatSpawned: (v: boolean) => void,
  setChatPtyState: (v: PtyState) => void,
  setActiveTab: (v: TabId) => void,
) {
  setChatSpawned(false);
  setChatPtyState({ status: "starting" });
  setActiveTab("gateway");
}

function App() {
  const [mode, setMode] = useState<AppMode>("welcome");
  const [isConfigured, setIsConfigured] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [gatewayPtyState, setGatewayPtyState] = useState<PtyState>({ status: "starting" });
  const [chatPtyState, setChatPtyState] = useState<PtyState>({ status: "starting" });
  const [settings, setSettings] = useState<Settings>({ apiKeys: {} });
  const [restartKey, setRestartKey] = useState(0);
  const [activeTab, setActiveTab] = useState<TabId>("gateway");
  const [chatSpawned, setChatSpawned] = useState(false);

  useEffect(() => {
    invoke<boolean>("check_openclaw_configured").then(setIsConfigured).catch(() => {});
    invoke<Settings>("load_settings_cmd").then(setSettings).catch(() => {});
  }, []);

  // Handle process exit transitions (onboard -> gateway)
  useEffect(() => {
    if (gatewayPtyState.status !== "stopped" && gatewayPtyState.status !== "error") return;
    if (mode !== "onboard") return;

    let cancelled = false;
    invoke<boolean>("check_openclaw_configured").then((configured) => {
      if (cancelled) return;
      if (configured) {
        setIsConfigured(true);
        setMode("gateway");
        setGatewayPtyState({ status: "starting" });
        resetChatState(setChatSpawned, setChatPtyState, setActiveTab);
        setRestartKey((k) => k + 1);
      }
    }).catch(() => {});

    return () => { cancelled = true; };
  }, [gatewayPtyState.status, mode]);

  const gatewayRunning = gatewayPtyState.status === "running";

  // Switch away from webui tab when gateway stops (iframe unmounts, panel would be blank)
  useEffect(() => {
    if (!gatewayRunning && activeTab === "webui") {
      setActiveTab("gateway");
    }
  }, [gatewayRunning, activeTab]);

  const handleStart = useCallback(async () => {
    setGatewayPtyState({ status: "starting" });
    // Re-check on click in case OpenClaw was configured externally
    let configured = isConfigured;
    try {
      configured = await invoke<boolean>("check_openclaw_configured");
      setIsConfigured(configured);
    } catch {
      // Fall back to cached value
    }
    if (configured) {
      setMode("gateway");
    } else {
      setMode("onboard");
    }
  }, [isConfigured]);

  const handleRestart = useCallback(() => {
    setGatewayPtyState({ status: "starting" });
    resetChatState(setChatSpawned, setChatPtyState, setActiveTab);
    setRestartKey((k) => k + 1);
  }, []);

  const handleBackToWelcome = useCallback(() => {
    setGatewayPtyState({ status: "starting" });
    resetChatState(setChatSpawned, setChatPtyState, setActiveTab);
    setMode("welcome");
    invoke<boolean>("check_openclaw_configured").then(setIsConfigured).catch(() => {});
  }, []);

  const handleSettingsSave = useCallback((newSettings: Settings) => {
    setSettings(newSettings);
    setShowSettings(false);
  }, []);

  const handleTabChange = useCallback((tab: TabId) => {
    setActiveTab(tab);
    setChatSpawned((prev) => prev || tab === "chat");
  }, []);

  const gatewayArgs = useMemo(
    () => mode === "onboard" ? ["onboard", "--skip-daemon"] : ["gateway"],
    [mode],
  );

  // Determine the active PtyState to show in status bar
  const activePtyState = mode === "onboard"
    ? gatewayPtyState
    : activeTab === "chat"
      ? chatPtyState
      : gatewayPtyState;

  if (mode === "welcome") {
    return <WelcomePage isConfigured={isConfigured} onStart={handleStart} />;
  }

  const showTabs = mode === "gateway";

  return (
    <div className="app">
      <div className="app-header">
        <h1>OpenClaw Desktop</h1>
        <div className="app-header-actions">
          <button onClick={() => setShowSettings(true)}>Settings</button>
        </div>
      </div>

      {showTabs && (
        <TabBar
          activeTab={activeTab}
          onTabChange={handleTabChange}
          gatewayRunning={gatewayRunning}
        />
      )}

      <div className="tab-panels">
        {/* Gateway terminal - always mounted when not on welcome */}
        <div className={`tab-panel${(!showTabs || activeTab === "gateway") ? " active" : ""}`}>
          <TerminalView
            key={`gateway-${restartKey}`}
            onStatusChange={setGatewayPtyState}
            settings={settings}
            args={gatewayArgs}
            active={!showTabs || activeTab === "gateway"}
          />
        </div>

        {/* Chat terminal - lazy mounted, only in gateway mode */}
        {showTabs && chatSpawned && (
          <div className={`tab-panel${activeTab === "chat" ? " active" : ""}`}>
            <TerminalView
              key={`chat-${restartKey}`}
              onStatusChange={setChatPtyState}
              settings={settings}
              args={CHAT_ARGS}
              active={activeTab === "chat"}
            />
          </div>
        )}

        {/* Web UI iframe - only mounted when gateway is running */}
        {showTabs && gatewayRunning && (
          <div className={`tab-panel${activeTab === "webui" ? " active" : ""}`}>
            <WebUIView />
          </div>
        )}
      </div>

      <StatusBar
        status={activePtyState}
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
