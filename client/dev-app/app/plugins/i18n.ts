// The dev host speaks to the screen with the bundle's own pack, read from its i18n folder: vue-i18n stood up by
// hand, as a console answers a bundle's words. The layer's `common.*` words, which a console answers from its own
// pack, stand here as their keys. A browser may ask for English, Japanese or Simplified Chinese by the dev host's
// own cookie; Korean stands where none is set, the language the browser tests read.

import { createI18n } from "vue-i18n";

import en from "../../../i18n/en.json";
import ja from "../../../i18n/ja.json";
import ko from "../../../i18n/ko.json";
import zhCN from "../../../i18n/zh-CN.json";

/** The cookie a browser names the dev host's language by. */
const LOCALE_COOKIE = "bmo-dev-locale";

/** The packs by the locale code the console speaks. */
const MESSAGES = { en, ko, ja, "zh-CN": zhCN };

export default defineNuxtPlugin({
  name: "review-dev-i18n",
  enforce: "pre",
  setup(nuxtApp) {
    const asked = useCookie<string | null>(LOCALE_COOKIE).value ?? "";
    const locale = Object.hasOwn(MESSAGES, asked) ? asked : "ko";

    nuxtApp.vueApp.use(createI18n({ legacy: false, globalInjection: true, locale, missingWarn: false, fallbackWarn: false, messages: MESSAGES }));
  }
});
