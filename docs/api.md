# The review service's boundary

Every route this service answers, its events, its codes, the calls it makes, its harness plugin, its console
bundle and its settings. What the code review does is `docs/code-review.md`, and the screen is
`docs/screen.md`; this document says how it is asked. The OpenAPI 3.1 document, `docs/openapi.json`, states the
same routes, bodies and tools as JSON Schemas generated from the service's own types; `bmo-review openapi` prints
it. A change to anything here changes this document and that one in the same commit and follows the version rule
(「Versions」).

## Conventions

The common wire format and the API design rules are bmo-chat's `docs/conventions.md`; this service keeps them.

- **Every route needs `Authorization: Bearer <token>`,** the token of the settings; a missing or wrong token is
  answered 401 `unauthorized`. An empty token in the settings lets no caller in.
- **Every route is answered again under `/screen`,** the screen's prefix (「The console bundle」), where a call
  carrying the token is also refused 403 `forbidden` when its `Owner-Role` header is missing or names no role
  that allows `member`: `administrator` and `member` both do. A route answers under the prefix exactly as it
  answers at its own path.
- **Bodies are JSON, both ways, with field names in camelCase.**
- **Times are integers of epoch milliseconds,** each field ending in `At`, and a commit's time is `at`.
- **A failure** is a non-2xx answer whose body is `code`, a kebab-case code a caller matches on, and `error`, an
  English sentence for a log (「Codes」).
- **A page** is `items` and `more`, whether anything stands past the last item. A list takes `limit`, 1 to 1,000
  and 100 where absent, and `after`, the cursor each route states: the next page is asked with the last item's
  cursor.
- **A project** is named by the name the graph service registers it under, `projectName`; **a revision** by
  seven to sixty-four hex characters, resolved to the full hash (`docs/code-review.md`, 「How revisions are
  accepted」), HEAD where left out or empty; **a path** by names joined by `/` relative to the repository, the
  empty path being the whole repository.

## Health

`GET /health` answers `ready`, `reason` (null while ready) and `version`. A service that answers has its store
open and migrated.

## Projects

| Route | Query | Answer |
|---|---|---|
| `GET /projects` | `after`, `limit` | a page of projects, in name order |
| `GET /projects/{projectName}/tree` | `revision`, `path`, `after`, `limit` | a page of tree entries |
| `GET /projects/{projectName}/files/{filePath}` | `revision` | a file |
| `GET /projects/{projectName}/commits` | `to`, `from`, `path`, `after`, `limit` | a page of commits, newest first |
| `GET /projects/{projectName}/commits/{commitId}` | | a commit with its diff |
| `GET /projects/{projectName}/matches` | `q`, `revision`, `after`, `limit` | a page of matches |
| `GET /projects/{projectName}/definitions` | `name`, `path`, `revision`, `after`, `limit` | a page of definitions |

