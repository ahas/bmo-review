# Code review

## What the code review is

- **Code review over git checkouts.** A caller reads a project's checkout at a revision (its tree, its files,
  its commits and each commit's diff, a content search and a definition search), opens a review of a revision,
  writes comments on it, on a line, a range, a file, a folder or the whole, and files a follow-up item from a
  review into the job service.
- **Who writes is the caller's opaque string.** A review and a comment each carry an `authorName` the caller
  gives; the service reads nothing into it but whether two names are the same.
- **It changes no code.** Whoever takes the filed item does that.
- **Where the rest is:** the routes, the events, the codes, the calls this service makes and its settings are
  `docs/api.md`; the console screen is `docs/screen.md`. This document says what is read from where, and who
  writes what.

## Projects

### Where the projects come from

- **The projects are the graph service's.** A project is a checkout registered with bmo-graph, under the name the
  graph gives it; this service keeps no list of its own and reads the list through bmo-graph's client on each
  request.
- **The checkout** is the absolute path the graph names. Its tree is the whole repository the checkout stands in,
  as git reads it from that directory.
- **A name the graph does not know** is no project.

### Unreadable checkouts are listed with the reason

- **They never drop silently,** so whoever registered a checkout can tell why it shows nothing.
- **The states of a project:**
  - `ready`: a git repository with at least one commit.
  - `missing`: the directory does not exist.
  - `not-git`: it is not in a git repository. This service shows only versions fixed to commits, so a directory
    with no repository has nothing to show.
  - `no-commits`: a repository with no commits.
- **Reading such a project** is refused with the state's reason, and no review can be opened on it.

## The revision read

- **Everything is read at a revision, never from the work tree.**
  - Trees, file bodies, searches and diffs are all read from one commit's git objects.
  - Reviews are fixed to a revision, so the lines a caller read and the lines a comment points at are the same
    version.
  - A checkout may carry uncommitted changes at any time, and those are not a review's subject.
- **A revision left out** is the checkout's HEAD.
- **Where a project stands:** its HEAD, the branch HEAD names (none where HEAD is detached), and the count of its
  tracked files' uncommitted changes. Untracked files are not counted.

### How revisions are accepted

- **By hash only.**
  - A request may give seven to sixty-four hex characters.
  - The service uses only the full hash it resolves them to as a commit object of that repository.
- **Refused forms:** branch names, expressions such as a parent suffix, object names with a colon, and words
  starting with a dash.
- **Unresolvable** means no such revision.
- **Unreachable commits are still read** as long as the object is in the repository, even if no branch reaches
  it, since reviews may sit on commits of a rewound branch.

### Commits

- **A commit list** walks back from a revision, HEAD where none is named, newest first. Given a second, older
  revision, it stops short of that revision and everything it reaches, so it lists the commits between the two.
  Given a path, it lists only the commits that touched it.
- **Each commit** carries its hash, its short hash, its subject, its author's name, its commit time and the files
  it touched against its first parent, the excluded ones left out.
- **One commit read alone** carries its diff besides: the patch against its first parent, or against nothing for
  a root commit, over the files it touched that may be shown, cut at the diff ceiling and marked so. An excluded
  file's change never stands in it.

## Path boundaries and exclusions

### What a request may give

- **Only three things:** a project name, a revision hash, and a path relative to the repository.
- **Relative paths** are refused if they contain any empty segment, `.` or `..` segment, absolute path,
  backslash, or NUL.
  - Judged on the path after URL decoding.
  - Segments starting with a dot are accepted: files like `.github` and `.gitignore` are part of the code too.
  - Only a `.git` segment is refused.
- **The list is the permission.**
  - Only files that stand as tracked files in that revision's tree, and that no exclusion rule catches, can be
    read.
  - Paths are looked up in the revision's tree, not resolved on the file system, so there is no way at all to
    follow a symbolic link out of the checkout.

### How git is called

- **Always** with that checkout as the working directory, as an argument list without a shell.
- **Words from a request** stand only after the end of options, or are passed as option values.
- **Revisions** are passed only as full hashes the service resolved itself.
- **So request words are never read as git options.** Paths are read as literal paths, not patterns.
- **What git answers is read in its machine formats:** NUL-separated records wherever git offers them.
- **A run lasts until git ends.** No clock cuts it: its output is read up to a cap, a run past its cap is ended
  there, and a run whose request is dropped (the caller left) ends its git at once.

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
- **Code that merely has the word in its name** (a component handling credentials, an entity of that table) is
  code, not a secret, and stands.
- **One list for everything:** the exclusion rules are one list in this service, and the tree, reading, search,
  definition search, commit list, diff and comments all judge by it.
- **An excluded file** does not stand in the tree, and is a nonexistent file even when its path is given
  directly.
