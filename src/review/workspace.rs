//! The reading of a worker's checkout (`docs/code-review.md`). Everything is read out of one commit's git objects,
//! never out of the working tree: the tree a commit holds is the list of what may be read, a path is looked up in
//! it rather than resolved on the disk, and a file's body is its blob. Every word a request supplied stands after
//! the end of the options or as an option's value, and a commit is only ever a full hash resolved here.

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use serde::Serialize;

use super::definitions::{self, Definition};
use super::git::{OUTPUT_CAP, Run, TIME_LIMIT, git};
use super::secrets::{is_secret_folder, is_secret_path};
use crate::settings::Limits;

// The longest path a request may name, in UTF-16 units.
const PATH_MOST: usize = 1024;
// What a commit's description is written in: hash, short hash, subject, commit time and author, fields ended by
// NUL and records by a record separator.
const COMMIT_FORMAT: &str = "--format=%H%x00%h%x00%s%x00%ct%x00%an%x1e";
// The caps of the small runs, which answer a line or a hash.
const SMALL: usize = 1024;
const STATUS_CAP: usize = 16 * 1024 * 1024;
const LINE_CAP: usize = 64 * 1024;
const SEARCH_CAP: usize = 8 * 1024 * 1024;

/// Whether a path inside a checkout is one a request may name: relative, with no empty, `.`, `..` or `.git`
/// part, no backslash and no NUL. The empty path is the whole repository.
pub fn is_workspace_path(path: &str) -> bool {
  if bmo_markup::js::units(path) > PATH_MOST {
    return false;
  }

  if path.is_empty() {
    return true;
  }

  !path.contains(['\0', '\\'])
    && !path.starts_with('/')
    && !path
      .split('/')
      .any(|part| part.is_empty() || part == "." || part == ".." || part == ".git")
}

/// A text cut to a number of UTF-16 units, never through a character.
pub fn cut_units(text: &str, limit: usize) -> String {
  let characters = bmo_markup::js::chars(text);

  bmo_markup::js::text(&characters[..bmo_markup::js::within(&characters, limit)])
}

/// One commit as the screen names it.
#[derive(Clone, Debug, Serialize)]
pub struct Commit {
  pub commit: String,
  pub short: String,
  pub subject: String,
  pub at: i64,
  pub author: String,
}

fn commits_of(output: &str) -> Vec<Commit> {
  output
    .split('\u{1e}')
    .map(|record| record.strip_prefix('\n').unwrap_or(record))
    .filter(|record| !record.is_empty())
    .map(|record| {
      let mut fields = record.split('\0');
      let mut next = || fields.next().unwrap_or_default().to_owned();
      let (commit, short, subject, at, author) = (next(), next(), next(), next(), next());

      Commit {
        commit,
        short,
        subject,
        at: at.trim().parse().unwrap_or(0),
        author,
      }
    })
    .collect()
}

/// A commit a request names, as the full hash of a commit object of this checkout, or none. Only hex is taken: a
/// ref name, an expression, an object name with a path in it or a word that could be an option is no commit here.
pub async fn resolve_commit(root: &Path, asked: &str) -> Option<String> {
  let hex =
    (7..=64).contains(&asked.len()) && asked.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'));

  if !hex {
    return None;
  }

  let object = format!("{asked}^{{commit}}");
  let run = git(
    root,
    &["rev-parse", "--verify", "--quiet", "--end-of-options", &object],
    SMALL,
    TIME_LIMIT,
  )
  .await;
  let full = run.text().trim().to_owned();
  let whole = (full.len() == 40 || full.len() == 64) && full.bytes().all(|byte| byte.is_ascii_hexdigit());

  (run.ok() && whole).then_some(full)
}

/// One commit's description, where the commit can be read.
pub async fn read_commit(root: &Path, commit: &str) -> Option<Commit> {
  let run =
    git(root, &["show", "-s", COMMIT_FORMAT, "--end-of-options", commit], OUTPUT_CAP, TIME_LIMIT).await;

  if run.ok() { commits_of(&run.text()).into_iter().next() } else { None }
}

