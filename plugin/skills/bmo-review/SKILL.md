---
name: bmo-review
description: Review a change in a git checkout through the bmo-review MCP server: read a commit's diff, open a review of a revision, write findings as comments on its lines, files or folders, and file a follow-up item into the job service. Use when asked to review a commit or a branch of a project registered with the graph service, to read or answer an open review, or to turn a review's findings into an item of work.
---

# bmo-review

The review service reads the checkouts its settings name, as projects, always at a revision: a
commit's own git objects, never the work tree. It keeps reviews, each a titled conversation about one revision
of one project, with comments on the whole review, on a folder, a file, a line or a range of lines of that
revision, and it files follow-up items into the job service. The `bmo-review` MCP server gives a harness one
tool for each route it needs.

Who writes a review or a comment is the `authorName` the caller gives: use the same name for every review and
comment you write, so a review's state follows who answered whom.

## Starting the server

Register the server with the harness as a stdio MCP server:

```
bmo-review mcp
```

It reaches the service at the address and with the token its settings name, or those `--address` and `--token`
give. A wrong token answers every tool with a 401 `unauthorized` error.

## What to do first

1. `review_projects`: the projects, each with its `name`, its `checkout`, its `state` and its `head`. Every
   other project tool takes a `projectName`; pick the project whose `checkout` holds the code at hand. A project
   that is not `ready` cannot be read.
2. `review_commits` with `projectName`: commits newest first, back from `to` (HEAD where absent), stopping short
   of `from` where given, so `from` and `to` together list a branch's commits since it left its base. `path`
   narrows to the commits that touched it.
3. `review_diff` with `projectName` and `commitId`: one commit, the files it touched and its patch against its
   first parent. `cut` true means the diff ceiling cut the patch; read the files the patch leaves out through
   the commit's `files`.

## Writing a review

1. `review_reviews` with `projectName` and `revision` first: an open review of the same revision may already
   stand; add to it rather than opening another.
2. `review_create` with `projectName`, `revision` (the commit's `id`), a `title` naming what was reviewed, and
   your `authorName`.
3. `review_comment` once for each finding, on its exact place: `path` and `line` for one line, with `endLine`
   for a range; `path` alone for a whole file or folder; neither for the review as a whole. Lines count from 1
   in the file at the review's revision, never in the work tree.
4. `review_review` with `reviewId` reads the review and its comments, oldest first, with `after` and `limit`
   to page through a long thread. Its `state` is `open` while the last comment is the review's author's,
   `answered` once someone else had the last word, and `closed` when closed.

## Filing a follow-up

`review_file_item` with `reviewId`, a `title`, a `brief` that stands on its own (what to change, where, and
why, citing the review's comments it comes from) and the job service's `queueName`. The answer's `itemId` is the
item's number in the job service; the review keeps it in its `itemIds`.

## When a tool answers an error

A tool error's text is JSON: `status`, `code` and `error`, the service's English sentence. Match on `code`:

- `not-found`: no project, revision, path or review stands at what was named; list again.
- `unreadable-checkout`: the project's checkout is missing, not a git repository, or has no commits.
- `unknown-project`, `unknown-revision`: a review names a project the settings do not name, or a revision that is
  no commit of it.
- `no-such-place`: a comment's path is no file or folder of the review's revision, or its lines lie off the
  file or point into a folder or a file with no body.
- `conflict`: a review cannot move to that state.
- `filing-refused`: the job service refused the item, its code and sentence in the error; fix the queue or the
  fields and call again.
- `upstream-failed`: the job service could not answer.
- `bad-request`: the arguments are not the tool's shape; the sentence names what is wrong.

An error with no `code` means the service could not be reached; check the address and that the service runs.

The routes behind each tool are stated in `docs/api.md` and `docs/openapi.json`, whose `x-tools` lists every
tool's input and output; what is read, fenced and excluded is `docs/code-review.md`.
