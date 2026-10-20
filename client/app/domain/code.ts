import { fileIcon, folderIcon } from "bmo-ui/app/domain/file-icons";
import { openInTabs } from "bmo-ui/app/domain/tabs";

import type { TreeItem } from "@nuxt/ui";

import type { Comment, Commit, DefinitionKind, PlacedComment, Review, ReviewState, TreeEntry } from "../../shared/types";
import type { Translate } from "bmo-ui/shared/wording";

// What the code review screen works out for itself (docs/screen.md; docs/code-review.md): the explorer's tree
// out of one revision's entries, the name search over it, which comments stand where, how the reviews are
// grouped, and the words a comment's place and a review's state are called by.

export interface CodeTreeItem extends TreeItem {
  value: string;
  label: string;
  entry: TreeEntry;
  children?: CodeTreeItem[];
  // How many comments at the revision shown stand on this entry itself.
  comments: number;
}

// The repository's own folder, which the tree stands everything under so a comment on the whole repository has
// a row to be made from.
export const ROOT_VALUE = "folder:";

export function entryValue(entry: Pick<TreeEntry, "kind" | "path">): string {
  return `${entry.kind === "folder" ? "folder" : "file"}:${entry.path}`;
}

function baseName(path: string): string {
  return path.slice(path.lastIndexOf("/") + 1);
}

function parentOf(path: string): string {
  const at = path.lastIndexOf("/");

  return at === -1 ? "" : path.slice(0, at);
}

const ICONS: Record<TreeEntry["kind"], string> = {
  folder: "i-material-symbols-folder-outline",
  file: "i-material-symbols-code-blocks-outline",
  link: "i-material-symbols-link",
  submodule: "i-material-symbols-package-2-outline"
};

// A file and a folder wear the marks a reader of code knows them by; a link and a submodule keep the console's
// own.
function entryIcon(entry: TreeEntry): string {
  return entry.kind === "file" ? fileIcon(entry.path) : entry.kind === "folder" ? folderIcon(entry.path, false) : ICONS[entry.kind];
}

// Folders first, then by name, as a file explorer stands them.
function byKindAndName(one: CodeTreeItem, other: CodeTreeItem): number {
  const folder = (item: CodeTreeItem): number => (item.entry.kind === "folder" || item.entry.kind === "submodule" ? 0 : 1);

  return folder(one) - folder(other) || one.label.localeCompare(other.label);
}

