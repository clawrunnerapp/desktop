import type { TabId } from "../types/index.ts";

interface TabBarProps {
  activeTab: TabId;
  onTabChange: (tab: TabId) => void;
  gatewayRunning: boolean;
}

const TABS: { id: TabId; label: string; requiresGateway: boolean }[] = [
  { id: "gateway", label: "Gateway", requiresGateway: false },
  { id: "chat", label: "Chat", requiresGateway: true },
  { id: "webui", label: "Web UI", requiresGateway: true },
];

export function TabBar({ activeTab, onTabChange, gatewayRunning }: TabBarProps) {
  return (
    <div className="tab-bar">
      {TABS.map((tab) => {
        const disabled = tab.requiresGateway && !gatewayRunning;
        return (
          <button
            key={tab.id}
            className={`tab-btn${activeTab === tab.id ? " active" : ""}${disabled ? " disabled" : ""}`}
            disabled={disabled}
            onClick={() => onTabChange(tab.id)}
          >
            {tab.label}
          </button>
        );
      })}
    </div>
  );
}
