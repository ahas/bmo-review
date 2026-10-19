# What this repository takes from the old bmo

Every file, crate, policy file, table, test, document and service unit this repository takes from the old single
repository, with where each came from, how many, and what becomes of it. Every set below was listed from the old
repository at its commit `3e4c032`, with `git ls-tree`, `git show` and `git grep` against that commit; the
command that lists a set stands beside it. Nothing here was copied with its history.

The tests are deferred: none is carried, written or run in this phase. They are written and run here once the
minimal services stand on the new repositories. The deferred tests are listed below so that work knows what the
old behaviour was held to.

## The server side: 18 files

`git grep -l -i -E 'code_review|code-review|codeReview|CodeReview' 3e4c032`, without `client/`, `docs/` and
the test folders, each counted with `git show 3e4c032:<file> | wc -l`.

| Old file | Lines | What of it is the code review's | Here |
|---|---|---|---|
| `bot/bmo-server/src/control/code_review.rs` | 414 | all: the four control routes, the worker lookup, the path and commit checks, the filing | ported into the reviews module (「The routes」) |
| `bot/bmo-server/src/chat/talk.rs` | 475 | the review topic, its place label, its reading line, the talk request, the item's instruction | the place label and the item's parts are ported into the reviews module; the topic's facts become the receiver talk's topic; the talk request and the reading line are the receiver's own, which it already carries |
| `bot/bmo-server/src/control/mod.rs` | 614 | the line declaring the module | not carried: this repository's own HTTP plumbing registers each route beside its answer |
| `bot/bmo-server/src/main.rs` | 3,499 | a test verb printing a review's talk request, place and instruction | not carried: a test harness, deferred with the tests |
| `archive/bmo-archive/src/migration/archive.sql` | 2,044 | the three tables and their two indexes | this repository's own SQLite migrations (「The tables」) |
| `archive/archive/archive-migrator.ts` | 4,738 | the same three tables, and the agent rename that touches `code_reviews` | not carried: the store's own migrations stand in for it; the rename is the archive's |
| `archive/archive/talk-store.ts` | 322 | `sayAboutCodeReview`, `codeReviewTalk`, `fileCodeReviewTodo`, and the types `CodeReviewScope`, `CodeReviewTalkTurn` | ported as the store's own queries in the reviews module |
| `policy/bmo-policy/policy/code-review.toml` | 50 | all (「The policy keys」) | split: settings, this repository's Korean pack, and the receiver's own copy |
| `policy/bmo-policy/src/code_review.rs` | 64 | all: the policy's typed parts | ported as the settings' and the pack's typed parts |
| `policy/bmo-policy/src/lib.rs` | 122 | the line declaring the module | not carried: the policy crate is not a dependency |
| `lang/bmo-lang/lang/ko/talk.yaml` | 28 | `code_review.answering`, `code_review.worker_gone` | the refusal codes `review_busy` and `no_worker`, worded in the bundle's pack |
| `lang/bmo-lang/src/talk.rs` | 43 | `CodeReviewWords` and its field in `TalkWords` | not carried: refusals are codes |
| `agents/agent/events.ts` | 381 | the event kinds `code-review-talk` and `code-review` and their event type | this service's own event stream (`docs/api.md`, 「Events」) |
| `markup/markup/korean-in-code-ledger.json` | 1,068 | no entry for any code review file | not carried: this repository keeps its own `korean-ledger.json` |
| `graph/bot.json` | 5,760 | the node `bot/code-review-talk` | this repository's own `.graph` (「The graph nodes」) |
| `graph/console.json` | 4,696 | the node `console/code-review` | this repository's own `.graph` |
| `graph/crates.json` | 2,178 | no code review node | not carried |
| `README.md` | 278 | the line naming `docs/code-review.md` | not carried: this repository's own `README.md` |

### The routes: 4

`git grep -n 'const PATH' 3e4c032 -- bot/bmo-server/src/control/code_review.rs`. Each answers here at the
console's own path (`docs/api.md`), and the console's own route in front of it is the same route.

| Old route | Here |
|---|---|
| `POST /code-review` | `POST /api/code/reviews` |
| `GET /code-review-talk` (the review in the query) | `GET /api/code/reviews/{id}/talk` |
| `POST /code-review-talk` | `POST /api/code/reviews/{id}/talk` |
| `POST /code-review-todo` | `POST /api/code/reviews/{id}/todo` |

### The tables: 3, with 2 indexes

`git show 3e4c032:archive/bmo-archive/src/migration/archive.sql`, the statements naming `code_review`.

