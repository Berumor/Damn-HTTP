import { create } from "zustand";

export type ThemePreference = "system" | "light" | "dark";

const themes: readonly ThemePreference[] = ["system", "light", "dark"];
const storageKey = "damnhttp.theme";

function storedTheme(): ThemePreference {
  // Storage can be unavailable (private mode, tests); the default still works.
  try {
    const value = window.localStorage.getItem(storageKey);
    return themes.find((theme) => theme === value) ?? "system";
  } catch {
    return "system";
  }
}

interface SettingsState {
  theme: ThemePreference;
  setTheme: (theme: ThemePreference) => void;
}

/**
 * User interface settings. The theme is kept in the webview's local storage
 * for now; it moves to the app settings file with the settings commands.
 */
export const useSettings = create<SettingsState>((set) => ({
  theme: storedTheme(),
  setTheme: (theme) => {
    try {
      window.localStorage.setItem(storageKey, theme);
    } catch {
      // Not saved; the choice still applies until the app closes.
    }
    set({ theme });
  },
}));