// One revision's entries as a tree under the repository's own row. Given the paths a name search kept, only
// those and the folders above them stand.
export function codeTree(
  entries: readonly TreeEntry[],
  counts: ReadonlyMap<string, number>,
  kept: ReadonlySet<string> | null,
  rootLabel: string
): CodeTreeItem[] {
  const root: CodeTreeItem = {
    value: ROOT_VALUE,
    label: rootLabel,
    icon: "i-material-symbols-folder-data-outline",
    entry: { path: "", kind: "folder", byteCount: null, pinnedRevision: null },
    comments: counts.get(ROOT_VALUE) ?? 0,
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

  // The entries stand in path order, so every folder is made before what is in it.
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
      comments: counts.get(value) ?? 0,
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

// The folders above a path, root first, as the tree's values: what has to be unfolded for the path to be seen.
export function foldersAbove(path: string): string[] {
  const values = [ROOT_VALUE];
  const parts = path.split("/");

  for (let at = 1; at < parts.length; at++) {
    values.push(`folder:${parts.slice(0, at).join("/")}`);
  }

  return values;
}

// How well a path holds the typed letters in order, or null when it does not. Letters that run together count
// for more, and so do letters in the file's own name; a shorter path wins a tie.
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

// The files of a revision a name search keeps, best first, as many as a list can carry.
export function searchNames(entries: readonly TreeEntry[], query: string, limit: number): TreeEntry[] {
  return entries
    .filter((entry) => entry.kind !== "folder")
    .map((entry) => ({ entry, score: nameScore(entry.path, query) }))
    .filter((scored): scored is { entry: TreeEntry; score: number } => scored.score !== null)
    .sort((one, other) => other.score - one.score)
    .slice(0, limit)
    .map((scored) => scored.entry);
}

// What kind of place a comment points at.
export type PlaceKind = "whole" | "folder" | "file" | "line" | "range";

// A place a comment is written on, before it is written.
export interface CommentPlace {
  kind: PlaceKind;
  path: string | null;
  line: number | null;
  endLine: number | null;
}

// What kind of place a written comment stands on, as the screen can tell it: a path with no line is a file or a
// folder, which the comment itself cannot say and the tree of its revision can, so where that tree is not the one
// held the path stands as a path alone.
export type ShownKind = PlaceKind | "path";

export function placeKind(comment: Pick<Comment, "path" | "line" | "endLine">, folders: ReadonlySet<string> | null): ShownKind {
  if (comment.path === null || comment.path === undefined) {
    return "whole";
  }

  if (comment.line !== null && comment.line !== undefined) {
    return comment.endLine === null || comment.endLine === undefined ? "line" : "range";
  }

  if (comment.path === "") {
    return "folder";
  }

  return folders === null ? "path" : folders.has(comment.path) ? "folder" : "file";
}

// Where a comment stands in the tree: on the folder or the file its path names.
function commentValue(comment: Comment, folders: ReadonlySet<string>): string | null {
  const kind = placeKind(comment, folders);

  return kind === "whole" ? null : kind === "folder" ? `folder:${comment.path}` : `file:${comment.path}`;
}

// How many comments at the revision shown stand on each row of the tree.
export function commentCounts(placed: readonly PlacedComment[], folders: ReadonlySet<string>): Map<string, number> {
  const counts = new Map<string, number>();

  for (const { comment } of placed) {
    const value = commentValue(comment, folders);

    if (value !== null) {
      counts.set(value, (counts.get(value) ?? 0) + 1);
    }
  }

  return counts;
}

// The line comments of one file, in line order.
export function lineComments(placed: readonly PlacedComment[], path: string | null): PlacedComment[] {
  return placed
    .filter(({ comment }) => path !== null && comment.path === path && comment.line !== null && comment.line !== undefined)
    .sort((one, other) => (one.comment.line ?? 0) - (other.comment.line ?? 0) || one.comment.id - other.comment.id);
}

export interface ReviewGroups {
  // The reviews of the revision shown, newest first.
  here: Review[];
  // The reviews of other revisions, a group per revision, newest first.
  elsewhere: Array<{ revision: string; reviews: Review[] }>;
}

// A project's reviews, sorted onto the revision shown and the others.
export function reviewGroups(reviews: readonly Review[], revision: string | null): ReviewGroups {
  const newest = [...reviews].sort((one, other) => other.id - one.id);
  const elsewhere = new Map<string, Review[]>();

  for (const review of newest.filter((one) => one.revision !== revision)) {
    elsewhere.set(review.revision, [...(elsewhere.get(review.revision) ?? []), review]);
  }

  return {
    here: newest.filter((one) => one.revision === revision),
    elsewhere: [...elsewhere].map(([sha, list]) => ({ revision: sha, reviews: list }))
  };
}

// The review a new comment goes into where the reader names none: the one open in the review section where it is
// of the revision shown, else that revision's newest review still open, else none and a new one is opened.
export function defaultTarget(reviews: readonly Review[], revision: string | null, openId: number | null): number | null {
  const ofRevision = reviews.filter((one) => one.revision === revision);

  if (openId !== null && ofRevision.some((one) => one.id === openId)) {
    return openId;
  }

  return [...ofRevision].sort((one, other) => other.id - one.id).find((one) => one.state !== "closed")?.id ?? null;
}

// The commits the picker offers: the ones that touched what is open, and ahead of them the ones the screen must
// be able to go back to (HEAD and the revision shown) where the list does not hold them.
export function offeredCommits(touched: readonly Commit[], always: ReadonlyArray<Commit | null>): Commit[] {
  const ahead: Commit[] = [];

  for (const one of always) {
    if (one !== null && !touched.some((listed) => listed.id === one.id) && !ahead.some((listed) => listed.id === one.id)) {
      ahead.push(one);
    }
  }

  return [...ahead, ...touched];
}

export function shortSha(revision: string): string {
  return revision.slice(0, 8);
}

// What a comment's place is called on the screen.
export function placeText(t: Translate, place: { path?: string | null; line?: number | null; endLine?: number | null }, kind: ShownKind): string {
  const path = place.path === "" ? t("code.label.whole_repository") : (place.path ?? "");

  switch (kind) {
    case "whole":
      return t("code.label.place_whole");
    case "path":
      return path;
    case "line":
      return `${path}:${place.line}`;
    case "range":
      return `${path}:${place.line}–${place.endLine}`;
    case "file":
      return t("code.label.place_file", { path });
    case "folder":
      return t("code.label.place_folder", { path });
  }
}

// The words a review's state is called by.
export const STATE_LABELS: Record<ReviewState, string> = {
  open: "code.label.state_open",
  answered: "code.label.state_answered",
  closed: "code.label.state_closed"
};

export const STATE_COLORS: Record<ReviewState, "primary" | "success" | "neutral"> = {
  open: "primary",
  answered: "success",
  closed: "neutral"
};

// What a line declares, in the words a reader of this console reads. The search names the kinds in the language
// each one comes from; each kind here holds the key of its word.
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

// Where each file was left off, so that coming back to a tab comes back to the place rather than to the head of
// the file.
//
// A place belongs to the version it was read in. The same path at another revision is another file (shorter,
// longer, or nothing like it), so a place kept at one revision is no place in the other and is never given back
// for it.
export interface ReadingPlaces {
  keep: (revision: string, path: string, line: number) => void;
  taken: (revision: string, path: string) => number | null;
}

export function readingPlaces(): ReadingPlaces {
  const places = new Map<string, number>();
  const at = (revision: string, path: string): string => `${revision}:${path}`;

  return {
    keep(revision, path, line): void {
      places.set(at(revision, path), line);
    },

    taken: (revision, path) => places.get(at(revision, path)) ?? null
  };
}

// The lines a new line comment would take, out of a line pressed and, where a line was already chosen, the line
// pressed before it.
export function lineChoice(anchor: number | null, pressed: number): { start: number; end: number } {
  if (anchor === null) {
    return { start: pressed, end: pressed };
  }

  return { start: Math.min(anchor, pressed), end: Math.max(anchor, pressed) };
}

// A file's lines as the place a comment is written on.
export function linesPlace(path: string, lines: { start: number; end: number }): CommentPlace {
  return lines.start === lines.end
    ? { kind: "line", path, line: lines.start, endLine: null }
    : { kind: "range", path, line: lines.start, endLine: lines.end };
}

// What the filing dialog opens with: the review's title, and its comments in order as the brief, each under its
// writer's name and place, for the reader to rewrite.
export function filingDraft(t: Translate, review: Review, comments: readonly Comment[], folders: ReadonlySet<string> | null, limit: number): { title: string; brief: string } {
  const lines = comments.map((comment) => {
    const head = t("code.label.brief_head", { name: comment.authorName, place: placeText(t, comment, placeKind(comment, folders)) });

    return `${head}\n${comment.text}`;
  });
  const opening = t("code.label.brief_opening", { title: review.title, revision: shortSha(review.revision), project: review.projectName });

  return {
    title: review.title,
    brief: [opening, ...lines].join("\n\n").slice(0, limit)
  };
}

export interface OpenTab {
  path: string;
  revision: string;
  seenAt: number;
  pinned?: boolean;
}

// What the code review keeps of its tabs in the browser: each project's open files and the one standing, so a
// project opened again stands as it was left.
export interface KeptCodeTabs {
  projects: Record<string, { tabs: OpenTab[]; active: string | null }>;
}

// A kept value read as far as it holds; anything else is dropped.
export function readKeptCodeTabs(value: unknown, cap: number): KeptCodeTabs {
  const projects: KeptCodeTabs["projects"] = {};
  const listed = (value as { projects?: unknown } | null)?.projects;

  if (listed === null || listed === undefined || typeof listed !== "object") {
    return { projects };
  }

  for (const [name, entry] of Object.entries(listed as Record<string, unknown>)) {
    const { tabs, active } = (entry ?? {}) as { tabs?: unknown; active?: unknown };
    const read = (Array.isArray(tabs) ? tabs : []).flatMap((tab: unknown): OpenTab[] => {
      const { path, revision, seenAt, pinned } = (tab ?? {}) as Record<string, unknown>;

      return typeof path === "string" && path !== "" && typeof revision === "string" && revision !== ""
        ? [{ path, revision, seenAt: typeof seenAt === "number" ? seenAt : 0, ...(pinned === true ? { pinned: true } : {}) }]
        : [];
    });
    const unique = read.filter((tab, at) => read.findIndex((other) => other.path === tab.path) === at).slice(0, cap * 4);

    projects[name] = { tabs: unique, active: typeof active === "string" && unique.some((tab) => tab.path === active) ? active : null };
  }

  return { projects };
}

// The open files after one more is opened, at the revision it is read at.
export function openTab(tabs: readonly OpenTab[], path: string, revision: string, now: number, cap: number): OpenTab[] {
  return openInTabs(tabs, (tab) => tab.path === path, { path, revision, seenAt: now }, cap);
}
