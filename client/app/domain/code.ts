import { fileIcon, folderIcon } from "bmo-ui/app/domain/file-icons";
import { openInTabs } from "bmo-ui/app/domain/tabs";

import type { TreeItem } from "@nuxt/ui";

import type { DefinitionKind } from "../../shared/definitions";
import type { CodeReviewScope, CodeReviewSummary, CodeReviewTurn, CodeTreeEntry } from "../../shared/types";
import type { Translate } from "bmo-ui/shared/wording";

// What the code review screen works out for itself (docs/screen.md;
// docs/code-review.md): the explorer's tree out of one commit's entries, the name search over it,
// which reviews stand where, and the words a review is called by.

export interface CodeTreeItem extends TreeItem {
  value: string;
  label: string;
  entry: CodeTreeEntry;
  children?: CodeTreeItem[];
  // How many reviews of the commit shown stand on this entry itself.
  reviews: number;
}

// The repository's own folder, which the tree stands everything under so a review of the whole
// repository has a row to be made from.
export const ROOT_VALUE = "folder:";

export function entryValue(entry: CodeTreeEntry): string {
  return `${entry.kind === "folder" ? "folder" : "file"}:${entry.path}`;
}

function baseName(path: string): string {
  return path.slice(path.lastIndexOf("/") + 1);
}

function parentOf(path: string): string {
  const at = path.lastIndexOf("/");

  return at === -1 ? "" : path.slice(0, at);
}

const ICONS: Record<CodeTreeEntry["kind"], string> = {
  folder: "i-material-symbols-folder-outline",
  file: "i-material-symbols-code-blocks-outline",
  link: "i-material-symbols-link",
  submodule: "i-material-symbols-package-2-outline"
};

// A file and a folder wear the marks a reader of code knows them by; a link and a submodule keep
// the console's own.
function entryIcon(entry: CodeTreeEntry): string {
  return entry.kind === "file" ? fileIcon(entry.path) : entry.kind === "folder" ? folderIcon(entry.path, false) : ICONS[entry.kind];
}

// Folders first, then by name, as a file explorer stands them.
function byKindAndName(one: CodeTreeItem, other: CodeTreeItem): number {
  const folder = (item: CodeTreeItem): number => (item.entry.kind === "folder" || item.entry.kind === "submodule" ? 0 : 1);

  return folder(one) - folder(other) || one.label.localeCompare(other.label);
}

// One commit's entries as a tree under the repository's own row. Given the paths a name search
// kept, only those and the folders above them stand.
export function codeTree(
  entries: readonly CodeTreeEntry[],
  counts: ReadonlyMap<string, number>,
  kept: ReadonlySet<string> | null,
  rootLabel: string
): CodeTreeItem[] {
  const root: CodeTreeItem = {
    value: ROOT_VALUE,
    label: rootLabel,
    icon: "i-material-symbols-folder-data-outline",
    entry: { path: "", kind: "folder", size: null, pinned: null },
    reviews: counts.get(ROOT_VALUE) ?? 0,
    children: []
  };
  const folders = new Map<string, CodeTreeItem>([["", root]]);
  const wanted = kept === null ? null : new Set<string>();

  if (kept !== null && wanted !== null) {
    for (const path of kept) {
      for (let at = parentOf(path); at !== ""; at = parentOf(at)) {
        wanted.add(at);
      }

      wanted.add(path);
    }
  }

  // git lists a folder before what is in it, so every parent is made before its children.
  for (const entry of entries) {
    if (wanted !== null && !wanted.has(entry.path)) {
      continue;
    }

    const value = entryValue(entry);
    const item: CodeTreeItem = {
      value,
      label: baseName(entry.path),
      icon: entryIcon(entry),
      entry,
      reviews: counts.get(value) ?? 0,
      ...(entry.kind === "folder" ? { children: [] } : {})
    };
    const parent = folders.get(parentOf(entry.path)) ?? root;

    parent.children!.push(item);

    if (entry.kind === "folder") {
      folders.set(entry.path, item);
    }
  }

  for (const folder of folders.values()) {
    folder.children!.sort(byKindAndName);

    // An empty folder is a folder all the same; it simply has nothing to unfold.
    if (folder.children!.length === 0 && folder !== root) {
      delete folder.children;
    }
  }

  return [root];
}

// The folders above a path, root first, as the tree's values — what has to be unfolded for the path
// to be seen.
export function foldersAbove(path: string): string[] {
  const values = [ROOT_VALUE];
  const parts = path.split("/");

  for (let at = 1; at < parts.length; at++) {
    values.push(`folder:${parts.slice(0, at).join("/")}`);
  }

  return values;
}

