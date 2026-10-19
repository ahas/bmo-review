// The names the screen's words are filled with (docs/api.md, 「Names」): the owner's and the agent's, read once
// from the service's settings for the whole page, so the pack carries only the marks `{owner}` and `{agent}`.

import type { WordingParams } from "bmo-ui/shared/wording";

interface Names {
  owner: string;
  agent: string;
}

const names = ref<Names>({ owner: "", agent: "" });
let asked = false;

// The names as the pack's marks take them, read from the service the first time a part asks. A read that failed
// is asked again by the next part.
export function useNames(): Readonly<Ref<WordingParams>> {
  if (import.meta.client && !asked) {
    asked = true;
    $fetch<Names>("/api/code/names").then(
      (read) => {
        names.value = read;
      },
      () => {
        asked = false;
      }
    );
  }

  return computed(() => ({ owner: names.value.owner, agent: names.value.agent }));
}

// The names as they stand now, for words made outside a part.
export function currentNames(): WordingParams {
  return { owner: names.value.owner, agent: names.value.agent };
}
