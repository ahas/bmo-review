# The review service's boundary

Every route this service answers, every call it makes, its events and its settings. This document is the
contract other units match: bmo-web, whose console passes the code review screen's requests on to these routes
and extends this repository's console plugin bundle; and the switch-over, which points the console at this
service. What the code review does is `docs/code-review.md`, and the screen is `docs/screen.md`; this document
says how it is asked. A change to anything here changes this document in the same commit and follows the version
rule (「Versions」).

## Conventions

- **Every route needs `Authorization: Bearer <token>`,** the token of the settings; a missing or wrong token is
  answered 401 `unauthorized`. The console presents it on every request it passes on.
- **Bodies are JSON, both ways, with field names in camelCase.**
- **Times are Unix seconds.**
- **A refusal** is a non-2xx answer whose body carries an English sentence under `error` and a snake_case code
  under `code` (「Refusals」), and beside them the fields its row names. The sentence is for a log; the screen
  shows the code in its own pack's words, as `code.error.<code>`.
- **A project** is named by its worker's name, `project`; **a commit** by seven to sixty-four hex characters,
  resolved to the full hash (`docs/code-review.md`, 「How commits are accepted」); **a path** by names joined by
  `/` relative to the repository, the empty path being the whole repository.
- **A commit left out or empty** is the checkout's HEAD.

## Health

`GET /health` answers `pid`, `version` and `ready`: the store is open and migrated.

## Reading a checkout

| Route | Query | Answer |
|---|---|---|
| `GET /api/code/projects` | | `[projects]` |
| `GET /api/code/tree` | `project`, `commit?` | a tree |
| `GET /api/code/file` | `project`, `commit?`, `path` | a file |
| `GET /api/code/commits` | `project`, `path?` | the commits |
| `GET /api/code/search` | `project`, `commit?`, `q` | the matches |
| `GET /api/code/definition` | `project`, `commit?`, `path`, `name` | the definitions |

- **A project:** `name` (the worker's name), `label` (its label, the name where it has none), `state` (`ready`,
  `missing`, `not-git` or `no-commits`), `head` (a commit, null unless `ready`), `branch` (null where HEAD names
  none), `dirty` (the count of uncommitted changes, untracked files left out). Every worker of the catalogue
  naming a directory stands, an unreadable one with its state; two workers on one directory stand once.
