import type en from "./locales/en.json";

// Makes t("...") reject keys that are not in the English catalogue.
declare module "i18next" {
  interface CustomTypeOptions {
    defaultNS: "translation";
    resources: { translation: typeof en };
  }
}
