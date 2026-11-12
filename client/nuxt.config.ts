import { fileURLToPath } from "node:url";

// The code review screen's sources as a Nuxt project on bmo-ui, which the typecheck reads; the module the service
// serves is built from the same sources by `bundle/vite.config.ts` (README.md, "The console bundle").
export default defineNuxtConfig({
  extends: ["bmo-ui"],
  // The screen's own stylesheet, the one the served module's build carries.
  css: [fileURLToPath(new URL("./app/assets/css/review.css", import.meta.url))],
  compatibilityDate: "2025-09-01",
  devtools: { enabled: false }
});
