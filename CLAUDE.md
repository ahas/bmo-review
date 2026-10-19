# Working in bmo-review

## The three standing principles

These hold every bmo-next repository, and every change here (archive document #301, its principles):

1. **Each service runs on its own, with no context bleeding in from other repositories:** it builds,
   tests and runs from its own repository, data and settings, and never reads another repository's
   code, files, data folders or tables.
2. **Services talk to each other only through documented APIs** (HTTP routes, events, a documented
   command), never by importing another service's code.
3. **References run one way:** lower code uses the base, and the base never knows what is below it (no
   per-provider branch such as `if telegram`, no list of the units below, no dependency on any service);
   where two repositories must know the same policy value, the lowest service in the dependency
   direction owns it and the ones above read it through that service's API; and each repository holds
   only what it uses itself, since the same thing kept in several places is the wrong structure.

## The documents come first

- `docs/` is the canonical statement of what the service does; the code is one realisation of it.
  Before changing code, read the document covering that area: `docs/code-review.md` for what is read,
  fenced, written and filed, `docs/screen.md` for the console screen, `docs/api.md` for the routes, the
  events, the calls, the refusals and the settings, and `docs/inventory.md` for what came from where.
- A change that would make a document wrong or incomplete repairs the document first, stating what is
  true now, and the code is written to match.
- A document states what is true now: no change history, no rationale told through an earlier design,
  no pasted source, signature or config fragment, and no file-and-line pins.

## Boundaries

- **Every other unit is reached through its API,** never through its code, its tables or its files.
  The routes this service serves and every call it makes to another unit are listed in `docs/api.md`.
  A worker's checkout is read only through git, at a commit, as `docs/code-review.md` fences it.
- **A client of another service is one module named for that service** under `src/clients/`, holding
  only the calls this repository makes, so it can be swapped for that service's own client.
- **A call a sibling does not document is not invented.** It stands in `docs/api.md`, 「What the
  siblings lack」, and the call answers its own code until the sibling documents a route.
- **Dependencies:** crates from the registry pinned in `Cargo.toml`, and among other repositories only
  the base libraries `bmo-chat`, `bmo-chat-client`, `bmo-ui` and `bmo-markup`.
- **The plumbing knows no route.** A route registers itself beside its own answer; adding one changes no
  file but its own.
- **No lookup table standing in for polymorphism.** A value that differs per kind of place or per
  language belongs to that kind or that language.
- **No hand-written parser for a general format.** TOML, YAML, JSON, HTTP headers, git's output formats
  where git offers a machine one, and their kin are read by a maintained library or by git's own
  machine-readable output.

## Language

- Everything in this repository is English: code, comments, strings, tests, documents, commit messages
  and branch names. No Korean in a code file or a test, written plainly or as escapes.
- **The words a person reads are this repository's own Korean pack,** never bmo-web's: the screen's
  words in the console plugin bundle's locale file, and the words of an item's parts in `lang/ko/`. Code
  names them by key. Every pack file is recorded in `korean-ledger.json` with its count of Hangul lines;
  a code file the ledger does not name holds none.
- What the service answers is an English sentence for a log and a snake_case code the screen shows in
  its pack's words (`docs/api.md`, 「Refusals」). No model prompt is written here: the receiver words the
  talk's turn.
- No personal data: no user names, home directories, machine names, internal addresses or names of any
  agent. The owner's and the agent's names, and what the service reaches, come from its settings, never
  from the code or a pack, which carry the marks `{owner}` and `{agent}`.

## Layout

- `Cargo.toml` is the only manifest at the root, one crate split into its own modules, and no directory
  is named after its parent.
- The console plugin bundle stands in `client/`, a project of its own.
- Tests live in `tests/`; no test module stands in `src/`. The tests are deferred for now (README.md).

## The stages of a change

1. **Writing:** the only check is `cargo check --all-targets`, and for the bundle its typecheck.
2. **Review:** read the whole diff against the document governing it, then `cargo fmt --all --check`
   and `cargo clippy --all-targets -- -D warnings`.
3. **Verification:** the tests touched, then the whole suite once, output written to a file and the
   file read. Until the deferred tests stand, a change reaches stage 2 and is reported as such.