| Old table | Columns | Here |
|---|---|---|
| `code_reviews` | `id`, `worker`, `root`, `commit_sha`, `scope`, `path`, `start_line`, `end_line`, `excerpt`, `created_at` (10), index `code_reviews_place` on `worker`, `path`, `id` | the same, in this repository's store |
| `code_review_talk` | `id`, `review_id`, `speaker`, `text`, `said_at` (5), index `code_review_talk_review` on `review_id`, `id` | the same; a speaker is `owner` or `agent` |
| `code_review_todos` | `review_id`, `todo_id` (2) | `review_id` and `filed_item`, the item's number as bmo-job answered it, with no reference into bmo-job's tables |

The old tables held no rows when they were listed, so no data moves.

### The policy keys: 22

`git show 3e4c032:policy/bmo-policy/policy/code-review.toml`, its leaf keys.

| Old keys | Count | Here |
|---|---|---|
| `excerpt_clip` | 1 | the setting `filing.excerpt_clip` |
| `requester` | 1 | the setting `filing.requester`, a deployment's value; the old value was a person's name |
| `places.whole`, `places.line`, `places.range`, `places.file`, `places.folder` | 5 | the Korean pack's place part, `lang/ko/place.yaml` |
| `talk.words`, `talk.head`, `talk.lines`, `talk.folder_reading`, `talk.file_reading`, `talk.excerpt`, `talk.excerpt_end` | 7 | not carried: the receiver words the talk's turn and carries its own copy, in English, under its `code-review.toml` |
| `filing.source`, `filing.his`, `filing.owner_turn`, `filing.bmo_turn`, `filing.what`, `filing.pinned`, `filing.excerpt`, `filing.conclusion` | 8 | the Korean pack's filing part, `lang/ko/filing.yaml`, with the owner's and the agent's names as the marks `{owner}` and `{agent}` the settings fill, and the two keys that named a person renamed `owner_filed` and `agent_turn` |

The old readers were the bot's control route and talk module; the receiver reads its own copy. No other unit reads
these keys.

## The console side: 32 files