/// Whether a checkout has anything to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum State {
  Ready,
  Missing,
  NotGit,
  NoCommits,
}

/// Where a checkout stands.
#[derive(Clone, Debug, Serialize)]
pub struct CheckoutState {
  pub state: State,
  pub head: Option<Commit>,
  pub branch: Option<String>,
  pub dirty: usize,
}

impl CheckoutState {
  fn unreadable(state: State) -> Self {
    Self {
      state,
      head: None,
      branch: None,
      dirty: 0,
    }
  }
}

/// Whether a worker's checkout has anything to show, and where it stands: its HEAD, its branch and how many
/// uncommitted changes its tracked files carry.
pub async fn checkout_state(root: &Path) -> CheckoutState {
  if !tokio::fs::metadata(root).await.is_ok_and(|found| found.is_dir()) {
    return CheckoutState::unreadable(State::Missing);
  }

  if !git(root, &["rev-parse", "--show-toplevel"], 4096, TIME_LIMIT).await.ok() {
    return CheckoutState::unreadable(State::NotGit);
  }

  let Some(head) = read_commit(root, "HEAD").await else {
    return CheckoutState::unreadable(State::NoCommits);
  };
  let branch = git(root, &["symbolic-ref", "--quiet", "--short", "HEAD"], SMALL, TIME_LIMIT).await;
  let status =
    git(root, &["status", "--porcelain", "--untracked-files=no", "-z"], STATUS_CAP, TIME_LIMIT).await;

  CheckoutState {
    state: State::Ready,
    head: Some(head),
    branch: branch.ok().then(|| branch.text().trim().to_owned()),
    dirty: status.text().split('\0').filter(|entry| !entry.is_empty()).count(),
  }
}

/// What a tree entry is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EntryKind {
  File,
  Folder,
  Link,
  Submodule,
}

impl EntryKind {
  fn of(mode: &str, kind: &str) -> Option<Self> {
    match kind {
      "tree" => Some(Self::Folder),
      "commit" => Some(Self::Submodule),
      "blob" if mode == "120000" => Some(Self::Link),
      "blob" => Some(Self::File),
      _ => None,
    }
  }
}

/// One entry of a commit's tree as the screen shows it.
#[derive(Clone, Debug, Serialize)]
pub struct Entry {
  pub path: String,
  pub kind: EntryKind,
  /// A file's bytes.
  pub size: Option<u64>,
  /// A submodule's commit.
  pub pinned: Option<String>,
}

struct TreeLine {
  path: String,
  kind: EntryKind,
  object: String,
  size: Option<u64>,
}

impl TreeLine {
  fn allowed(&self) -> bool {
    if self.kind == EntryKind::Folder {
      !is_secret_folder(&self.path)
    } else {
      !is_secret_path(&self.path)
    }
  }

  fn entry(&self) -> Entry {
    Entry {
      path: self.path.clone(),
      kind: self.kind,
      size: if self.kind == EntryKind::File { self.size } else { None },
      pinned: (self.kind == EntryKind::Submodule).then(|| self.object.clone()),
    }
  }
}

// `ls-tree -l -z` writes "<mode> <type> <object> <size>\t<path>" records, each ended by NUL.
fn tree_lines(output: &str) -> Vec<TreeLine> {
  output
    .split('\0')
    .filter_map(|record| {
      let (head, path) = record.split_once('\t')?;
      let mut fields = head.split(' ').filter(|field| !field.is_empty());
      let (mode, kind, object, size) =
        (fields.next()?, fields.next()?, fields.next()?, fields.next().unwrap_or("-"));

      Some(TreeLine {
        path: path.to_owned(),
        kind: EntryKind::of(mode, kind)?,
        object: object.to_owned(),
        size: size.parse().ok(),
      })
    })
    .collect()
}

/// A commit's tree as it may be shown.
#[derive(Clone, Debug, Serialize)]
pub struct Tree {
  pub entries: Vec<Entry>,
  pub cut: bool,
}

