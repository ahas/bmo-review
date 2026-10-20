// What this browser keeps for the screen between visits (docs/screen.md): the writer's name a review and a
// comment are signed with, the queue an item was last filed into, and each project's open tabs. A browser that
// keeps nothing forgets them, which is all there is to lose.

const AUTHOR_KEY = "bmo-code-author";
const QUEUE_KEY = "bmo-code-queue";
const TABS_KEY = "bmo-code-tabs";

function readKept(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function writeKept(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Nothing kept is nothing lost but the prefill.
  }
}

// A word kept under a key: read once the page is the browser's own, and written each time it changes.
function useKeptWord(key: string): Ref<string> {
  const word = ref("");

  onMounted(() => {
    word.value = readKept(key) ?? "";
  });

  watch(word, (value) => {
    if (value.trim() !== "") {
      writeKept(key, value.trim());
    }
  });

  return word;
}

// The name the reader signs reviews and comments with.
export function useAuthorName(): Ref<string> {
  return useKeptWord(AUTHOR_KEY);
}

// The queue the reader last filed an item into.
export function useQueueName(): Ref<string> {
  return useKeptWord(QUEUE_KEY);
}

// Each project's open tabs as last left, read as far as they hold.
export function readKeptTabs(): unknown {
  const text = readKept(TABS_KEY);

  try {
    return text === null ? null : JSON.parse(text);
  } catch {
    return null;
  }
}

export function writeKeptTabs(value: unknown): void {
  writeKept(TABS_KEY, JSON.stringify(value));
}
