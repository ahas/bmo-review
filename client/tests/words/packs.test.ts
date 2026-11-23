// The bundle's language pack (docs/screen.md, its opening): one file to a locale, each holding the
// same keys under the bundle's namespace and the same placeholders in each word, so no locale a console speaks
// shows a key raw or drops a value the screen hands it.

import { describe, expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { basename, join } from "node:path";
import { fileURLToPath } from "node:url";

const FOLDER = fileURLToPath(new URL("../../i18n", import.meta.url));
const NAMESPACE = "code";

/** Each word of a pack by its dotted key, in the order the pack holds them. */
function wordsOf(tree: Record<string, unknown>, prefix = ""): [string, string][] {
  return Object.entries(tree).flatMap(([key, value]): [string, string][] =>
    typeof value === "string" ? [[`${prefix}${key}`, value]] : wordsOf(value as Record<string, unknown>, `${prefix}${key}.`)
  );
}

/** The placeholders a word names, sorted. */
function placeholders(word: string): string[] {
  return [...word.matchAll(/\{(\w+)\}/g)].map((found) => found[1]!).sort();
}

const files = readdirSync(FOLDER).filter((file) => file.endsWith(".json")).sort();
const packs = Object.fromEntries(
  files.map((file) => [basename(file, ".json"), wordsOf(JSON.parse(readFileSync(join(FOLDER, file), "utf8")))])
);

describe("the language pack", () => {
  test("stands in English, Japanese, Korean and Simplified Chinese", () => {
    expect(Object.keys(packs)).toEqual(["en", "ja", "ko", "zh-CN"]);
  });

  test("holds every key under the bundle's namespace", () => {
    for (const words of Object.values(packs)) expect(words.filter(([key]) => !key.startsWith(`${NAMESPACE}.`))).toEqual([]);
  });

  test("holds the English keys in every locale, in the same order", () => {
    const keys = packs.en!.map(([key]) => key);

    for (const words of Object.values(packs)) expect(words.map(([key]) => key)).toEqual(keys);
  });

  test("names the English word's placeholders in every locale's word", () => {
    const english = new Map(packs.en);

    for (const words of Object.values(packs)) {
      for (const [key, word] of words) expect([key, placeholders(word)]).toEqual([key, placeholders(english.get(key)!)]);
    }
  });
});
