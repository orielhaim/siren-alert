import i18n from "i18next";
import LanguageDetector from "i18next-browser-languagedetector";
import { initReactI18next } from "react-i18next";
import en from "./locales/en/translation.json";
import he from "./locales/he/translation.json";
export const resources = { en: { translation: en }, he: { translation: he } };
export type Language = keyof typeof resources;
void i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources,
    supportedLngs: ["en", "he"],
    fallbackLng: "en",
    load: "languageOnly",
    initAsync: false,
    returnNull: false,
    interpolation: { escapeValue: false },
    react: { useSuspense: false },
    detection: {
      order: ["localStorage", "navigator"],
      lookupLocalStorage: "siren-alert.language",
      caches: ["localStorage"],
    },
  });
export default i18n;
