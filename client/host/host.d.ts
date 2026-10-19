// What the host owes the screen, declared for the bundle's own typecheck alone (README.md, "The console plugin
// bundle"). A host registers the five components by name and makes the three functions importable by name, as
// Nuxt makes a project's own composables; nothing here ships.

import type { DefineComponent } from "vue";

import type { Translate } from "bmo-ui/shared/wording";

// The console's markdown renderer, which builds a document out of a file written to be read.
interface HostRenderer {
  renderMarkdown: (t: Translate, text: string, kind: "document") => string;
}

// What a screen keeps of its open tabs across visits: what it keeps, read fresh each time it changes, and how a
// kept value is put back, null where nothing was kept.
interface HostKeptTabs<S> {
  read: () => S;
  restore: (kept: unknown) => void;
}

declare module "vue" {
  export interface GlobalComponents {
    // The three pieces of the console's title bar, which take no props from the screen.
    DeployMark: DefineComponent<Record<string, never>>;
    TodoOpenButton: DefineComponent<Record<string, never>>;
    UsageMeters: DefineComponent<Record<string, never>>;
    // An item's number that opens the item in the console's TODO panel when pressed.
    ItemTip: DefineComponent<{ id: number; press?: boolean }>;
    // A text drawn as the console's markdown.
    MarkdownText: DefineComponent<{ text: string }>;
  }
}

declare global {
  // Puts a text on the clipboard and answers whether it did.
  function copyText(text: string): Promise<boolean>;
  // The renderer, fetched where it has not arrived yet; null on the server and where the fetch failed.
  function loadMarkdownRenderer(): Promise<HostRenderer | null>;
  // Keeps a screen's open tabs for the account under the screen's key.
  function useKeptTabs<S>(screen: string, kept: HostKeptTabs<S>): void;
}

export {};