- **A commit:** `commit` (the full hash), `short`, `subject`, `at`, `author`.
- **A tree:** `project`, `commit` (a commit), `entries` and `cut` (true where the entry ceiling dropped some).
  Each entry is `path`, `kind` (`file`, `folder`, `link` or `submodule`), `size` (a file's bytes, null
  otherwise) and `pinned` (a submodule's commit, null otherwise). Excluded files and folders do not stand.
- **A file:** `project`, `commit`, `path`, `size`, `text` (null for a binary file), `lines` and `cut` (true where
  the body ceiling cut it).
- **The commits:** `project`, `path`, `head` (HEAD's full hash) and `commits`, those that touched the path,
  newest first, up to the commit ceiling.
- **The matches:** `project`, `commit`, `matches` (each `path`, `line`, `text`, the text cut to the match line
  ceiling) and `cut` (true where a ceiling of count or time was reached).
- **The definitions:** `project`, `commit`, `path` (the reader's file), `name` and `definitions` (each `path`,
  `line`, `text` and `kind`, the kind of declaration), ordered and capped as `docs/code-review.md` 「Search」
  states. A name nothing declares answers an empty list.

## Reviews

| Route | Request | Answer |
|---|---|---|
| `GET /api/code/reviews` | query `project` | `[summaries]`, newest first |
| `POST /api/code/reviews` | `project`, `commit`, `scope`, `path`, `startLine?`, `endLine?`, `text` | 202 `review` (its number) |
| `GET /api/code/reviews/{id}` | | a thread |
| `GET /api/code/reviews/{id}/talk` | | `answering`, `failure` |
| `POST /api/code/reviews/{id}/talk` | `text` | 202 |
| `POST /api/code/reviews/{id}/todo` | `title`, `toDo`, `conclusion?` | `raised` (the filed item's number) |

- **A summary:** `id`, `worker`, `commit`, `scope` (`line`, `range`, `file` or `folder`), `path`, `startLine`,
  `endLine`, `createdAt`, `first` (the thread's first words), `turns` (their count), `lastAt` (the last words'
  time) and `todoIds` (the items filed from it).
- **Making a review** judges the place in the commit's own tree: a line review's lines lie inside the file (a
  `line` names one line, `startLine` alone; a `range` names a start before its end), a file review's path is a
  file and a folder review's a folder. A line review's lines are read from that commit as its excerpt, cut to the
  excerpt ceiling. The review is written, its first words become the thread's first turn, and the answer comes
  before the turn ends.
- **A thread:** `review` (a summary with its `excerpt`, null for a file or folder review), `readable` (the
  commit can still be read in the worker's checkout, false where the worker is gone), `turns` (each `id`,
  `speaker` — `owner` or `agent` —, `text` and `saidAt`), `answering` and `failure`.
- **The talk's state:** `answering` is true while a turn of that review is out; `failure` is the code its last
  turn came back with (「Refusals」, the thread's failures), null where it was answered.
- **Talking** writes the owner's words at once and answers 202; the turn goes to the receiver in the background
  (「Calls this service makes」), and its answer is written as the agent's turn when it ends. The answer or the
  failure is told by a `code-review-talk` event.
- **Filing** builds the item's parts (`docs/code-review.md`, 「The item's shape」) and hands them to bmo-job,
  assigned to the review's worker; the item's number is remembered under the review, and a `code-review` event is
  sent.

## Events

`GET /api/code/events` is a server-sent event stream: the events kept in a ring are replayed after the cursor
the `Last-Event-ID` header names, then every new one as it is sent. Each event carries its cursor as its id, its
kind as its name, and `review` (the review's number) as its JSON data.

| Event | When |
|---|---|
| `code-review` | a review was made, or an item was filed from it |
| `code-review-talk` | a turn of a review's thread was written, answered or failed |

## Refusals

| Status | Code | When |
|---|---|---|
| 400 | `bad_request` | the body is not JSON of the route's shape |
| 401 | `unauthorized` | no token, or not the one the settings name |
| 404 | `no_such_worker` | the project is no worker of the catalogue naming a directory |
| 409 | `no_path` | the worker's directory does not exist |
| 409 | `not_git` | the worker's directory is not a git repository |
| 409 | `no_commits` | the repository has no commits |
| 409 | `unreadable_checkout` | the checkout cannot be read for another reason |
| 404 | `no_such_commit` | the commit does not resolve to a commit of that repository |
| 404 | `no_such_path` | the commit list's path is not one that may be shown |
| 404 | `no_such_file` | the path is no file of that commit that may be shown |
| 404 | `no_such_folder` | a folder review's path is no folder of that commit |
| 400 | `definition_no_file` | a definition search names no reader's file |
| 400 | `definition_word` | the word is not one the definition search takes |
| 400 | `search_too_long` | the search words are empty, over `limit`, or hold a NUL or a line break; carries `limit` |
| 400 | `review_empty` | a review's first words are empty |
| 400 | `talk_empty` | a turn's words are empty |
| 400 | `too_long` | the words are over `limit`, the talk's ceiling; carries `limit` |
| 400 | `bad_review_place` | `scope` is not one of its four, or the path is not a path a review takes |
| 400 | `bad_review_line` | the lines are not whole numbers from 1, or a range's start is not before its end |
| 400 | `no_such_line` | a line lies past the file's last |
| 409 | `no_line_review` | a line review of a file with no body |
| 404 | `no_such_review` | no review has that number |
| 409 | `review_busy` | a turn of that review is out: no words and no filing until it ends |
| 400 | `title_too_long` | the title is empty or over `limit`; carries `limit` |
| 400 | `todo_too_long` | what to do is empty or over `limit`; carries `limit` |
| 400 | `conclusion_too_long` | the conclusion is over `limit`; carries `limit` |
| 409 | `no_worker` | the review's worker is gone from the catalogue, as bmo-job answers it |
| 501 | `job_filing_unsupported` | bmo-job carries no route for this service to file an item (「What the siblings lack」) |
| 502 | `job_failed` | a call on bmo-job failed for another reason |
| 500 | `database` | the store failed |

**The thread's failures** are the codes a turn comes back with, kept as the thread's `failure` rather than
answered to the talk's request:

| Code | When |
|---|---|
| `receiver_talk_unsupported` | the receiver carries no talk topic for a code review (「What the siblings lack」) |
| `resting` | the receiver refused the talk with `resting`: the agent rests or the owner's chat has fallen silent |
| `limit` | the receiver refused the talk with `limit`: the provider's limit is reached |
| `outage` | the receiver refused the talk with `outage`: the provider is out |
| `talk_failed` | the receiver refused the talk with any other code, or could not be reached |
| `no_answer` | the talk came back with no answer |

## Calls this service makes

Each unit is reached through one module named for it under `src/clients/`, holding only the calls this service
makes, with the base address and token its settings name.

### bmo-job

Stated in bmo-job's `docs/api.md`, 「Workers」:

| Route | Request | Answer |
|---|---|---|
| `GET /workers` | | `[workers]` |
| `GET /workers/{name}` | | a worker |

- "**A worker:** `name`, `label?`, `cwd?`, `icon?`, `builtin`, `preamble?`, `dev_command?`, `support_count`,
  `support_keepalive`, `support_of?`, `support_slot?`, `node?`, `role?`, `resting_since?` and `created_at`, as
  `workers` keeps it. A name no worker has is refused `no_worker`."
- This service reads `name`, `label` and `cwd`: the list for the projects, one worker for making a review and
  before filing. `no_worker` is carried through as this service's own `no_worker`.
- **Filing an item** is shaped as bmo-job's filing parts, its `docs/api.md` 「A filing part: `FilingPart`」:
  `title`, `what`, `do`, `conclusion`, `instruction` (the owner's words in the thread) and `reply` (the agent's
  answers), with the worker, the requester and the approval pending. No route of bmo-job's takes a filing from
  another unit (「What the siblings lack」), so the call answers `job_filing_unsupported` until one stands.

### bmo-receiver

Stated in the receiver's `docs/boundary-api.md`, 「For bmo-job」, which the receiver serves with snake_case bodies:

| Call | Route | Request | Answer |
|---|---|---|---|
| Talk | `POST /talks` | `topic`, `[turns]`, `said` | `answer?`: null where the agent said nothing |

- "A turn is `owner` and `text`. The talk is a round of the owner's private chat on the chat agent's session. A
  talk that comes back with nothing is refused: `resting` with 409 where the agent rests or the owner's chat has
  fallen silent, `limit` with 429 where its provider's limit is reached, `outage` with 503 where its provider is
  out, and any other code for any other failure."
- **The topic** this service sends is of kind `code-review`, holding the review's facts: `id`, `worker`,
  `label`, `root` (the checkout's absolute path), `commit`, `scope`, `path`, `start_line?`, `end_line?` and
  `excerpt?`. The receiver words the turn's head from its own policy. The receiver documents only the kinds
  `item` and `document` (「What the siblings lack」), so a talk ends at once with the failure
  `receiver_talk_unsupported` until the kind stands.
- **The turns** are the thread's earlier turns, the owner's marked `owner`; **`said`** is the words just said.

### What the siblings lack

| Unit | What is missing |
|---|---|
| bmo-receiver | a `code-review` kind of a talk's topic in `POST /talks`, with the fields above, in its `docs/boundary-api.md` and its talk route |
| bmo-job | a route by which another unit files a fresh item out of its own conversation: the filing parts, the worker, the requester, its source named, waiting for approval, placed at the end of the queue, with the queue's news its own |

## What it reaches beside the units

- **git,** run in a worker's checkout as `docs/code-review.md` 「How git is called」 states, reading only.
- **Its own store,** the SQLite file the settings name, migrated from `migrations/` at start.

## Settings

The settings are read at start in layers, each overriding the one before: the defaults, which are the
repository's own `settings.toml` built into the binary; then `settings.toml` and `settings.local.toml` where they
stand in the settings folder, the one `BMO_REVIEW_SETTINGS` names, else the folder above the one holding the
binary (a deployment's root); then `BMO_REVIEW_<TABLE>__<KEY>` in the environment. An empty token lets no caller
in.

| Table | Key | What it is |
|---|---|---|
| `server` | `listen` | where the service answers, as `host:port` |
| `server` | `token` | the bearer token every caller presents |
| `store` | `path` | the SQLite file the reviews and threads stand in |
| `job` | `address`, `token` | bmo-job's base address and the token it takes |
| `receiver` | `address`, `token` | the receiver's base address and the token it takes |
| `filing` | `owner` | the owner's name, filling `{owner}` in the item's parts |
| `filing` | `agent` | the chat agent's name, filling `{agent}` in the item's parts |
| `filing` | `requester` | who an item filed from a review is requested by |
| `filing` | `excerpt_clip` | how much of the reviewed lines an item carries, in characters |
| `limits` | `file_bytes`, `binary_probe` | the body ceiling, and how far into a file a NUL marks it binary |
| `limits` | `tree_entries`, `commits` | the tree's entry ceiling and the commit list's |
| `limits` | `search_query`, `search_per_file`, `search_total`, `search_timeout_ms` | the content search's ceilings |
| `limits` | `definition_scan_per_file`, `definition_per_file`, `definition_total`, `definition_timeout_ms` | the definition search's ceilings |
| `limits` | `match_line` | how much of a matched line comes back |
| `limits` | `excerpt` | how much of the reviewed lines a review keeps |
| `limits` | `talk`, `title`, `to_do`, `conclusion` | the ceilings of a turn's words and of a filing's three fields |

## What the console does

These are bmo-web's to carry; this repository holds none of them.

- **Pass on** every request under `/api/code/` behind the console's sign-in, whole — the method, the query, the
  body and the headers that say what is asked — to the address this service answers on, with its token, and hand
  the answer back as it came, the event stream as it streams.
- **Extend the console plugin bundle** under `client/`, packed as `bmo-review-client`, and serve its page at
  `/code`.
- **Owe the bundle what it asks of the host:** bmo-ui at the version the bundle's peer names, an i18n module that
  loads the bundle's own Korean pack beside the host's, the components `MarkdownText`, `ItemTip`, `DeployMark`,
  `TodoOpenButton` and `UsageMeters` declared in `client/host/` by the props the screen hands them, the console's
  tab-state store the screen keeps its open tabs in, and the TODO panel an item's number opens.
- **The words are the bundle's own:** the screen's keys and `code.error.<code>` for each code above stand in the
  bundle's pack, not the host's.

## Versions

The repository's version is the crate's, in semver. The major moves on a change to this document's routes,
fields, codes, events or settings that a caller must follow; the minor on an addition to any of them; the patch
on a change that leaves every statement here true.