The list bmo-web's `docs/inventory.md` gives for bmo-review, checked against `git grep -l -i -E
'code_review|code-review|codeReview|CodeReview|/api/code' 3e4c032 -- client/` and `git ls-tree -r --name-only
3e4c032 client/server/api/code`; each counted with `wc -l`. 5,546 lines in all.

| Old file | Lines | Here |
|---|---|---|
| `client/app/pages/code.vue` | 681 | the bundle's page |
| `client/app/components/CodeDefinitionList.vue` | 76 | the bundle |
| `client/app/components/CodeExplorer.vue` | 371 | the bundle |
| `client/app/components/CodeReviewComposer.vue` | 64 | the bundle |
| `client/app/components/CodeReviewPane.vue` | 298 | the bundle |
| `client/app/components/CodeReviewRow.vue` | 36 | the bundle |
| `client/app/components/CodeReviewSheet.vue` | 62 | the bundle |
| `client/app/components/CodeReviewTodo.vue` | 131 | the bundle |
| `client/app/components/CodeViewer.vue` | 876 | the bundle |
| `client/app/domain/code.ts` | 380 | the bundle, the file-name match with it |
| `client/shared/definitions.ts` | 308 | the bundle for the word a definition search takes; the service for the patterns, the ordering and the limits |
| `client/server/api/code/projects.get.ts` | 15 | ported to the service, `GET /api/code/projects` |
| `client/server/api/code/tree.get.ts` | 21 | ported, `GET /api/code/tree` |
| `client/server/api/code/file.get.ts` | 20 | ported, `GET /api/code/file` |
| `client/server/api/code/commits.get.ts` | 20 | ported, `GET /api/code/commits` |
| `client/server/api/code/search.get.ts` | 20 | ported, `GET /api/code/search` |
| `client/server/api/code/definition.get.ts` | 30 | ported, `GET /api/code/definition` |
| `client/server/api/code/reviews.get.ts` | 8 | ported, `GET /api/code/reviews` |
| `client/server/api/code/reviews.post.ts` | 95 | ported, `POST /api/code/reviews` |
| `client/server/api/code/reviews/[id].get.ts` | 28 | ported, `GET /api/code/reviews/{id}` |
| `client/server/api/code/reviews/[id]/talk.post.ts` | 34 | ported, `POST /api/code/reviews/{id}/talk` |
| `client/server/api/code/reviews/[id]/todo.post.ts` | 43 | ported, `POST /api/code/reviews/{id}/todo` |
| `client/server/utils/workspace.ts` | 434 | ported to the service's checkout reading |
| `client/server/utils/code-request.ts` | 63 | ported to the service's request checks |
| `client/server/utils/code-reviews.ts` | 93 | ported to the store's queries |
| `client/app/components/CodeViewer.test.ts` | 26 | deferred test |
| `client/app/domain/code.test.ts` | 254 | deferred test |
| `client/app/pages/code.test.ts` | 142 | deferred test |
| `client/browser/code-review-touch.test.ts` | 164 | deferred test |
| `client/server/api/code/reviews.post.test.ts` | 138 | deferred test |
| `client/server/utils/workspace.test.ts` | 373 | deferred test |
| `client/shared/definitions.test.ts` | 242 | deferred test |

### The console API routes: 11

`git ls-tree -r --name-only 3e4c032 client/server/api/code`, without its one test. Each is the same route here,
answered by this service rather than the console (`docs/api.md`): `GET` `projects`, `tree`, `file`, `commits`,
`search`, `definition`; `GET` and `POST` `reviews`; `GET` `reviews/{id}`; `POST` `reviews/{id}/talk`; `POST`
`reviews/{id}/todo`. The old routes answered refusals as sentences of the console's pack; here they answer codes
(`docs/api.md`, 「Refusals」), each code the old pack key's last part, `talk_empty` standing for the key that
named the agent.

### Shared console files holding parts of the code review: 3

Read with `git show 3e4c032:<file>` and the names the 32 files import from them.

| Old file | The code review's parts | Here |
|---|---|---|
| `client/shared/config.ts` | `CODE_VIEW` (its sixteen ceilings), `TALK_LIMIT`, `CODE_REVIEW_TITLE_LIMIT`, `CODE_REVIEW_TODO_LIMIT`, `CODE_REVIEW_CONCLUSION_LIMIT` | the ceilings the service enforces become its `limits` settings; those the screen needs, the open tabs' count and width among them, stand in the bundle's own config |
| `client/shared/types.ts` | `TalkSpeaker`, `CodeCheckoutState`, `CodeCommit`, `CodeProject`, `CodeEntryKind`, `CodeTreeEntry`, `CodeTree`, `CodeFile`, `CodeCommits`, `CodeMatch`, `CodeSearch`, `CodeDefinition`, `CodeDefinitions`, `CodeReviewScope`, `CodeReviewTurn`, `CodeReviewSummary`, `CodeReviewThread`, and the event kinds `code-review` and `code-review-talk` | the bundle's own types, a speaker being `owner` or `agent`, and the service's answers of the same shapes |
| `client/app/assets/css/main.css` | the rules of the review marks and the inter-line cells | the bundle's own stylesheet |

### The console modules the screen imports: 19

`git show 3e4c032:<file> | grep '^import'` over the 25 non-test files, the module each names.

| Module | Count | Here |
|---|---|---|
| `closing`, `code-copy`, `errors`, `file-icons`, `monaco`, `numbers`, `screen`, `side-parts`, `tabs`, the shared wording | 10 | bmo-ui's, imported from it; two parts are the bundle's own: the review view of the old `monaco` (its review marks, zones, line choice and code points, which bmo-ui's editor does not carry) as `domain/review-view` on bmo-ui's editor, and the refusals, which the service answers as codes, worded by `domain/refusals` in place of `errors`' reading of a sentence |
| `domain/copy` for `copyText` (100 lines), `domain/markdown-load` (36), `domain/tab-state` for `useKeptTabs` (92) | 3 | the host console's: functions the bundle declares in `client/host/`, as the code graph's bundle does |
| `domain/attachments` for `fileSizeText` (124), `domain/graph` for `hasRenderedForm` (804) | 2 | the bundle's own `domain/files`, the two functions alone, the size in the screen's own locale |
| `domain/events` (1,123) | 1 | the bundle's own `domain/events`, on this service's own stream rather than the console's |
| `server/utils/checkouts` (44 lines) | 1 | ported to the service: the checkouts read from bmo-job's worker list |
| `#shared/config`, `#shared/types` | 2 | the bundle's own (above) |

The server side of the console imported four more beside them, none of them a console module: `core/util/secrets`
(176 lines), whose `isSecretPath` is the exclusion list's one rule and is ported into the service's checkout
reading; the archive's SQL helper, which the store replaces; and `h3` and `node:fs`, which the Rust server
replaces.

### The components the screen draws beside Nuxt UI's: 14

The component tags of the 9 `.vue` files that are neither Nuxt UI's nor the code review's own.

| Components | Count | Here |
|---|---|---|
| `ActionGroup`, `ContentSkeleton`, `GrowingTextarea`, `SkeletonBar`, `SkeletonLine`, `SkeletonText`, `TabBar`, `TabSwitch`, `ToolBar` | 9 | bmo-ui's |
| `MarkdownText`, `ItemTip`, `DeployMark`, `TodoOpenButton`, `UsageMeters` | 5 | the host console's, declared in the bundle's `client/host/` by the props the screen hands them |

### The language pack keys: 157

`git show 3e4c032:client/i18n/locales/ko.yaml`, its `code` section's leaves read with a YAML reader; and the keys
the 25 non-test files ask outside it, found by the literal keys they pass to the translation calls.

