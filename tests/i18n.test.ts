import { expect, test } from "bun:test";
import { createInstance } from "i18next";
import { resources } from "../src/i18n";
test("Hebrew regional locales resolve offline and interpolate translated values", async () => {
  const i18n = createInstance();
  await i18n.init({
    resources,
    lng: "he-IL",
    supportedLngs: ["en", "he"],
    load: "languageOnly",
    fallbackLng: "en",
    initAsync: false,
  });
  expect(i18n.resolvedLanguage).toBe("he");
  expect(i18n.dir()).toBe("rtl");
  expect(i18n.t("key")).toBe("מפתח API פרטי");
  expect(i18n.t("soundName", { number: 2 })).toBe("סירנה 2");
  expect(i18n.t("seconds", { count: 12 })).toBe("12 שניות");
});
test("unsupported locales fall back to English with plural rules", async () => {
  const i18n = createInstance();
  await i18n.init({
    resources,
    lng: "fr",
    supportedLngs: ["en", "he"],
    fallbackLng: "en",
    initAsync: false,
  });
  expect(i18n.t("seconds", { count: 1 })).toBe("1 second");
  expect(i18n.t("seconds", { count: 2 })).toBe("2 seconds");
});
test("bundled locale keys remain in sync", () => {
  function keys(value: object, prefix = ""): string[] {
    return Object.entries(value)
      .flatMap(([key, item]) =>
        typeof item === "object" && item !== null
          ? keys(item, `${prefix}${key}.`)
          : [`${prefix}${key}`],
      )
      .sort();
  }
  expect(keys(resources.he.translation)).toEqual(
    keys(resources.en.translation),
  );
});
