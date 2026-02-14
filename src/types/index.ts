export interface Settings {
  apiKeys: Record<string, string>;
}

export type PtyStatus = "starting" | "running" | "stopped" | "error";

export interface PtyState {
  status: PtyStatus;
  errorMessage?: string;
}
