# Code review

## What the code review is

- **Where the owner reads and reviews:** the console's code review is where the owner browses and reads the
  checkout each worker works in, and leaves reviews on it.
- **What a review sits on:** a line or a line range, a file, or a folder, fixed to the commit it was left at.
- **What follows a review:** a thread with the chat agent, and refactoring instructions from it filed as a TODO
  item assigned to that worker.
- **It changes no code.** A worker's job does that after the item is approved.
- **Where the rest is:** the screen's shape is `docs/screen.md`; the routes, the calls this service makes and its
  settings are `docs/api.md`. This document says what is read from where, and who writes what.

## The worker's checkout

### Where the projects come from

- **The worker catalogue is bmo-job's.** One project stands for each worker bmo-job lists that names a working
  directory (its `cwd`). A worker naming none is no project.
- **No separate list:** the worker name is the project name, and the worker's label is its display name, the
  name where it has no label.
- **A support worker is no project of its own.** Its worktree is its base's repository on a job's branch, so the
  base's checkout stands for it.
- **The directory** is taken as a path standing on its own: its `.` and `..` parts are read away without asking
  the disk, a relative one read from where the service runs.
- **Shared checkouts:** when two workers point at one checkout, it stands once, under the name of the worker
  listed first.

### Unreadable checkouts are listed with the reason

- **They never drop silently.** Otherwise the owner, having added a worker, could not tell why it does not show.
- **The reasons,** each a state of the project:
  - `missing`: the directory does not exist.
  - `not-git`: it is not a git repository. This place shows only versions fixed to commits, so a directory with
    no commits has nothing to show.
  - `no-commits`: it is a repository with no commits.
- **Selecting such a worker** shows an empty explorer that states only the reason, and no review can be left.

## The version shown

- **This place shows commits, not the work tree.**
  - File lists, file bodies and searches are all read from one commit's git objects.
  - Reviews are fixed to commits, so if the screen showed the work tree, the lines the owner read and the lines a
    review points at would be different versions.
  - A worker's checkout may always have uncommitted changes while a job runs, and those are not a review target.
- **The opened commit** is the checkout's HEAD. The screen shows its short hash, branch name, commit title and
  time. A HEAD with no branch is shown as such.
- **Uncommitted changes:** if the work tree has any, their count is shown, meaning this screen does not show
  them. Untracked files are not counted.

### Choosing another commit

- **Moving:** choosing a commit moves to the version at that point.
- **The choices:**
  - Commits that touched the open file — or the chosen folder if no file is open, or the whole repository if
    neither — newest first, up to a hundred at once.
  - The current commit and the commit being viewed always stand among them, even if they did not touch it.
- **The explorer follows:**
  - Moving to a commit makes the explorer tree that commit's.
  - Files that did not exist then drop out of the tree.
  - If the open file is not in that commit, the screen says so.

### How commits are accepted

- **By hash only.**
  - A request may give seven to sixty-four hex characters.
  - The service uses only the full hash it resolves them to as a commit object of that repository.
- **Refused forms:** branch names, expressions such as a parent suffix, object names with a colon, and words
  starting with a dash.
- **Unresolvable** means no such commit.
- **Unreachable commits are still read** as long as the object is in the repository, even if no branch reaches
  it, since reviews may sit on commits of a rewound branch.

## Path boundaries and exclusions

### What a request may give

- **Only three things:** a worker name, a commit hash, and a path relative to the repository.
- **Absolute paths never leave the service.** The screen shows worker names and relative paths; a checkout's
  absolute path rides only in the facts handed to the receiver for a talk turn (below).
- **Worker names** must be in the catalogue. An unknown name is no project.
- **Relative paths** are refused if they contain any empty segment, `.` or `..` segment, absolute path,
  backslash, or NUL.
  - Judged on the path after URL decoding.
  - Segments starting with a dot are accepted: files like `.github` and `.gitignore` are part of the code too.
  - Only a `.git` segment is refused.
- **The list is the permission.**
  - Only files that stand as tracked files in that commit's tree, and that no exclusion rule catches, can be
    read.
  - Paths are looked up in the commit's tree, not resolved on the file system, so there is no way at all to
    follow a symbolic link out of the checkout.

### How git is called

- **Always** with that checkout as the working directory, as an argument list without a shell.
- **Words from a request** stand only after the end of options, or are passed as option values.
- **Commits** are passed only as full hashes the service resolved itself.
- **So request words are never read as git options.** Paths are read as literal paths, not patterns.

### What is excluded

