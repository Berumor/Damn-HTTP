import { create } from "zustand";

import { commands, type AppInfo } from "../bindings";

interface AppState {
  /** Facts about the running app; null until loaded. */
  info: AppInfo | null;
  loadInfo: () => Promise<void>;
}

export const useApp = create<AppState>((set) => ({
  info: null,
  loadInfo: async () => {
    set({ info: await commands.appInfo() });
  },
}));