/// Everything one commit's tree holds that may be shown, folders included, in git's order, up to the ceiling.
pub async fn list_tree(root: &Path, commit: &str, limits: &Limits) -> Tree {
  let run = git(
    root,
    &["ls-tree", "-r", "-t", "-l", "-z", "--full-tree", "--end-of-options", commit],
    OUTPUT_CAP,
    TIME_LIMIT,
  )
  .await;

  if !run.ok() {
    return Tree {
      entries: Vec::new(),
      cut: false,
    };
  }

  let mut entries: Vec<Entry> = tree_lines(&run.text())
    .iter()
    .filter(|line| line.allowed())
    .map(TreeLine::entry)
    .collect();
  let cut = run.cut || entries.len() > limits.tree_entries;

  entries.truncate(limits.tree_entries);

  Tree { entries, cut }
}

async fn tree_line(root: &Path, commit: &str, path: &str) -> Option<TreeLine> {
  let run = git(
    root,
    &["ls-tree", "-l", "-z", "--full-tree", "--end-of-options", commit, "--", path],
    LINE_CAP,
    TIME_LIMIT,
  )
  .await;

  if !run.ok() {
    return None;
  }

  tree_lines(&run.text())
    .into_iter()
    .find(|line| line.path == path)
    .filter(TreeLine::allowed)
}

/// The one entry a path names in a commit's tree, where it stands there and may be shown. The empty path is the
/// repository's own folder.
pub async fn find_entry(root: &Path, commit: &str, path: &str) -> Option<Entry> {
  if !is_workspace_path(path) {
    return None;
  }

  if path.is_empty() {
    return Some(Entry {
      path: String::new(),
      kind: EntryKind::Folder,
      size: None,
      pinned: None,
    });
  }

  tree_line(root, commit, path).await.map(|line| line.entry())
}

/// One file as it stood at one commit.
#[derive(Clone, Debug, Serialize)]
pub struct File {
  pub project: String,
  pub commit: String,
  pub path: String,
  pub size: u64,
  /// The body; none for a binary file.
  pub text: Option<String>,
  pub lines: usize,
  pub cut: bool,
}

/// One file of a commit, read out of its blob. A file with a NUL near its head is binary and comes back without a
/// body; a file past the ceiling comes back with its head and says so.
pub async fn read_tree_file(root: &Path, commit: &str, path: &str, limits: &Limits) -> Option<File> {
  if !is_workspace_path(path) || path.is_empty() {
    return None;
  }

  let line = tree_line(root, commit, path).await?;

  if line.kind != EntryKind::File {
    return None;
  }

  let run = git(root, &["cat-file", "blob", &line.object], limits.file_bytes, TIME_LIMIT).await;

  if !run.ok() {
    return None;
  }

  let size = line.size.unwrap_or(run.out.len() as u64);
  let probe = &run.out[..run.out.len().min(limits.binary_probe)];
  let file = |text: Option<String>, lines: usize, cut: bool| File {
    project: String::new(),
    commit: commit.to_owned(),
    path: path.to_owned(),
    size,
    text,
    lines,
    cut,
  };

  if probe.contains(&0) {
    return Some(file(None, 0, false));
  }

  let mut body = run.text();
  let cut = run.cut || size > run.out.len() as u64;

  // A cut through a multi-byte character reads as a replacement at the very end; it is dropped rather than shown
  // as a character the file never had.
  if cut && body.ends_with('\u{fffd}') {
    body.pop();
  }

  let lines = body.split('\n').count();

  Some(file(Some(body), lines, cut))
}

/// The commits reachable from HEAD that touched a path, newest first, up to the ceiling; none where the path is no
/// path that may be shown. The empty path is the whole repository.
pub async fn list_commits(root: &Path, path: &str, limits: &Limits) -> Option<Vec<Commit>> {
  if !is_workspace_path(path) || (!path.is_empty() && (is_secret_path(path) || is_secret_folder(path))) {
    return None;
  }

  let most = format!("--max-count={}", limits.commits);
  let mut args = vec!["log", &most, COMMIT_FORMAT, "HEAD", "--"];

  if !path.is_empty() {
    args.push(path);
  }

  let run = git(root, &args, OUTPUT_CAP, TIME_LIMIT).await;

  run.ok().then(|| commits_of(&run.text()))
}

