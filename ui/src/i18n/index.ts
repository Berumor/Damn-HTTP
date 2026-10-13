import i18n from "i18next";
import { initReactI18next } from "react-i18next";

import en from "./locales/en.json";

/**
 * Every string the user sees comes from a catalogue in ./locales. English is
 * the only language for now; adding one means adding a file and listing it
 * in `resources`.
 */
void i18n.use(initReactI18next).init({
  resources: { en: { translation: en } },
  lng: "en",
  fallbackLng: "en",
  interpolation: {
    // React already escapes what it renders.
    escapeValue: false,
  },
});

export default i18n;
