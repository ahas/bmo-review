# bmo-review

Code review over git checkouts as one service with its own API: the checkouts its settings name, each
read at a revision (its tree, its files, its commits and each commit's diff, a content search and a definition
search), reviews of a revision with their comments on its lines, files and folders, and items filed from a
review into the job service. Nothing is written to a checkout. The job service is reached
only through the clients they publish, as `docs/api.md` lists.

The service publishes its own client (`bmo_review::client`), its OpenAPI document (`docs/openapi.json`, printed
by `bmo-review openapi`), and its harness plugin: the MCP server `bmo-review mcp` and the `bmo-review` skill
under `plugin/skills/`.

## Tests

The testbed's `personal-data` check holds the personal-data rule (`PERSONAL-DATA.md`). The repository carries
the import's test in `tests/import.rs`; and the console face's
test in `tests/screen.rs`, which reads the manifest the service serves through bmo-ui's own reading on the release
the client vendors, run by bmo-chat's `bmo-console-face` (Bun on the machine), and holds which credential each of
the service's routes takes; and the screen's select check in `tests/choices.rs`, which holds that every value the
screen lets be chosen from a list stands in bmo-ui's one select. The console bundle carries one browser test,
`client/tests/browser/composer-frame.test.ts`, which opens the screen in its dev host `client/dev-app/` over a
stand-in of the service and reads off the page, in both schemes, that the box a comment is written in has one frame
among the code at 1280 and in the modal at 360 and at 1280, that the modal asks before throwing written words away
and closes at once with none, and that its foot stays on a screen as short as a phone's with its keyboard up
(`docs/screen.md`, 「Reviewing」); the testbed runs it in its browser lane as the client's manifest declares, and
the bundle's typecheck reads its types through `client/tests/tsconfig.json`.
The other behaviour tests are deferred: they are written and run here once the minimal services stand on the new
repositories, and `docs/inventory.md` lists what the old behaviour was held to. Until
they stand, the build check is `cargo check --all-targets` and the bundle's typecheck.

## Layout

| Folder | Holds |
|---|---|
| `src/` | The service, its client, its OpenAPI document and its MCP server, one crate split into its own modules |
| `migrations/` | The store, its tables made by these statements in order |
| `plugin/` | The harness plugin's skill, `bmo-review` |
| `client/` | The console bundle: the code review screen, its own Korean pack, its build under `bundle/`, its dev host under `dev-app/` and its browser tests, a project of its own |
| `deploy/` | The service's user unit |
| `docs/` | The canonical statements of what the service does, and its OpenAPI document |
| `tests/` | The import's test, the console face's test and the screen's select check |

At the root stand the Cargo manifest, `LICENSE`, `VERSION`, `testbed.toml`, `rustfmt.toml`, `CLAUDE.md`,
`PERSONAL-DATA.md`, `korean-ledger.json`, `settings.toml` and `settings.local.toml.example`, the example of the
local settings a deployment gives.

## The console bundle

`client/` holds the code review screen as a console bundle under bmo-ui's console plugin contract (bmo-ui
`docs/console-bundles.md`), built on the bmo-ui release bmo-chat's `shared-versions.toml` pins, whose file stands in
`client/vendor/` as the release verb wrote it. `bun run bundle` builds it into `client/dist/` with bmo-ui's bundle
build, and a deployment builds it into the release before the release goes live. The service serves it through
bmo-chat's `bmo-console-face` (`docs/api.md`, 「The console face」), and its words are its own pack under
`client/i18n/`. The same sources are typechecked as a Nuxt project; its types are generated from the OpenAPI
document by `bun run api`. The bundle's check is `bun install --frozen-lockfile && bun run typecheck` inside
`client/`.

## Documents

| Document | States |
|---|---|
| `docs/code-review.md` | The code review: the projects, the revision read, the fence and exclusions, search, reviews, comments and filing |
| `docs/screen.md` | The console screen's shape |
| `docs/api.md` | The routes, their answers and codes, the events, the calls to other units, the client, the harness plugin, the console face and the settings |
| `docs/openapi.json` | The same routes, bodies and tools as the OpenAPI 3.1 document generated from the service's types |
| `docs/inventory.md` | What this repository takes from the old repository, with each set's count and source |
| `PERSONAL-DATA.md` | What may never stand in this repository: a person's name, address, path, device or network details, a room member's, a company's, or a line of an actual conversation, and the check that holds it |

## Version

`VERSION` holds the repository's version, the crate's own, `0.1.0-rev.<n>` before the first release. It moves
only at a release the owner declares (`docs/api.md`, 「Versions」).