/// One line a search found.
#[derive(Clone, Debug, Serialize)]
pub struct Match {
  pub path: String,
  pub line: u64,
  pub text: String,
}

// `grep -n -z` writes "<commit>:<path>\0<line>\0<text>\n" records.
fn matches_of(output: &str, commit: &str, limits: &Limits) -> Vec<Match> {
  let prefix = format!("{commit}:");
  let mut matches = Vec::new();
  let mut rest = output;

  while let Some((name, after)) = rest.split_once('\0') {
    let Some((line, after)) = after.split_once('\0') else { break };
    let Some((text, after)) = after.split_once('\n') else { break };

    if let Some(path) = name.strip_prefix(&prefix) {
      matches.push(Match {
        path: path.to_owned(),
        line: line.parse().unwrap_or(0),
        text: cut_units(text, limits.match_line),
      });
    }

    rest = after;
  }

  matches
}

/// What a content search found.
#[derive(Clone, Debug, Serialize)]
pub struct Search {
  pub matches: Vec<Match>,
  pub cut: bool,
}

/// The lines of one commit that hold the words, as written and ignoring case. Binary files are passed over, a
/// secret's lines never come back, and the search stops at its ceilings and its time. git's grep takes no
/// end-of-options marker: the words go in as the value of its pattern option.
pub async fn search_tree(root: &Path, commit: &str, query: &str, limits: &Limits) -> Search {
  let most = format!("--max-count={}", limits.search_per_file);
  let args = ["grep", "-I", "-n", "-i", "-F", "-z", "--no-color", &most, "-e", query, commit, "--"];
  let run = git(root, &args, SEARCH_CAP, Duration::from_millis(limits.search_timeout_ms)).await;
  let mut matches: Vec<Match> = matches_of(&run.text(), commit, limits)
    .into_iter()
    .filter(|found| !is_secret_path(&found.path))
    .collect();
  let cut = run.cut || run.code > 1 || matches.len() > limits.search_total;

  matches.truncate(limits.search_total);

  Search { matches, cut }
}

/// Where a name is declared at one commit: every family's shapes run at once, then each line kept only where its
/// file's own family owns a shape that fits it, at most so many to a file, ordered and capped. git is asked for
/// more of a file than the answer keeps, so the lines a file's own family drops never hide the declarations under
/// them.
pub async fn find_definitions(
  root: &Path,
  commit: &str,
  reader: &str,
  name: &str,
  limits: &Limits,
) -> Vec<Definition> {
  if definitions::definition_name(name, reader).is_none() {
    return Vec::new();
  }

  let most = format!("--max-count={}", limits.definition_scan_per_file);
  let pattern = definitions::definition_pattern(name);
  let args = ["grep", "-I", "-n", "-E", "-z", "--no-color", &most, "-e", &pattern, commit, "--"];
  let run: Run = git(root, &args, SEARCH_CAP, Duration::from_millis(limits.definition_timeout_ms)).await;
  let mut taken: HashMap<String, usize> = HashMap::new();
  let mut found = Vec::new();

  for found_line in matches_of(&run.text(), commit, limits) {
    let kind = if is_secret_path(&found_line.path) {
      None
    } else {
      definitions::definition_kind(&found_line.path, &found_line.text, name)
    };
    let already = taken.entry(found_line.path.clone()).or_default();

    if let Some(kind) = kind
      && *already < limits.definition_per_file
    {
      *already += 1;
      found.push(Definition {
        path: found_line.path,
        line: found_line.line,
        text: found_line.text,
        kind,
      });
    }
  }

  definitions::rank(&mut found, reader);
  found.truncate(limits.definition_total);

  found
}
