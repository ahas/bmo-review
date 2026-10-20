// The shapes the code review screen reads from the review service, each taken from the types generated out of
// the service's OpenAPI document (`shared/openapi.ts`, made by `bun run api` from `docs/openapi.json`), so the
// screen and the service never part.

import type { components } from "./openapi";

type Schemas = components["schemas"];

export type Page<T> = { items: T[]; more: boolean };

export type Project = Schemas["Project"];
export type ProjectState = Schemas["ProjectState"];
export type TreeEntry = Schemas["TreeEntry"];
export type EntryKind = Schemas["EntryKind"];
export type FileSource = Schemas["FileSource"];
export type Commit = Schemas["Commit"];
export type Match = Schemas["Match"];
export type Definition = Schemas["Definition"];
export type DefinitionKind = Schemas["DefinitionKind"];
export type Review = Schemas["Review"];
export type ReviewState = Schemas["ReviewState"];
export type Comment = Schemas["Comment"];
export type NewComment = Schemas["NewComment"];
export type Filing = Schemas["Filing"];
export type ServiceEvent = Schemas["Event"];

// One revision's tree as the screen holds it: every entry the service listed for it, read a page at a time, and
// whether the screen stopped short of the last page.
export interface RevisionTree {
  projectName: string;
  revision: string;
  entries: TreeEntry[];
  cut: boolean;
}

// A comment beside the review it was written into, as the code shows it among the lines.
export interface PlacedComment {
  comment: Comment;
  review: Review;
}