| Keys | Count | Here |
|---|---|---|
| `code.title` 2, `code.label` 46, `code.action` 14, `code.placeholder` 5, `code.aria` 12, `code.status` 17, `code.message` 4, `code.error` 28 | 128 | the bundle's own Korean pack, carried as they stand, with a `code.error.<code>` for every code of `docs/api.md` |
| `common.action` (`cancel`, `clear_search`, `close`, `refresh`, `send`), `common.aria.search_scope`, `common.error` (`editor_failed`, `file_read_failed`, `renderer_failed`, `say_to_bmo_empty`, `send_failed`, `too_long`), `common.label` (`code`, `count`, `line_number`, `line_range`, `link`, `name`, `path`, `rendered`, `size`, `speaker_bmo`, `speaker_owner`, `view`), `common.status` (`bmo_answering`, `head_only`, `source_loading`), `screens.label.code` | 29 | the bundle's own Korean pack under its own keys; a key or value naming the agent is renamed and worded with the agent's mark |

## The tests: 9, deferred

`git grep -l -E 'code.review|code_review|CodeReview|code-review' 3e4c032 -- '*.test.ts'
'bot/bmo-server/tests/**/*.rs'`, each counted with `wc -l`.

| Old test | Lines | What it held the behaviour to |
|---|---|---|
| `client/app/domain/code.test.ts` | 254 | the explorer tree, the file-name match and its ranking, the review counts and their places, line ranges and the folder tree |
| `client/app/pages/code.test.ts` | 142 | the screen's sections and their shares, the address carrying the place, the definition jump, the sheets on a narrow screen |
| `client/app/components/CodeViewer.test.ts` | 26 | the read-only editor |
| `client/shared/definitions.test.ts` | 242 | the words a definition search takes, the patterns per language and the ordering |
| `client/server/utils/workspace.test.ts` | 373 | reading a commit's tree and files, the path fence, the commit forms accepted, the exclusions |
| `client/server/api/code/reviews.post.test.ts` | 138 | making a review: the fields required, the lines inside the file, the place's kind |
| `client/browser/code-review-touch.test.ts` | 164 | a finger's use of the screen: the sheets, the line bar, scrolling and tabs |
| `bot/bmo-server/tests/integration/control.rs` | 999 | among the control routes, the four review routes standing behind the token |
| `bot/bmo-server/tests/integration/receiver/talk.rs` | 431 | among the talks, making a review, its thread and its filing: the statuses, the refusal while answering and for a gone worker, the rows written |

## The documents: 1 whole, 6 with passages

`git grep -n -i -E 'code.review|code_review|code-review' 3e4c032 -- docs README.md`.

| Old document | What of it | Here |
|---|---|---|
| `docs/code-review.md` | whole | `docs/code-review.md`, the owner and the chat agent in place of names, and the talk and the filing as they go through the receiver and bmo-job |
| `docs/web.md` | 「Code review screen」 | `docs/screen.md` |
| `docs/web.md` | the passages listing the code review among the console's screens, sections and kept tab states | not carried: the console's own |
| `docs/control-port.md` | the four control routes | `docs/api.md` |
| `docs/documents-viewer.md` | the passage naming the code review's reading path | not carried: the document viewer's |
| `docs/index.md` | the index line | not carried: `README.md` lists this repository's documents |
| `docs/rust-migration.md` | the tables' move and the review talk among the console talks | not carried: a record of the old repository |
| `README.md` | the index line | not carried |

## The graph nodes: 2

`git show 3e4c032:graph/bot.json` and `graph/console.json`, the nodes whose files are the code review's.

| Old node | Here |
|---|---|
| `bot/code-review-talk` | this repository's `.graph`, its aliases carried |
| `console/code-review` | this repository's `.graph`, its aliases carried |

## Service units: 0

The code review ran inside the bot's server and the console's server; no unit of its own stood. This repository's
service unit is new, `deploy/bmo-review.service`.

## Personal data: 12

`git grep -n -i -E` over every file above for the owner's name, machine names, home paths and agent names: 12
occurrences, all of the owner's name in `docs/code-review.md`, and the names in the policy's `requester` and
`filing` values and in the pack's speaker and answering keys. None is carried: the documents say the owner and
the chat agent, and the values become the settings `filing.owner`, `filing.agent` and `filing.requester`.

## What bmo-markup serves and what it does not

| Text handling | Here |
|---|---|
| markdown of a thread's turns | the host console's `MarkdownText`, which renders through bmo-markup |
| cutting a body or an excerpt on a character boundary | bmo-markup's `js` lengths |
| the fuzzy file-name match and its ranking | not served: the bundle's own, specific to this screen |
| the per-language definition patterns, keywords and ordering | not served: the service's own, specific to this search |
| literal case-insensitive search of a commit | not served: git's text search |
| binary detection by a NUL near the start | not served: the service's own |
