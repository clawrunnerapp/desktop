import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Settings } from "../types/index.ts";
import { useAutostart } from "../hooks/useAutostart.ts";

interface SettingsPanelProps {
  settings: Settings;
  onSave: (settings: Settings) => void;
  onClose: () => void;
}

const API_KEY_FIELDS = [
  { key: "OPENAI_API_KEY", label: "OpenAI API Key" },
  { key: "ANTHROPIC_API_KEY", label: "Anthropic API Key" },
  { key: "ELEVENLABS_API_KEY", label: "ElevenLabs API Key" },
  { key: "GOOGLE_MAPS_API_KEY", label: "Google Maps API Key" },
];

export function SettingsPanel({ settings, onSave, onClose }: SettingsPanelProps) {
  const [apiKeys, setApiKeys] = useState<Record<string, string>>({
    ...settings.apiKeys,
  });
  const [saveError, setSaveError] = useState<string | null>(null);
  const autostart = useAutostart();
  const panelRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    panelRef.current?.focus();
  }, []);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
        return;
      }
      if (e.key === "Tab" && panelRef.current) {
        const focusable = panelRef.current.querySelectorAll<HTMLElement>(
          'input, button, [tabindex]:not([tabindex="-1"])'
        );
        if (focusable.length === 0) return;
        const first = focusable[0];
        const last = focusable[focusable.length - 1];
        if (e.shiftKey && document.activeElement === first) {
          e.preventDefault();
          last.focus();
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  const handleChange = useCallback((key: string, value: string) => {
    setApiKeys((prev) => ({ ...prev, [key]: value }));
    setSaveError(null);
  }, []);

  const handleSave = async () => {
    const newSettings: Settings = { apiKeys };
    try {
      await invoke("save_settings", { settings: newSettings });
    } catch (err) {
      setSaveError(`Failed to save settings to disk: ${String(err)}`);
      return;
    }
    onSave(newSettings);
  };

  const autostartBusy = autostart.loading || autostart.toggling;

  return (
    <div className="settings-overlay" onClick={onClose}>
      <div
        className="settings-panel"
        ref={panelRef}
        tabIndex={-1}
        role="dialog"
        aria-modal="true"
        aria-label="Settings"
        onClick={(e) => e.stopPropagation()}
      >
        <h2 id="general-heading">General</h2>
        <div
          className={`settings-toggle-field${autostartBusy ? " settings-toggle-busy" : ""}`}
          role="group"
          aria-labelledby="general-heading"
          aria-busy={autostartBusy}
        >
          <label htmlFor="autostart-toggle">Launch at Login</label>
          <input
            id="autostart-toggle"
            type="checkbox"
            checked={autostart.enabled ?? false}
            disabled={autostartBusy}
            onChange={() => autostart.toggle()}
            aria-describedby="autostart-hint"
          />
        </div>
        <span className="settings-toggle-hint" id="autostart-hint">Applied immediately</span>
        {autostart.error && (
          <div className="settings-error settings-autostart-error" role="alert">
            {autostart.error}
          </div>
        )}
        <h2 className="settings-section-heading">API Keys</h2>
        {API_KEY_FIELDS.map(({ key, label }) => (
          <div className="settings-field" key={key}>
            <label htmlFor={`settings-${key}`}>{label}</label>
            <input
              id={`settings-${key}`}
              type="password"
              value={apiKeys[key] || ""}
              onChange={(e) => handleChange(key, e.target.value)}
              placeholder={`Enter ${label}...`}
            />
          </div>
        ))}
        {saveError && (
          <div className="settings-error" role="alert">{saveError}</div>
        )}
        <div className="settings-actions">
          <button className="btn-secondary" onClick={onClose}>
            Cancel
          </button>
          <button className="btn-primary" onClick={handleSave}>
            Save
          </button>
        </div>
      </div>
    </div>
  );
}
