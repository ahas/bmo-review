import { fileURLToPath } from "node:url";

// The code review screen as a console bundle: a layer on top of bmo-ui adding the screen's page, components,
// domain modules and stylesheet. A console extends it as it extends bmo-ui and attaches it by its declaration,
// `console-bundle.json`, which names the screen's path, the service it calls and the folder of its words
// (README.md, "The console bundle").
export default defineNuxtConfig({
  extends: ["bmo-ui"],
  // The screen's own stylesheet, named from the layer's own folder so a host extending the layer reads this one.
  css: [fileURLToPath(new URL("./app/assets/css/review.css", import.meta.url))],
  compatibilityDate: "2025-09-01",
  devtools: { enabled: false }
});
