import { useTranslation } from "react-i18next";

import { useSettings, type ThemePreference } from "../stores/settings";
import styles from "./ThemeSwitch.module.css";

const options: readonly ThemePreference[] = ["system", "light", "dark"];

export function ThemeSwitch() {
  const { t } = useTranslation();
  const theme = useSettings((state) => state.theme);
  const setTheme = useSettings((state) => state.setTheme);

  return (
    <div className={styles.group} role="radiogroup" aria-label={t("theme.label")}>
      {options.map((option) => (
        <button
          key={option}
          type="button"
          role="radio"
          aria-checked={theme === option}
          className={styles.option}
          onClick={() => {
            setTheme(option);
          }}
        >
          {t(`theme.${option}`)}
        </button>
      ))}
    </div>
  );
}
