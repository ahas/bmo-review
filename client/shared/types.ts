// The shapes the code review screen reads from the service (docs/api.md): the projects, a commit's tree, a
// file, the commits, the searches, the reviews and their threads.

import type { DefinitionKind } from "./definitions";

// Who said one turn of a review's thread: the owner, or the agent answering.
export type TalkSpeaker = "owner" | "agent";

// A worker's checkout as the code review offers it. The absolute directory stays on the server;
// what a screen is told is whether there is anything to read, and why not where there is not.
export type CodeCheckoutState = "ready" | "missing" | "not-git" | "no-commits";

export interface CodeCommit {
  commit: string;
  short: string;
  subject: string;
  // Committer time, unix seconds.
  at: number;
  author: string;
}

export interface CodeProject {
  name: string;
  label: string;
  state: CodeCheckoutState;
  // HEAD, its branch (null on a detached HEAD) and how many tracked files differ from it, where the
  // checkout is ready.
  head: CodeCommit | null;
  branch: string | null;
  dirty: number;
}

export type CodeEntryKind = "file" | "folder" | "link" | "submodule";

export interface CodeTreeEntry {
  path: string;
  kind: CodeEntryKind;
  // Bytes, for a file.
  size: number | null;
  // The commit a submodule is pinned to.
  pinned: string | null;
}

export interface CodeTree {
  project: string;
  commit: CodeCommit;
  entries: CodeTreeEntry[];
  cut: boolean;
}

export interface CodeFile {
  project: string;
  commit: string;
  path: string;
  size: number;
  // Null for a binary file, whose body is not shown.
  text: string | null;
  lines: number;
  cut: boolean;
}

export interface CodeCommits {
  project: string;
  path: string;
  head: string;
  commits: CodeCommit[];
}

export interface CodeMatch {
  path: string;
  line: number;
  text: string;
}

export interface CodeSearch {
  project: string;
  commit: string;
  matches: CodeMatch[];
  cut: boolean;
}

// One place a name is declared at the commit being read, as the screen shows it: where it stands,
// the line itself, and what that line declares.
export interface CodeDefinition {
  path: string;
  line: number;
  text: string;
  kind: DefinitionKind;
}

// The answer to one press on a name. The file the reader stood in and the name they pressed come
// back with it, so an answer that arrives after the reader has moved on is known for what it is.
// Nothing found is an empty list: a press that finds no definition does nothing at all.
export interface CodeDefinitions {
  project: string;
  commit: string;
  path: string;
  name: string;
  definitions: CodeDefinition[];
}

export type CodeReviewScope = "line" | "range" | "file" | "folder";

export interface CodeReviewTurn {
  id: number;
  speaker: TalkSpeaker;
  text: string;
  saidAt: number;
}

// A review as a list shows it: where it is pinned, how its thread opens and how long it is.
export interface CodeReviewSummary {
  id: number;
  worker: string;
  commit: string;
  scope: CodeReviewScope;
  path: string;
  startLine: number | null;
  endLine: number | null;
  createdAt: number;
  first: string;
  turns: number;
  lastAt: number;
  todoIds: number[];
}

export interface CodeReviewThread {
  review: CodeReviewSummary & { excerpt: string | null };
  // Whether the commit it is pinned to can still be read out of the worker's checkout.
  readable: boolean;
  turns: CodeReviewTurn[];
  answering: boolean;
  failure: string | null;
}
