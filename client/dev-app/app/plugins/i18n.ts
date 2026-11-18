// The dev host speaks to the screen with the bundle's own pack, read from its i18n folder: vue-i18n stood up by
// hand, as a console answers a bundle's words. The layer's `common.*` words, which a console answers from its own
// pack, stand here as their keys.

import { createI18n } from "vue-i18n";

import ko from "../../../i18n/ko.json";

export default defineNuxtPlugin({
  name: "review-dev-i18n",
  enforce: "pre",
  setup(nuxtApp) {
    nuxtApp.vueApp.use(createI18n({ legacy: false, globalInjection: true, locale: "ko", missingWarn: false, fallbackWarn: false, messages: { ko } }));
  }
});
