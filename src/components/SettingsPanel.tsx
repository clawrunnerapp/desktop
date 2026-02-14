import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Settings } from "../types/index.ts";

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

  const handleChange = (key: string, value: string) => {
    setApiKeys((prev) => ({ ...prev, [key]: value }));
  };

  const handleSave = async () => {
    const newSettings: Settings = { apiKeys };
    try {
      await invoke("save_settings", { settings: newSettings });
    } catch {
      // Settings save failed - continue anyway, runtime will use in-memory
    }
    onSave(newSettings);
  };

  return (
    <div className="settings-overlay" onClick={onClose}>
      <div className="settings-panel" onClick={(e) => e.stopPropagation()}>
        <h2>API Keys</h2>
        {API_KEY_FIELDS.map(({ key, label }) => (
          <div className="settings-field" key={key}>
            <label>{label}</label>
            <input
              type="password"
              value={apiKeys[key] || ""}
              onChange={(e) => handleChange(key, e.target.value)}
              placeholder={`Enter ${label}...`}
            />
          </div>
        ))}
        <div className="settings-actions">
          <button className="btn-secondary" onClick={onClose}>
            Cancel
          </button>
          <button className="btn-primary" onClick={handleSave}>
            Save & Apply
          </button>
        </div>
      </div>
    </div>
  );
}