- **What `.gitignore` excludes,** git already excludes. Only tracked files stand, so ignored and untracked files
  are not in the tree.
- **Files that look secret are excluded even when tracked.** The patterns:
  - files named `.env` or starting with `.env.` (samples ending in `.example`, `.sample`, `.template` are kept)
  - private key and certificate extensions (`.pem`, `.key`, `.p12`, `.pfx`, `.jks`, `.keystore`)
  - SSH key names (`id_rsa`, `id_dsa`, `id_ecdsa`, `id_ed25519` and anything after them)
  - `.netrc`, `.pgpass`
  - files named `credentials`, `secret` or `secrets` with no extension or a config/data extension (`.json`,
    `.yaml`, `.yml`, `.toml`, `.ini`, `.txt`)
  - everything under a directory named `env`
- **Code that merely has the word in its name** — a component handling credentials, an entity of that table —
  is code, not a secret, and stands.
- **One list for everything:** the exclusion rules are one list in this service, and the tree, reading, search,
  definition search, commit list and review creation all judge by it.
- **An excluded file** does not stand in the tree, and is a nonexistent file even when its path is given
  directly.
- **Submodules** stand as folders but do not open — marked as submodules with that commit's hash. They are
  another repository's tree.
- **Symbolic links** stand in the tree but do not open — marked as links. Their target is not read.
- **Binary files** stand in the tree without a body — files with a NUL near the start.
- **Bodies have a size ceiling.** A larger file carries only its beginning, marked as cut.

## Search

- **File name search** is done by the browser.
  - The service hands over the commit's permitted file list at once. The count has a ceiling, and going over it
    is stated.
  - A file matches when the characters typed in the search box appear in its path in order.
  - Files whose matched characters are more contiguous, and closer to the file name, rank higher.
- **Content search** is done by the service with git's text search on that commit.
  - **Matching:** the typed text is literal, not a regex; case is ignored; binary files are skipped.
  - **Exclusions:** lines of excluded files do not appear in results.
  - **Limits:** results are capped per file and in total, and time has a ceiling too. Going over is stated.
  - **Opening a hit:** pressing a matched line opens that file at that line.
- **Definition search** is done by the service with git's text search on that commit, for the word the reader
  points at in the code.
  - **What may be asked:** a word of letters, digits, underscore and dollar that does not begin with a digit, of
    two characters at least, and not a keyword of its language. Anything else is not asked at all.
  - **Matching:** the patterns that stand for a declaration of that name — a function, a class, a type, a
    constant, a struct, a method — one set per language family, joined into a single search. Case matters here,
    unlike the content search, and a mention of the name, a call of it or an import of it is not a match. A
    match in a file whose language does not own the pattern that found it is dropped.
  - **The languages:** TypeScript and JavaScript with Vue, Python, Rust, C#, Lua and shell. A file of any other
    kind holds no definition here.
  - **Exclusions:** the same one list as everything else on this screen. An excluded file's lines never come
    back, and the tree of that commit is the fence, as it is for reading.
  - **Order:** the reader's own file first, then a declaration before a bare re-export, then the files nearest
    the reader's path, then the same language, then path order, so one answer never reshuffles itself.
  - **Limits:** five to a file and fifty in all, with a time ceiling of its own. The five to a file are five
    definitions counted after the dropping, not five lines read: more lines than that are read from one file,
    up to a ceiling of their own, so that lines another language's pattern found never stand in the places of
    the declarations under them. What is over is dropped without being stated — this is a choice to make, not a
    count to report.
  - **Nothing found** is an empty answer, not an error.

## Reviews

A review is **words left on some place of some commit of some worker**. There are four kinds of place.

| Place | What it points at | Lines |
|---|---|---|
| One line | One line of one file | Start and end line are equal |
| Line range | Consecutive lines of one file | Start and end line, the start before the end |
| File | One whole file | None |
| Folder | Everything under one folder (an empty path is the whole repository) | None |

### The review table

- **Each review records:**
  - the worker name
  - the checkout path at the moment it was left
  - the commit's full hash
  - the kind of place
  - the relative path and lines
  - an **excerpt**
  - when it was left
  - the TODO items filed from it, by their numbers in bmo-job
- **The excerpt:** the lines a line review points at, copied from that commit's version (its length has a
  ceiling). File and folder reviews have no excerpt.
- **The table is this service's own,** in its own store. The items filed from a review are remembered by the
  number bmo-job answered, and this service reads nothing of bmo-job's tables.

### A review is shown at its commit

- **No remapping.** Even if the file changed since, a review's lines are not remapped onto the new version.
  Remapping is guesswork, and what the review spoke about is that line of that commit.