// How well a path holds the typed letters in order, or null when it does not. Letters that run
// together count for more, and so do letters in the file's own name; a shorter path wins a tie.
export function nameScore(path: string, query: string): number | null {
  const wanted = query.toLowerCase().replace(/\s+/g, "");

  if (wanted === "") {
    return null;
  }

  const hay = path.toLowerCase();
  const nameStart = path.lastIndexOf("/") + 1;
  let score = 0;
  let from = 0;
  let last = -2;

  for (const letter of wanted) {
    const at = hay.indexOf(letter, from);

    if (at === -1) {
      return null;
    }

    score += 1 + (at === last + 1 ? 3 : 0) + (at >= nameStart ? 2 : 0);
    last = at;
    from = at + 1;
  }

  return score * 1000 - path.length;
}

// The files of a commit a name search keeps, best first, as many as a list can carry.
export function searchNames(entries: readonly CodeTreeEntry[], query: string, limit: number): CodeTreeEntry[] {
  return entries
    .filter((entry) => entry.kind !== "folder")
    .map((entry) => ({ entry, score: nameScore(entry.path, query) }))
    .filter((scored): scored is { entry: CodeTreeEntry; score: number } => scored.score !== null)
    .sort((one, other) => other.score - one.score)
    .slice(0, limit)
    .map((scored) => scored.entry);
}

// Where a review stands in the tree: a folder review on its folder, any other on its file.
export function reviewValue(review: Pick<CodeReviewSummary, "scope" | "path">): string {
  return review.scope === "folder" ? `folder:${review.path}` : `file:${review.path}`;
}

// How many reviews of one commit stand on each row of the tree.
export function reviewCounts(reviews: readonly CodeReviewSummary[], commit: string | null): Map<string, number> {
  const counts = new Map<string, number>();

  for (const review of reviews) {
    if (review.commit === commit) {
      const value = reviewValue(review);

      counts.set(value, (counts.get(value) ?? 0) + 1);
    }
  }

  return counts;
}

export interface ReviewGroups {
  // The reviews on the commit shown, in line order with the file's and folder's own first.
  here: CodeReviewSummary[];
  // The reviews on the same place made at other commits, a group per commit, newest first.
  elsewhere: Array<{ commit: string; reviews: CodeReviewSummary[] }>;
}

// The reviews of one file or folder, sorted onto the commit shown and the others.
export function reviewGroups(
  reviews: readonly CodeReviewSummary[],
  place: { kind: "file" | "folder"; path: string },
  commit: string | null
): ReviewGroups {
  const mine = reviews.filter((review) =>
    place.kind === "folder" ? review.scope === "folder" && review.path === place.path : review.scope !== "folder" && review.path === place.path
  );
  const here = mine
    .filter((review) => review.commit === commit)
    .sort((one, other) => (one.startLine ?? 0) - (other.startLine ?? 0) || one.id - other.id);
  const elsewhere = new Map<string, CodeReviewSummary[]>();

  for (const review of mine.filter((one) => one.commit !== commit).sort((one, other) => other.id - one.id)) {
    elsewhere.set(review.commit, [...(elsewhere.get(review.commit) ?? []), review]);
  }

  return { here, elsewhere: [...elsewhere].map(([sha, list]) => ({ commit: sha, reviews: list })) };
}

// The commits the picker offers: the ones that touched what is open, and ahead of them the ones the
// screen must be able to go back to — HEAD and the commit shown — where the list does not hold them.
export function offeredCommits<T extends { commit: string }>(touched: readonly T[], always: ReadonlyArray<T | null>): T[] {
  const ahead: T[] = [];

  for (const one of always) {
    if (one !== null && !touched.some((listed) => listed.commit === one.commit) && !ahead.some((listed) => listed.commit === one.commit)) {
      ahead.push(one);
    }
  }

  return [...ahead, ...touched];
}

export function shortSha(commit: string): string {
  return commit.slice(0, 8);
}

// What a review's place is called on the screen.
export function reviewPlace(t: Translate, review: { scope: CodeReviewScope; path: string; startLine: number | null; endLine: number | null }): string {
  const path = review.path === "" ? t("code.label.whole_repository") : review.path;

  switch (review.scope) {
    case "line":
      return `${path}:${review.startLine}`;
    case "range":
      return `${path}:${review.startLine}–${review.endLine}`;
    case "file":
      return t("code.label.place_file", { path });
    case "folder":
      return t("code.label.place_folder", { path });
  }
}

export const SCOPE_LABELS: Record<CodeReviewScope, string> = {
  line: "code.label.scope_line",
  range: "code.label.scope_range",
  file: "code.label.scope_file",
  folder: "code.label.scope_folder"
};

