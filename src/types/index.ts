export const APP_VERSION = "0.0.1";

export interface Settings {
  apiKeys: Record<string, string>;
}

export type AppMode = "welcome" | "onboard" | "gateway";

export type TabId = "gateway" | "chat" | "webui";

export type PtyStatus = "starting" | "running" | "stopped" | "error";

export interface PtyState {
  status: PtyStatus;
  errorMessage?: string;
}