- **A project:** `name`, `checkout` (the absolute path the graph names), `state` (`ready`, `missing`, `not-git` or
  `no-commits`), `head` (HEAD's full hash, null unless `ready`), `branch` (the branch HEAD names, null where it
  names none or the project is not `ready`) and `uncommittedCount` (the tracked files' uncommitted changes). The
  cursor is the last project's `name`. Every project the graph serves stands, an unreadable one with its state.
- **A tree entry:** `path`, `kind` (`file`, `folder`, `link` or `submodule`), `byteCount` (a file's bytes, null
  otherwise) and `pinnedRevision` (a submodule's revision, null otherwise). The entries are every one under
  `path`, at any depth, in path order, the excluded ones left out; the cursor is the last entry's `path`, and
  `limit` here reaches up to the tree ceiling. A `path` that is no folder of the revision is answered
  `not-found`.
- **A file:** `path`, `revision` (the full hash), `byteCount`, `text` (null for a binary file), `lineCount` and
  `cut` (true where the body ceiling cut it). `filePath` stands as the rest of the route's path, its slashes as
  they are.
- **A commit:** `id` (the full hash), `shortId`, `subject`, `authorName`, `at`, `files` (the paths it touched
  against its first parent, the excluded ones left out), `diff` and `cut`. In a list `diff` is null and `cut`
  false; read alone, `diff` is the patch over its files and `cut` true where the diff ceiling cut it. The list
  walks back from `to`, HEAD where absent, and stops short of `from` and its history where it is given; with
  `path`, only the commits that touched it stand. The cursor is the last commit's `id`. `commitId` is a revision
  as above.
- **A match:** `path`, `line` and `text`, the line cut to the match line ceiling. `q` is the words, taken
  literally and ignoring case; empty words, words past the search ceiling, or words holding a NUL or a line break
  are answered `bad-request`.
- **A definition:** `name`, `path`, `line`, `text` and `kind` (`function`, `method`, `class`, `interface`, `type`,
  `enum`, `const`, `struct`, `trait`, `module`, `macro` or `re-export`), ordered and capped as `docs/code-review.md`
  「Search」 states. `path` is the reader's file, which orders the answer and names the language whose keywords
  are no name; a `name` the search does not take is answered `not-a-name`. A name nothing declares answers an
  empty page.
- **The cursor of matches and definitions** is the count of items already read, since a search keeps nothing to
  key on; `more` is true where items stand past the page or a ceiling or the time cut the search.
- **A project the graph does not know** is answered `not-found`, and so is a revision that resolves to no commit
  of it; a checkout that cannot be read is answered `unreadable-checkout`, its state in the sentence.

## Reviews

| Route | Body or query | Answer |
|---|---|---|
| `GET /reviews` | query `projectName`, `revision`, `state`, `after`, `limit` | a page of reviews, newest first |
| `POST /reviews` | `projectName`, `revision`, `title`, `authorName` | 201, the review |
| `GET /reviews/{reviewId}` | | the review |
| `PATCH /reviews/{reviewId}` | `state` | the review |
| `GET /reviews/{reviewId}/comments` | query `after`, `limit` | a page of comments, oldest first |
| `POST /reviews/{reviewId}/comments` | `path?`, `line?`, `endLine?`, `text`, `authorName` | 201, the comment |
| `POST /reviews/{reviewId}/filings` | `title`, `brief`, `queueName` | 201, the filing |

- **A review:** `id`, `projectName`, `revision` (the full hash), `title`, `state` (`open`, `answered` or
  `closed`), `authorName`, `commentCount`, `itemIds` (the job service's items filed from it, oldest first),
  `createdAt` and `changedAt`. The list's cursor is the last review's `id`, and each filter given narrows it.
- **Opening a review** resolves the revision in the project's checkout; a project the graph does not know is
  refused `unknown-project`, a revision that resolves to no commit `unknown-revision`, and a checkout that cannot
  be read `unreadable-checkout`. The title and the author's name are refused `bad-request` where empty or past
  their ceilings.
- **Changing a review** is a change of its state, `open` or `closed`; a move its states do not allow, `answered`
  among them, is refused `conflict` (`docs/code-review.md`, 「Its states」).
- **A comment:** `id`, `reviewId`, `path` (null for the whole review), `line` and `endLine` (null where it points
  at no lines; `endLine` null for one line), `excerpt` (the lines it points at, copied from the review's
  revision, null where it points at none), `text`, `authorName` and `createdAt`. The list's cursor is the last
  comment's `id`.
- **Commenting** judges the place against the review's revision: `line` without `path`, `endLine` without `line`,
  or an `endLine` not past `line` is `bad-request`; a path that is no file or folder of the revision that may be
  shown, lines on a folder, lines past the file's last, or lines on a file with no body is `no-such-place`. The
  text and the author's name are refused `bad-request` where empty or past their ceilings. A comment moves the
  review's state as its states say, and is kept on a closed review.
- **A filing:** `id`, `reviewId`, `itemId` (the number the job service answered) and `createdAt`. Filing
  creates the item in the job service (「Calls this service makes」); the job service's refusal is answered
  `filing-refused`, its code and sentence in this service's sentence, and a job service that cannot be reached
  or fails `upstream-failed`.
- **An unknown review** is answered `not-found`.

## Events

`GET /events` is the service's one stream, as server-sent events, or a page of them to a reader asking
`Accept: application/json`.

- **An event** is `id` (the event number, which only grows), `kind`, `at` and `data`, the member as a GET answers
  it. Its SSE `id` is its number and its SSE `event` its kind.
- **Narrowing:** `kinds`, a comma-separated list of kinds, and `reviewId`, the events of one review (a review's
  own and those of its comments and filings).
- **Resuming:** `after` or the `Last-Event-ID` header, the number of the last event read. The service keeps the
  last 1,000 events in its store; a resume from before them is answered 410 `gone`, and the reader reads the
  collections again.
- **A page** takes `after` and `limit`. The stream says a comment line every fifteen seconds.

| Kind | When | `data` |
|---|---|---|
| `review.created` | a review was opened | the review |
| `review.changed` | a review's state moved, or a comment or a filing was added to it | the review |
| `comment.created` | a comment was written | the comment |
| `filing.created` | an item was filed from a review | the filing |

## Codes

| Status | Code | When |
|---|---|---|
| 400 | `bad-request` | the body, the query or the path is not the shape the route takes; the sentence names what is wrong |
| 401 | `unauthorized` | no token, or not the one the settings name |
| 403 | `forbidden` | under `/screen`, the call's `Owner-Role` is missing or does not allow `member` |
| 404 | `no-route` | no route answers the method and path |
| 404 | `not-found` | nothing stands at the project, the revision, the path, the review or the bundle's file the request names, or no bundle stands |
| 409 | `conflict` | the review's states do not allow the change |
| 410 | `gone` | the events since the number asked for are no longer kept |
| 422 | `unreadable-checkout` | the project's checkout is missing, not in a git repository or without commits |
| 422 | `unknown-project` | a review names a project the graph does not know |
| 422 | `unknown-revision` | a review names a revision that resolves to no commit of the project |
| 422 | `no-such-place` | a comment's place is no file or folder of the revision that may be shown, or its lines lie off the file |
| 422 | `not-a-name` | the definition search does not take the word |
| 422 | `filing-refused` | the job service refused the item |
| 500 | `failed` | the service failed; the sentence is the failure |
| 502 | `upstream-failed` | the graph service or the job service could not be reached or failed |

## Calls this service makes

Each unit is reached only through the client it publishes, at the base address and with the token the settings
name, and its contract is that unit's own document.

- **bmo-graph,** through `bmo_graph::client` at 0.4.0-rev.1: `GET /projects` for the projects, a page at a time
  with the caller's cursor, and `GET /projects/{projectName}` for the one a request names; this service reads a
  project's `name` and `checkout`.
- **bmo-job,** through `bmo-job-client` at 0.1.0-rev.7: `POST /items` for a filing, with the caller's `title`,
  `brief` and `queueName`, and as `labels` the review's `reviewId`, `projectName` and `revision`; this service
  reads the item's `id`.
- **git,** run in a project's checkout as `docs/code-review.md` 「How git is called」 states, reading only.
- **Its own store,** the SQLite file the settings name, migrated from `migrations/` at start.

## The client

`bmo_review::client` is the service's own client: one method for each route, sending and reading the bodies of
`bmo_review::wire`, which the routes answer in and the OpenAPI document is generated from, and a failure read
back as its status, code and sentence. It carries the contract and no caller's policy.

## The harness plugin

The plugin is `bmo-review`: the MCP server `bmo-review mcp` starts on standard input and output, and the skill
`plugin/skills/bmo-review/SKILL.md`. The server reaches the service at the address and with the token the
settings name (`http://` and `server.listen`, `server.token`), or those `--address` and `--token` give. Each
tool is a thin call of one route through the client; a tool's failure is the error shape with the status, or the
sentence alone where nothing answered.

| Tool | Route | Input | Output |
|---|---|---|---|
| `review_projects` | `GET /projects` | `after?`, `limit?` | a page of projects |
| `review_commits` | `GET /projects/{projectName}/commits` | `projectName`, `to?`, `from?`, `path?`, `after?`, `limit?` | a page of commits |
| `review_diff` | `GET /projects/{projectName}/commits/{commitId}` | `projectName`, `commitId` | the commit with its diff |
| `review_reviews` | `GET /reviews` | `projectName?`, `revision?`, `state?`, `after?`, `limit?` | a page of reviews |
| `review_create` | `POST /reviews` | `projectName`, `revision`, `title`, `authorName` | the review |
| `review_review` | `GET /reviews/{reviewId}`, `GET /reviews/{reviewId}/comments` | `reviewId`, `after?`, `limit?` | `review` and `comments`, a page of its comments |
| `review_comment` | `POST /reviews/{reviewId}/comments` | `reviewId`, `path?`, `line?`, `endLine?`, `text`, `authorName` | the comment |
| `review_file_item` | `POST /reviews/{reviewId}/filings` | `reviewId`, `title`, `brief`, `queueName` | the filing |

What a harness does first: it reads a change with `review_diff`, writes its findings with `review_create` and
`review_comment`, and files a follow-up item with `review_file_item`.

## The console bundle

`client/` is the service's console bundle, conforming to bmo-ui's console plugin contract at 0.1.0-rev.7 (bmo-ui's
`docs/console-bundles.md`). Its build, `bun run bundle` in `client/`, writes the folder the settings name
(`screen.bundle`): the browser module `review.js`, whose default export holds the screen's one route, `/code`;
the chunks and assets it loads by relative path; its stylesheet `review.css`; and its words `words.json`, each
locale file of `client/i18n/` under the locale's code. The module leaves `vue`, `vue-router`, `vue-i18n`,
`nuxt/app`, `@nuxt/ui` and `bmo-ui` unbundled, written as the package's own path of what it imports, and
carries everything else itself.

The service reads that folder when it starts. Where the folder does not stand it answers no screen:
`GET /console-manifest` is answered 404 `not-found`, and so are the words and the bundle's files.

| Route | Answer |
|---|---|
| `GET /console-manifest` | the manifest, JSON |
| `GET /words` | the bundle's words, JSON |
| `GET /bundle/{version}/{fileName}` | one file of the bundle, under its own media type |

- **The manifest** is the contract's shape, answered to the token alone:

  | Field | Value |
  |---|---|
  | `name` | `code`: its words' namespace and its components' prefix, `Code` |
  | `title` | `code.title.screen` |
  | `icon` | `i-material-symbols-rate-review-outline` |
  | `screenPath` | `/code` |
  | `serviceName` | `review` |
  | `bundle` | `/screen/bundle/{version}/review.js` |
  | `stylesheet` | `/screen/bundle/{version}/review.css` |
  | `words` | `/words` |
  | `apiPrefix` | `/screen` |
  | `role` | `member` |
  | `ownerKey` | false: no route reads the reader's owner key |
  | `events` | true: the screen reads `GET /events` |
  | `websocket` | false |
  | `pushKinds` | none |

- **The version** is a digest of every file the folder holds, by its path and its bytes, so the module's and
  the stylesheet's paths change whenever any file of the bundle does. A file is answered only under the version
  standing; under any other, or a name the folder does not hold, it is answered 404 `not-found`. What is answered
  may be kept by the reader for as long as it likes.
- **The words** are answered to the token alone. The bundle's files are answered under `/screen`, as the
  manifest names them, so the role guards them like every call the screen makes.

The screen calls only this service's routes, at the address bmo-ui's `relay` module answers for `review` and a
path under `/screen`, the event stream included, and says a failure by its code in its own pack's words, as
`code.error.<code>`.

The same screen is also packed as `bmo-review-client` (`bun pm pack` in `client/`): a Nuxt layer on top of
bmo-ui carrying the page, its components, its domain modules, its stylesheet and its words, declared by
`client/console-bundle.json`, for a console that extends the layer and builds the screen in rather than loading
the served module. The layer's screen calls the same routes at their own paths, without the prefix, since such a
console sets no `Owner-Role`.

## Settings

The settings are read at start in layers, each overriding the one before: the defaults, which are the
repository's own `settings.toml` built into the binary; then `settings.toml` and `settings.local.toml` where they
stand in the settings folder, the one `BMO_REVIEW_SETTINGS` names, else the folder above the one holding the
binary (a deployment's root); then `BMO_REVIEW_<TABLE>__<KEY>` in the environment.

| Table | Key | What it is |
|---|---|---|
| `server` | `listen` | where the service answers, as `host:port` |
| `server` | `token` | the bearer token every caller presents |
| `store` | `path` | the SQLite file the reviews, comments, filings and events stand in |
| `graph` | `address`, `token` | bmo-graph's base address and the token it takes |
| `job` | `address`, `token` | bmo-job's base address and the token it takes |
| `screen` | `bundle` | the folder the console bundle's build writes, read from where the process starts where it is relative |
| `limits` | `file_bytes`, `binary_probe` | the body ceiling, and how far into a file a NUL marks it binary |
| `limits` | `tree_entries`, `diff_bytes` | the most entries a tree page holds, and the diff ceiling |
| `limits` | `search_query`, `search_per_file`, `search_total`, `search_timeout_ms` | the content search's ceilings |
| `limits` | `definition_scan_per_file`, `definition_per_file`, `definition_total`, `definition_timeout_ms` | the definition search's ceilings |
| `limits` | `match_line` | how much of a matched line comes back |
| `limits` | `excerpt` | how much of the lines a comment points at it keeps |
| `limits` | `title`, `comment`, `author_name` | the ceilings of a review's title, a comment's text and an author's name |

## Versions

The repository has one version, the crate's, which its health, its OpenAPI document and its MCP server answer.
Before its first release it is `0.1.0-rev.<n>`, `n` counting the version bumps since `0.1.0`, as bmo-chat's
`docs/conventions.md` 「Versions」 states; every change to this document's routes, fields, codes, events, tools or
settings is one bump.
