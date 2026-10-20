import type { Translate } from "bmo-ui/shared/wording";

// What the screen says of a call the service refused (docs/api.md, 「Codes」): the service answers a kebab-case
// code and an English sentence, and the screen says the code in its own pack's words under `code.error.<code>`.
// A call that failed with no code of the service's says the caller's own words.
export function refusalText(t: Translate, error: unknown, fallback: string): string {
  const code = (error as { data?: { code?: unknown } } | null)?.data?.code;

  return typeof code === "string" && code !== "" ? t(`code.error.${code}`) : fallback;
}