// What a line declares, in the words a reader of this console reads. The search names the kinds in
// the language each one comes from; each kind here holds the key of its word.
export const DEFINITION_LABELS: Record<DefinitionKind, string> = {
  function: "code.label.definition_function",
  method: "code.label.definition_method",
  class: "code.label.definition_class",
  interface: "code.label.definition_interface",
  type: "code.label.definition_type",
  enum: "code.label.definition_enum",
  const: "code.label.definition_const",
  struct: "code.label.definition_struct",
  trait: "code.label.definition_trait",
  module: "code.label.definition_module",
  macro: "code.label.definition_macro",
  "re-export": "code.label.definition_re_export"
};

// Where each file was left off, so that coming back to a tab comes back to the place rather than
// to the head of the file.
//
// A place belongs to the version it was read in. The same path at another commit is another file —
// shorter, longer, or nothing like it — so a place kept at one commit is no place in the other and
// is never given back for it: a file read down to its two hundredth line and then looked at as it
// stood when it had fifty would otherwise open at a position belonging to neither.
export interface ReadingPlaces {
  keep: (commit: string, path: string, line: number) => void;
  taken: (commit: string, path: string) => number | null;
}

export function readingPlaces(): ReadingPlaces {
  const places = new Map<string, number>();
  const at = (commit: string, path: string): string => `${commit}:${path}`;

  return {
    keep(commit, path, line): void {
      places.set(at(commit, path), line);
    },

    taken: (commit, path) => places.get(at(commit, path)) ?? null
  };
}

// The lines a new line review would take, out of a line pressed and, where a line was already
// chosen, the line pressed before it.
export function lineChoice(anchor: number | null, pressed: number): { start: number; end: number } {
  if (anchor === null) {
    return { start: pressed, end: pressed };
  }

  return { start: Math.min(anchor, pressed), end: Math.max(anchor, pressed) };
}

export function scopeOfLines(start: number, end: number): CodeReviewScope {
  return start === end ? "line" : "range";
}

// What the filing dialog opens with: a title naming the place, and the thread's last answer as what
// is to be done, for him to rewrite.
export function filingDraft(t: Translate, review: CodeReviewSummary, turns: readonly CodeReviewTurn[]): { title: string; toDo: string } {
  const answer = [...turns].reverse().find((turn) => turn.speaker === "agent");
  const path = review.path === "" ? t("code.label.whole_repository") : review.path;

  return {
    title: t("code.label.filing_title", { path }).slice(0, 200),
    toDo: (answer?.text ?? "").slice(0, 8_000)
  };
}

export interface OpenTab {
  path: string;
  commit: string;
  seenAt: number;
  pinned?: boolean;
}

// What the code review keeps of its tabs for the account: each worker's open files and the one
// standing, so a worker opened again stands as it was left.
export interface KeptCodeTabs {
  projects: Record<string, { tabs: OpenTab[]; active: string | null }>;
}

// A kept value read as far as it holds; anything else is dropped.
export function readKeptCodeTabs(value: unknown, cap: number): KeptCodeTabs {
  const projects: KeptCodeTabs["projects"] = {};
  const listed = (value as { projects?: unknown } | null)?.projects;

  if (listed === null || typeof listed !== "object") {
    return { projects };
  }

  for (const [name, entry] of Object.entries(listed as Record<string, unknown>)) {
    const { tabs, active } = (entry ?? {}) as { tabs?: unknown; active?: unknown };
    const read = (Array.isArray(tabs) ? tabs : []).flatMap((tab: unknown): OpenTab[] => {
      const { path, commit, seenAt, pinned } = (tab ?? {}) as Record<string, unknown>;

      return typeof path === "string" && path !== "" && typeof commit === "string" && commit !== ""
        ? [{ path, commit, seenAt: typeof seenAt === "number" ? seenAt : 0, ...(pinned === true ? { pinned: true } : {}) }]
        : [];
    });
    const unique = read.filter((tab, at) => read.findIndex((other) => other.path === tab.path) === at).slice(0, cap * 4);

    projects[name] = { tabs: unique, active: typeof active === "string" && unique.some((tab) => tab.path === active) ? active : null };
  }

  return { projects };
}

// The open files after one more is opened, at the commit it is read at (docs/screen.md, 「The code」
// says how every screen's tabs move).
export function openTab(tabs: readonly OpenTab[], path: string, commit: string, now: number, cap: number): OpenTab[] {
  return openInTabs(tabs, (tab) => tab.path === path, { path, commit, seenAt: now }, cap);
}
