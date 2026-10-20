# bmo-review

Code review over git checkouts as one service with its own API: the projects the graph service registers, each
read at a revision (its tree, its files, its commits and each commit's diff, a content search and a definition
search), reviews of a revision with their comments on its lines, files and folders, and items filed from a
review into the job service. Nothing is written to a checkout. The graph service and the job service are reached
only through the clients they publish, as `docs/api.md` lists.

The service publishes its own client (`bmo_review::client`), its OpenAPI document (`docs/openapi.json`, printed
by `bmo-review openapi`), and its harness plugin: the MCP server `bmo-review mcp` and the `bmo-review` skill
under `plugin/skills/`.

## Tests

The repository carries one test, the repository-wide personal-data check in `tests/personal_data.rs`
(`PERSONAL-DATA.md`), and runs it with `cargo test --test personal_data`. The behaviour tests are deferred:
they are written and run here once the minimal services stand on the new repositories, and `docs/inventory.md`
lists what the old behaviour was held to. Until they stand, the build check is `cargo check --all-targets` and
the bundle's typecheck.

## Layout

| Folder | Holds |
|---|---|
| `src/` | The service, its client, its OpenAPI document and its MCP server, one crate split into its own modules |
| `migrations/` | The store, its tables made by these statements in order |
| `plugin/` | The harness plugin's skill, `bmo-review` |
| `client/` | The console bundle: the code review screen and its own Korean pack, a project of its own |
| `deploy/` | The service's user unit |
| `docs/` | The canonical statements of what the service does, and its OpenAPI document |
| `tests/` | The repository-wide personal-data check, `personal_data.rs` |
| `.graph/` | The repository's code dependency graph |

At the root stand the Cargo manifest, `LICENSE`, `VERSION`, `testbed.toml`, `rustfmt.toml`, `CLAUDE.md`,
`PERSONAL-DATA.md`, `korean-ledger.json`, `settings.toml` and `settings.local.toml.example`, the example of the
local settings a deployment gives.

## The console bundle

`client/` holds the code review screen as a console bundle by bmo-ui's console plugin contract at 0.1.0-rev.2,
packed as `bmo-review-client`: a Nuxt layer on top of bmo-ui, declared by `client/console-bundle.json`. A console
extends it as it extends bmo-ui and attaches it by that declaration alone; the screen calls this service's routes
at the address bmo-ui's `relay` module answers for `review`, and its words are its own pack under `client/i18n/`
(`docs/api.md`, 「The console bundle」). For the bundle's own install and typecheck a packed copy of bmo-ui stands
in `client/vendor/`. Its types are generated from the OpenAPI document by `bun run api`. The bundle's check is
`bun install --frozen-lockfile && bun run typecheck` inside `client/`.

## Documents

| Document | States |
|---|---|
| `docs/code-review.md` | The code review: the projects, the revision read, the fence and exclusions, search, reviews, comments and filing |
| `docs/screen.md` | The console screen's shape |
| `docs/api.md` | The routes, their answers and codes, the events, the calls to other units, the client, the harness plugin, the console bundle and the settings |
| `docs/openapi.json` | The same routes, bodies and tools as the OpenAPI 3.1 document generated from the service's types |
| `docs/inventory.md` | What this repository takes from the old repository, with each set's count and source |
| `PERSONAL-DATA.md` | What may never stand in this repository: a person's name, address, path, device or network details, a room member's, a company's, or a line of an actual conversation, and the check that holds it |

## Version

`VERSION` holds the repository's version, the crate's own, `0.1.0-rev.<n>` before the first release.
`docs/api.md`, 「Versions」, says which change is a bump.
