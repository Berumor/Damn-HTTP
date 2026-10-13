import { useSettings, type ThemePreference } from "../stores/settings";

export type Theme = "light" | "dark";

const darkQuery = "(prefers-color-scheme: dark)";

/** The theme a preference results in on this machine. */
export function resolveTheme(preference: ThemePreference): Theme {
  if (preference !== "system") {
    return preference;
  }
  return window.matchMedia(darkQuery).matches ? "dark" : "light";
}

function apply(preference: ThemePreference): void {
  document.documentElement.dataset.theme = resolveTheme(preference);
}

/**
 * Keeps <html data-theme> in step with the user's preference and, while the
 * preference is "system", with the operating system. Returns a function
 * that stops watching.
 */
export function watchSystemTheme(): () => void {
  apply(useSettings.getState().theme);
  const unsubscribe = useSettings.subscribe((state) => {
    apply(state.theme);
  });
  const media = window.matchMedia(darkQuery);
  const onSystemChange = () => {
    apply(useSettings.getState().theme);
  };
  media.addEventListener("change", onSystemChange);
  return () => {
    unsubscribe();
    media.removeEventListener("change", onSystemChange);
  };
}
