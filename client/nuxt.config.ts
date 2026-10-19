import { dirname } from "node:path";
import { fileURLToPath } from "node:url";

import type { Nuxt } from "@nuxt/schema";

// The code review screen as a layer a console extends (README.md, "The console plugin bundle"). It extends the
// bmo-ui layer and adds the screen's page, components and domain modules, and its own Korean pack: the words
// the screen draws are this repository's, loaded by the i18n module the host brings, which merges a layer's
// locales with its own. The host brings the console's title-bar components, the item tip and the markdown text,
// the three functions the screen asks of it, and hands the screen's routes on to the service.
const here = dirname(fileURLToPath(import.meta.url));

export default defineNuxtConfig({
  extends: ["bmo-ui"],
  modules: [
    "@nuxtjs/i18n",
    // What the host owes is declared under host/ for the bundle's own typecheck alone: where the bundle is the
    // project being prepared, those declarations are read with the screen; a host extending the bundle reads
    // none of them and keeps its own components' and functions' types.
    (_options: unknown, nuxt: Nuxt) => {
      if (nuxt.options.rootDir === here) {
        nuxt.hook("prepare:types", ({ tsConfig }) => {
          tsConfig.include = [...(tsConfig.include ?? []), "../host/**/*.d.ts"];
        });
      }
    }
  ],
  // The screen's own stylesheet, named from the layer's own folder so a host extending the layer reads this one.
  css: [fileURLToPath(new URL("./app/assets/css/review.css", import.meta.url))],
  i18n: {
    locales: [{ code: "ko", language: "ko-KR", file: "ko.yaml" }],
    defaultLocale: "ko",
    strategy: "no_prefix"
  },
  compatibilityDate: "2025-09-01",
  devtools: { enabled: false }
});
