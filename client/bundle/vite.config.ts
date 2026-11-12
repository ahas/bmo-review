import { existsSync, readdirSync, readFileSync } from "node:fs";
import { basename, extname } from "node:path";
import { fileURLToPath } from "node:url";

import tailwind from "@tailwindcss/vite";
import vue from "@vitejs/plugin-vue";
import { consoleBundle } from "bmo-ui/bundle/console-bundle.js";
import autoImport from "unplugin-auto-import/vite";
import components from "unplugin-vue-components/vite";
import { defineConfig } from "vite";

import type { Options as AutoImportOptions } from "unplugin-auto-import/types";
import type { ComponentResolver } from "unplugin-vue-components";
import type { Plugin } from "vite";

// The served module's build (docs/api.md, 「The console face」; bmo-ui's docs/console-bundles.md, 「What a bundle is」
// and 「The layer a bundle is built on」): the screen built for the browser as one ECMAScript module with the chunks
// it loads, its stylesheet, its words and the layer it is built on, written to dist/, the folder the service reads.
// `bun run bundle` runs it.

// A path of the bundle's own project.
function own(path: string): string {
  return fileURLToPath(new URL(`../${path}`, import.meta.url));
}

// What Nuxt hands a page of the layer without an import, imported from the shared libraries by name.
const NAMED: AutoImportOptions["imports"] = ["vue", { "nuxt/app": ["useRoute", "useRouter"], "@nuxt/ui/composables/useToast": ["useToast"] }];

// A component Nuxt registers for a page of the layer, imported by the package's own path: Nuxt UI's under its
// `U` prefix, then bmo-ui's by its file's name. The screen's own are found in its components folder.
const sharedComponent: ComponentResolver = (name) => {
  const nuxtUi = name.replace(/^U(?=[A-Z])/, "");

  if (nuxtUi !== name && existsSync(own(`node_modules/@nuxt/ui/dist/runtime/components/${nuxtUi}.vue`))) {
    return { from: `@nuxt/ui/components/${nuxtUi}.vue` };
  }

  if (existsSync(own(`node_modules/bmo-ui/app/components/${name}.vue`))) {
    return { from: `bmo-ui/app/components/${name}.vue` };
  }

  return undefined;
};

// The bundle's words: each locale file of i18n/ under the locale's code, as one JSON object.
function words(): Plugin {
  return {
    name: "bmo-review-words",
    generateBundle() {
      const locales = readdirSync(own("i18n"))
        .filter((file) => extname(file) === ".json")
        .map((file) => [basename(file, ".json"), JSON.parse(readFileSync(own(`i18n/${file}`), "utf8"))]);

      this.emitFile({ type: "asset", fileName: "words.json", source: JSON.stringify(Object.fromEntries(locales)) });
    }
  };
}

export default defineConfig({
  root: own(""),
  publicDir: false,
  // Nuxt UI's stylesheet reads the theme Nuxt writes when it prepares the project.
  resolve: { alias: { "#build": own(".nuxt") } },
  // The module runs in the browser alone.
  define: { "import.meta.client": "true", "import.meta.server": "false" },
  plugins: [
    // bmo-ui's bundle build: the libraries the console shares left as the package's own path of what is imported,
    // so the console resolves each to the copy its app runs, and the layer the bundle is built on written beside
    // the module.
    consoleBundle(),
    vue(),
    tailwind(),
    autoImport({ imports: NAMED, dts: false }),
    components({ dirs: [own("app/components")], resolvers: [sharedComponent], dts: false }),
    words()
  ],
  build: {
    outDir: own("dist"),
    emptyOutDir: true,
    cssCodeSplit: false,
    lib: { entry: { review: own("bundle/entry.ts") }, formats: ["es"], cssFileName: "review" },
    rolldownOptions: {
      output: {
        entryFileNames: "[name].js",
        chunkFileNames: "chunks/[name]-[hash].js",
        assetFileNames: (asset) => (asset.names.some((name) => extname(name) === ".css") ? "review.css" : "assets/[name]-[hash][extname]")
      }
    }
  }
});
