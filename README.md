# bmo-review

The code review as one service with its own API: the console's code review screen reading each worker's
checkout at a commit — its tree, its files, its commits, a content search and a definition search — the reviews
pinned to a line, a range, a file or a folder of that commit, the talk under each review with the chat agent,
and the TODO items filed from a review for that worker. Nothing is written to a checkout. Every other unit is
reached through its API, as `docs/api.md` lists.

## Tests are deferred

No test is carried, written or run in this repository yet. The tests are written and run here once the minimal
services stand on the new repositories; until then the only check is the build check, `cargo check
--all-targets` and the bundle's typecheck. `docs/inventory.md` lists the tests the old behaviour was held to.

## Layout

| Folder | Holds |
|---|---|
| `src/` | The service, one crate split into its own modules |
| `migrations/` | The reviews' store, its tables made by these statements in order |
| `lang/` | This repository's Korean pack for the words of an item filed from a review |
| `client/` | The console plugin bundle: the code review screen and its own Korean pack, a project of its own |
| `deploy/` | The service's user unit |
| `docs/` | The canonical statements of what the service does |

At the root stand the Cargo manifest, `VERSION`, `testbed.toml`, `rustfmt.toml`, `CLAUDE.md`,
`korean-ledger.json`, `settings.toml` and `settings.local.toml.example`, the example of the local settings a
deployment gives.

## The console plugin bundle

`client/` holds the console's code review screen as a Nuxt layer on top of bmo-ui, packed as
`bmo-review-client`. A host console extends it as it extends bmo-ui, and owes it:

- bmo-ui itself, at the version the bundle's peer names; for the bundle's own install and typecheck a packed
  copy stands in `client/vendor/`;
- its i18n module, loading the bundle's own Korean pack beside the host's: the screen's words and
  `code.error.<code>` for each code of `docs/api.md` are the bundle's, not the host's;
- the components `MarkdownText`, `ItemTip`, `DeployMark`, `TodoOpenButton` and `UsageMeters`, declared in
  `client/host/` by the props the screen hands them, the tab-state store the screen keeps its open tabs in, and
  the TODO panel an item's number opens;
- every route under `/api/code/`, the event stream included, passed on to this service behind the console's
  sign-in (`docs/api.md`, 「What the console does」).

The screen calls only those routes, and says a refusal by its code in its own pack's words. The bundle's check is
`bun install --frozen-lockfile && bun run typecheck` inside `client/`.

## Documents

| Document | States |
|---|---|
| `docs/code-review.md` | The code review: the checkouts, the version shown, the fence and exclusions, search, reviews, threads and filing |
| `docs/screen.md` | The console screen's shape |
| `docs/api.md` | The routes, their answers and refusals, the events, the calls to other units, what the siblings lack, the settings and what the console does |
| `docs/inventory.md` | What this repository takes from the old repository, with each set's count and source |

## Version

`VERSION` holds the repository's version in semver, the crate's own. `docs/api.md`, 「Versions」, says which
change moves which part.