- **So:**
  - Reviews on the same commit as the opened one stand attached to their line, file or folder.
  - Reviews left on the same file or folder at another commit stand in the review list with that commit's short
    hash; pressing one moves the screen to that commit and opens it there.
  - If a rewind or cleanup made the commit unreadable in the repository, the review stands on its saved excerpt
    and thread, stating the commit cannot be read.

### Who writes reviews

- **Reviews are never deleted.**
- **This service writes the table.** The console asks this service's routes, which judge, write and read.
- **Leaving a review:**
  - The request gives the place and the first words together.
  - The service judges the place against the boundaries above, reads the excerpt from that commit, writes the
    review, then takes those first words as the thread's first turn.
  - Lines beyond the file's line count are refused, and so is a line review of a file with no body.

## Review threads

### The same road as other talks

- **Each review has a conversation with the chat agent.** It takes the receiver's talk road, the one item talks
  and document talks take: the receiver composes the turn and puts it into the session continuing the owner's
  private chat, with owner rights. The agency is not asked by this service.
- **What this service hands the receiver** is the review's facts and the words: the worker, its label, the
  checkout's absolute path, the commit's full hash, the kind of place, the relative path, the lines, the excerpt,
  the last turns of the thread and the words just said. The turn's head, its rules and the reading instructions
  are the receiver's to word, from its own policy.
- **Shared with the road:** queueing behind room turns, and refusing during the rest state.
- **One answer at a time, per review:** while the chat agent answers a review, new words on that review are not
  accepted.
- **No full code:** the chat agent reads it with its own tools.

### The exchange

- **Where:** the review thread table, tied to the review number.
- **Each line:** who said it — the owner or the agent — the words, the time.
- **When the service writes:** the owner's words as soon as they are received; the agent's answer when the turn
  ends.
- **Events:**
  - When an answer arrives or fails, an event carrying that review's number is sent on this service's own event
    stream.
  - When a review is added or an item filed, an event that the review list moved is sent there.
- **A turn that comes back with nothing** is kept as the thread's failure, by its code: the rest state, the
  provider's limit, its outage, or another failure. Words not answered then are marked so.

## Filing a TODO

- **What it becomes:** refactoring instructions from a review are filed as a TODO item assigned to that review's
  worker.
- **The same shape** as any other filed item, in the parts bmo-job reads an item in.
- **It awaits approval.** Filing is where this place's work ends; the owner approves in the TODO panel.

### The filing request

- **What the owner writes** in the console dialog: a title and what is to be done. A conclusion is optional.
- **The prefill:** the to-do field is prefilled with the agent's last answer in that thread, and the owner edits
  it.
- **Refused while the agent is answering** that review, since the answer on its way would be missing from the
  item's conversation record.
- **Refused for a worker gone from the catalogue** meanwhile.

### The item's shape

This service builds the parts from the review table and the review thread table, worded by its own Korean pack
(`lang/ko/`), and hands them to bmo-job as filing parts. bmo-job orders and labels the parts as it does every
item's.

1. **What the work is** — built from the review.
   - A refactoring filed from a code review on some place (kind, path, lines) of some commit of some checkout of
     some worker.
   - That the review is pinned to its commit, and the work tree is read against that commit's version.
   - For a line review, the excerpt, cut to the excerpt ceiling.
   - Its source line names the console code review's number.
2. **What is to be done** — what the owner wrote in the dialog, as written. Its source line is the owner and the
   time.
3. **The conversation, as it was said** — the whole thread in order: the owner's words as the messages that asked
   for it, the agent's answers as the reply. Each source line shows the speaker and time. Neither translated nor
   shortened.
4. **The conclusion** — what the owner wrote, or if left empty, one line this service writes: that the owner
   instructed that worker to refactor that place from that review.

### Filing details

- **Item fields:** the requester is the owner as the settings name them; approval pending; state waiting; model
  left undecided; place at the end of the queue; the worker is the review's worker. Placing and the queue's news
  are bmo-job's.
- **After filing:** the review remembers the filed item's number and shows it in the thread, and an event that
  the review list moved is sent.
- **Repeats:** one review may file several times.

## Read only

- **Nothing written to the checkout:** no files, no index, no refs. Only reading git commands are called.
- **The only writes** are the reviews and threads in this service's own store, and the items bmo-job files when
  this service asks.
- **The real checkout is read.** A worker's checkout is the directory the catalogue names, not any deployment's
  tree, so every console shows the commits of the checkout the worker actually works in.