- **Submodules** stand as their own kind of entry but do not open, carrying the revision they pin. They are
  another repository's tree.
- **Symbolic links** stand in the tree but do not open, marked as links. Their target is not read.
- **Binary files** stand in the tree without a body: files with a NUL near the start.
- **Bodies have a size ceiling.** A larger file carries only its beginning, marked as cut.

## Search

- **File name search** is the caller's: the tree hands over the revision's permitted file list.
- **Content search** is done by the service with git's text search on that revision.
  - **Matching:** the words are literal, not a regex; case is ignored; binary files are skipped.
  - **Exclusions:** lines of excluded files do not appear in results.
  - **Limits:** results are capped per file and in total. A search runs until git has searched the whole
    revision, however long that takes, and one the ceilings cut says there is more.
- **Definition search** is done by the service with git's text search on that revision, for a name.
  - **What may be asked:** a word of letters, digits, underscore and dollar that does not begin with a digit, of
    two characters at least, and not a keyword of the reader's language (of any language where no reader's file
    is named). Anything else is not asked at all.
  - **Matching:** the patterns that stand for a declaration of that name (a function, a class, a type, a
    constant, a struct, a method), one set per language family, joined into a single search. Case matters here,
    unlike the content search, and a mention of the name, a call of it or an import of it is not a match. A
    match in a file whose language does not own the pattern that found it is dropped.
  - **The languages:** TypeScript and JavaScript with Vue, Python, Rust, C#, Lua and shell. A file of any other
    kind holds no definition here.
  - **Exclusions:** the same one list as everything else. An excluded file's lines never come back, and the tree
    of that revision is the fence, as it is for reading.
  - **Order:** the reader's own file first, then a declaration before a bare re-export, then the files nearest
    the reader's path, then the same language, then path order, so one answer never reshuffles itself.
  - **Limits:** five to a file and fifty in all, and the search runs until git is done. The five to a file are
    five definitions counted after the dropping, not five lines read: more lines than that are read from one
    file, up to a ceiling of their own, so that lines another language's pattern found never stand in the places
    of the declarations under them. What is over is dropped without being stated.
  - **Nothing found** is an empty answer, not an error.

## Reviews

A review is **a titled conversation about one revision of one project**, opened by an author.

- **Each review records:** the project's name, the revision's full hash, its title, its author's name, its
  state, when it was opened and when it last changed.
- **Its comments** stand under it, in the order they were written.
- **The items filed from it** are remembered by the number the job service answered.
- **Reviews are never deleted.**
- **The store is this service's own.** It reads nothing of the job service's or the graph service's data but
  what their clients answer.

### Its states

| State | When |
|---|---|
| `open` | It was just opened, or the last comment on it is its author's. |
| `answered` | The last comment on it is someone else's. |
| `closed` | It was closed. |

- **A comment moves an open review** to `answered` where its author is not the review's, and an answered one back
  to `open` where it is. A comment on a closed review is kept, and the review stays closed.
- **Closing and reopening** are a change of its state: `closed` from `open` or `answered`, and `open` from
  `closed`, which brings back the state its last comment gives it. Any other move is refused as a conflict;
  `answered` is reached only by a comment.

### Comments

A comment is **words on a place of the review's revision**. There are five kinds of place.

| Place | What it points at | Path | Lines |
|---|---|---|---|
| The whole | The review itself | None | None |
| Folder | Everything under one folder (the empty path is the whole repository) | A folder | None |
| File | One whole file | A file | None |
| One line | One line of one file | A file | `line` |
| Line range | Consecutive lines of one file | A file | `line` and `endLine`, the start before the end |

- **The place is judged against the review's revision:** a path must stand in its tree as a file or folder that
  may be shown; lines only with a file's path, within the file, and never on a file with no body.
- **The excerpt:** a line comment keeps the lines it points at, copied from that revision (its length has a
  ceiling), so the comment still reads where the revision later cannot.
- **No remapping.** Even if the file changed since, a comment's lines are never remapped onto another version.
  What it spoke about is that line of that revision.

## Filing an item

- **What it becomes:** an item of the job service, created through its client in the queue the caller names,
  with the title and brief the caller writes. The job service places it and decides everything else about it.
- **Its source:** the item carries the review's number, project and revision among its labels, the caller's own
  metadata as the job service keeps it.
- **After filing:** the review remembers the item's number, and one review may file several items.
- **A refusal of the job service's** comes back as this service's refusal, naming the job service's code and
  sentence.

## Read only

- **Nothing written to the checkout:** no files, no index, no refs. Only reading git commands are called.
- **The only writes** are the reviews, comments, filings and events in this service's own store, and the items
  the job service creates when this service asks.
- **The real checkout is read.** A project's checkout is the directory the graph names, so every caller reads the
  commits of the checkout itself.
