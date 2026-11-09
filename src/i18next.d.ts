import "i18next";
import type { resources } from "./i18n";
declare module "i18next" {
  interface CustomTypeOptions {
    defaultNS: "translation";
    returnNull: false;
    resources: (typeof resources)["en"];
  }
}
