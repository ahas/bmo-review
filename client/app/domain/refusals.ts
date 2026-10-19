import type { Translate } from "bmo-ui/shared/wording";

// What the screen says of a call the service refused (docs/api.md, 「Refusals」): the service answers an English
// sentence and a code, with a field or two beside it, and the screen says the code in its own pack's words under
// `code.error.<code>`, a ceiling's `limit` read as the pack's `n`. A call that failed with no code of the
// service's says the caller's own words.
export function refusalText(t: Translate, error: unknown, fallback: string): string {
  const body = (error as { data?: Record<string, unknown> } | null)?.data;
  const code = body?.code;

  if (typeof code !== "string" || code === "") {
    return fallback;
  }

  const limit = typeof body?.limit === "number" ? body.limit : undefined;
  const worker = typeof body?.worker === "string" ? body.worker : undefined;

  return t(`code.error.${code}`, { ...(limit === undefined ? {} : { n: limit }), ...(worker === undefined ? {} : { worker }) });
}
