import { relayAddress } from "bmo-ui/app/domain/relay";

import { CODE_VIEW } from "../../shared/config";

import type {
  Comment,
  Commit,
  Definition,
  FileSource,
  Filing,
  Match,
  NewComment,
  Page,
  Project,
  Review,
  ReviewState,
  RevisionTree,
  TreeEntry
} from "../../shared/types";

// The screen's calls to the review service, each through the console's relay at the path the service's own API
// writes (`docs/api.md`). A list the screen needs whole is read a page at a time, each page asked past the last
// item's cursor.

const SERVICE = "review";
// The most one page of a list holds.
const PAGE = 1_000;

function address(path: string): string {
  return relayAddress(SERVICE, path);
}

function segment(key: string): string {
  return encodeURIComponent(key);
}

function projectPath(projectName: string, rest: string): string {
  return `/projects/${segment(projectName)}${rest}`;
}

// A query with its empty parts left out.
type Query = Record<string, string | number | null | undefined>;

function kept(query: Query): Record<string, string | number> {
  return Object.fromEntries(Object.entries(query).filter((pair): pair is [string, string | number] => pair[1] !== null && pair[1] !== undefined && pair[1] !== ""));
}

function read<T>(path: string, query: Query = {}): Promise<T> {
  return $fetch<T>(address(path), { query: kept(query) });
}

function send<T>(method: "POST" | "PATCH", path: string, body: object): Promise<T> {
  return $fetch<T>(address(path), { method, body });
}

// Every page of a list, while it says more stand and the screen wants more, each asked past the cursor the last
// item gives.
async function whole<T>(path: string, query: Query, cursor: (item: T) => string | number, most = Number.POSITIVE_INFINITY): Promise<{ items: T[]; more: boolean }> {
  const items: T[] = [];
  let after: string | number | null = null;

  for (;;) {
    const page: Page<T> = await read<Page<T>>(path, { ...query, after, limit: Math.min(PAGE, Math.max(1, most - items.length)) });

    items.push(...page.items);

    const last = page.items.at(-1);

    if (!page.more || last === undefined || items.length >= most) {
      return { items, more: page.more };
    }

    after = cursor(last);
  }
}

export async function listProjects(): Promise<Project[]> {
  return (await whole<Project>("/projects", {}, (project) => project.name)).items;
}

// A revision's whole tree, up to the screen's ceiling, a large page at a time.
export async function readTree(projectName: string, revision: string): Promise<RevisionTree> {
  const path = projectPath(projectName, "/tree");
  const entries: TreeEntry[] = [];
  let after: string | null = null;
  let more = true;

  while (more && entries.length < CODE_VIEW.treeCap) {
    const page: Page<TreeEntry> = await read<Page<TreeEntry>>(path, { revision, after, limit: Math.min(CODE_VIEW.treePage, CODE_VIEW.treeCap - entries.length) });

    entries.push(...page.items);
    more = page.more;
    after = page.items.at(-1)?.path ?? null;

    if (after === null) {
      break;
    }
  }

  return { projectName, revision, entries, cut: more };
}

export function readFile(projectName: string, filePath: string, revision: string): Promise<FileSource> {
  return read<FileSource>(projectPath(projectName, `/files/${filePath.split("/").map(segment).join("/")}`), { revision });
}

// The commits back from a revision that touched a path, newest first, one page.
export async function listCommits(projectName: string, query: { to?: string | null; path?: string | null; limit: number }): Promise<Commit[]> {
  return (await read<Page<Commit>>(projectPath(projectName, "/commits"), query)).items;
}

export async function findMatches(projectName: string, query: { q: string; revision: string }): Promise<{ items: Match[]; more: boolean }> {
  return read<Page<Match>>(projectPath(projectName, "/matches"), { ...query, limit: PAGE });
}

export async function findDefinitions(projectName: string, query: { name: string; path: string; revision: string }): Promise<Definition[]> {
  return (await read<Page<Definition>>(projectPath(projectName, "/definitions"), { ...query, limit: PAGE })).items;
}

// A project's reviews, newest first: the newest page, which is what a screen can show.
export async function listReviews(projectName: string): Promise<Review[]> {
  return (await read<Page<Review>>("/reviews", { projectName, limit: PAGE })).items;
}

export function readReview(reviewId: number): Promise<Review> {
  return read<Review>(`/reviews/${reviewId}`);
}

export function openReview(body: { projectName: string; revision: string; title: string; authorName: string }): Promise<Review> {
  return send<Review>("POST", "/reviews", body);
}

export function changeReview(reviewId: number, state: ReviewState): Promise<Review> {
  return send<Review>("PATCH", `/reviews/${reviewId}`, { state });
}

// A review's comments, oldest first, every page.
export async function listComments(reviewId: number): Promise<Comment[]> {
  return (await whole<Comment>(`/reviews/${reviewId}/comments`, {}, (comment) => comment.id)).items;
}

export function writeComment(reviewId: number, body: NewComment): Promise<Comment> {
  return send<Comment>("POST", `/reviews/${reviewId}/comments`, body);
}

export function fileItem(reviewId: number, body: { title: string; brief: string; queueName: string }): Promise<Filing> {
  return send<Filing>("POST", `/reviews/${reviewId}/filings`, body);
}

// The address of the service's event stream.
export function eventsAddress(): string {
  return address("/events");
}

// Where a comment is written: into the review it names, or into a review of the revision opened for it first by
// the title it gives. What comes back is the review it went into and the comment.
export interface CommentDraft {
  reviewId: number | null;
  title: string;
  authorName: string;
  text: string;
}

export async function postComment(
  projectName: string,
  revision: string,
  place: { path: string | null; line: number | null; endLine: number | null },
  draft: CommentDraft
): Promise<{ reviewId: number; comment: Comment }> {
  const reviewId = draft.reviewId ?? (await openReview({ projectName, revision, title: draft.title, authorName: draft.authorName })).id;
  const comment = await writeComment(reviewId, {
    path: place.path,
    line: place.line,
    endLine: place.endLine,
    text: draft.text,
    authorName: draft.authorName
  });

  return { reviewId, comment };
}
