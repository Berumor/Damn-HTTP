import { useEffect } from "react";
import { useTranslation } from "react-i18next";

import styles from "./App.module.css";
import { ThemeSwitch } from "./components/ThemeSwitch";
import { useApp } from "./stores/app";

export function App() {
  const { t } = useTranslation();
  const info = useApp((state) => state.info);
  const loadInfo = useApp((state) => state.loadInfo);

  useEffect(() => {
    void loadInfo();
  }, [loadInfo]);

  return (
    <div className={styles.app}>
      <header className={styles.header}>
        <span className={styles.name}>{t("app.name")}</span>
        <span className={styles.tagline}>{t("app.tagline")}</span>
        <span className={styles.spacer} />
        <ThemeSwitch />
      </header>
      <main className={styles.main}>
        <h1 className={styles.title}>{t("welcome.title")}</h1>
        <p className={styles.body}>{t("welcome.body")}</p>
      </main>
      <footer className={styles.footer}>
        {info ? t("app.version", { version: info.version }) : "\u00a0"}
      </footer>
    </div>
  );
}
