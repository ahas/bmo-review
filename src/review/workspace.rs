//! The reading of a project's checkout (`docs/code-review.md`). Everything is read out of one revision's git
//! objects, never out of the working tree: the tree a revision holds is the list of what may be read, a path is
//! looked up in it rather than resolved on the disk, and a file's body is its blob. Every word a request supplied
//! stands after the end of the options or as an option's value, and a revision is only ever a full hash resolved
//! here. git's answers are read in its NUL-separated machine formats.

use std::collections::HashMap;
use std::path::Path;

use super::definitions::{self, Definition};
use super::git::{OUTPUT_CAP, Run, git};
use super::secrets::{is_secret_folder, is_secret_path};
use crate::settings::Limits;
use crate::wire::{Commit, EntryKind, FileSource, Match, ProjectState, TreeEntry};

// The longest path a request may name, in UTF-16 units.
const PATH_MOST: usize = 1024;
// What a commit's description is written in: a record separator, then hash, short hash, subject, commit time and
// author, each ended by NUL, then the files it touched as `--name-only -z` writes them.
const COMMIT_FORMAT: &str = "--format=%x1e%H%x00%h%x00%s%x00%ct%x00%an%x00";
// The caps of the small runs, which answer a line or a hash.
const SMALL: usize = 1024;
const STATUS_CAP: usize = 16 * 1024 * 1024;
const LINE_CAP: usize = 64 * 1024;
const SEARCH_CAP: usize = 8 * 1024 * 1024;

/// Whether a path inside a checkout is one a request may name: relative, with no empty, `.`, `..` or `.git`
/// part, no backslash and no NUL. The empty path is the whole repository.
pub fn is_workspace_path(path: &str) -> bool {
  if bmo_text::js::units(path) > PATH_MOST {
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
  let characters = bmo_text::js::chars(text);

  bmo_text::js::text(&characters[..bmo_text::js::within(&characters, limit)])
}

/// Output read as text, where a cap cut through a multi-byte character at its very end: the replacement it reads
/// as is dropped rather than shown as a character the source never had.
fn text_of(run: &Run, cut: bool) -> String {
  let mut text = run.text();

  if cut && text.ends_with('\u{fffd}') {
    text.pop();
  }

  text
}

fn commits_of(output: &str) -> Vec<Commit> {
  output
    .split('\u{1e}')
    .filter(|record| !record.is_empty())
    .map(|record| {
      let mut fields = record.split('\0');
      let mut next = || fields.next().unwrap_or_default().to_owned();
      let (id, short_id, subject, at, author_name) = (next(), next(), next(), next(), next());
      let files = fields
        .map(|name| name.trim_start_matches('\n'))
        .filter(|name| !name.is_empty() && !is_secret_path(name))
        .map(str::to_owned)
        .collect();

      Commit {
        id,
        short_id,
        subject,
        at: at.trim().parse::<i64>().unwrap_or(0).saturating_mul(1000),
        author_name,
        files,
        diff: None,
        cut: false,
      }
    })
    .collect()
}

/// A revision a request names, as the full hash of a commit object of this checkout, or none. Only hex is taken:
/// a ref name, an expression, an object name with a path in it or a word that could be an option is no revision
/// here.
pub async fn resolve_commit(root: &Path, asked: &str) -> Option<String> {
  let hex =
    (7..=64).contains(&asked.len()) && asked.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'));

  if !hex {
    return None;
  }

  verified(root, &format!("{asked}^{{commit}}")).await
}

/// The full hash an object name the service wrote verifies to as a commit.
async fn verified(root: &Path, object: &str) -> Option<String> {
  let run = git(root, &["rev-parse", "--verify", "--quiet", "--end-of-options", object], SMALL).await;
  let full = run.text().trim().to_owned();
  let whole = (full.len() == 40 || full.len() == 64) && full.bytes().all(|byte| byte.is_ascii_hexdigit());

  (run.ok() && whole).then_some(full)
}

/// The commits of these full hashes described, in the order given, each with the files it touched against its
/// first parent.
pub async fn describe(root: &Path, hashes: &[String]) -> Option<Vec<Commit>> {
  if hashes.is_empty() {
    return Some(Vec::new());
  }

  let mut args = vec![
    "log",
    "--no-walk=unsorted",
    "--no-renames",
    "--diff-merges=first-parent",
    "--name-only",
    "-z",
    COMMIT_FORMAT,
    "--end-of-options",
  ];

  args.extend(hashes.iter().map(String::as_str));
  args.push("--");

  let run = git(root, &args, OUTPUT_CAP).await;

  run.ok().then(|| commits_of(&run.text()))
}

/// Where a checkout stands: whether it has anything to show, its HEAD, its branch and how many uncommitted
/// changes its tracked files carry.
#[derive(Clone, Debug)]
pub struct Standing {
  pub state: ProjectState,
  pub head: Option<String>,
  pub branch: Option<String>,
  pub uncommitted: u64,
}

impl Standing {
  fn unreadable(state: ProjectState) -> Self {
    Self {
      state,
      head: None,
      branch: None,
      uncommitted: 0,
    }
  }
}

/// A checkout's HEAD as a full hash, or the state that says why it has none to show.
pub async fn head_of(root: &Path) -> Result<String, ProjectState> {
  if !tokio::fs::metadata(root).await.is_ok_and(|found| found.is_dir()) {
    return Err(ProjectState::Missing);
  }

  if !git(root, &["rev-parse", "--show-toplevel"], 4096).await.ok() {
    return Err(ProjectState::NotGit);
  }

  verified(root, "HEAD^{commit}").await.ok_or(ProjectState::NoCommits)
}

/// Whether a checkout has anything to show, and where it stands.
pub async fn checkout_state(root: &Path) -> Standing {
  let head = match head_of(root).await {
    Ok(head) => head,
    Err(state) => return Standing::unreadable(state),
  };
  let branch = git(root, &["symbolic-ref", "--quiet", "--short", "HEAD"], SMALL).await;
  let status = git(root, &["status", "--porcelain", "--untracked-files=no", "-z"], STATUS_CAP).await;

  Standing {
    state: ProjectState::Ready,
    head: Some(head),
    branch: branch.ok().then(|| branch.text().trim().to_owned()),
    uncommitted: status.text().split('\0').filter(|entry| !entry.is_empty()).count() as u64,
  }
}

fn entry_kind(mode: &str, kind: &str) -> Option<EntryKind> {
  match kind {
    "tree" => Some(EntryKind::Folder),
    "commit" => Some(EntryKind::Submodule),
    "blob" if mode == "120000" => Some(EntryKind::Link),
    "blob" => Some(EntryKind::File),
    _ => None,
  }
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

  fn entry(&self) -> TreeEntry {
    TreeEntry {
      path: self.path.clone(),
      kind: self.kind,
      byte_count: if self.kind == EntryKind::File { self.size } else { None },
      pinned_revision: (self.kind == EntryKind::Submodule).then(|| self.object.clone()),
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
        kind: entry_kind(mode, kind)?,
        object: object.to_owned(),
        size: size.parse().ok(),
      })
    })
    .collect()
}

/// Everything one revision's tree holds that may be shown, folders included, in path order; none where the tree
/// cannot be read.
pub async fn list_tree(root: &Path, commit: &str) -> Option<Vec<TreeEntry>> {
  let run = git(
    root,
    &["ls-tree", "-r", "-t", "-l", "-z", "--full-tree", "--end-of-options", commit],
    OUTPUT_CAP,
  )
  .await;

  if !run.ok() {
    return None;
  }

  let mut entries: Vec<TreeEntry> = tree_lines(&run.text())
    .iter()
    .filter(|line| line.allowed())
    .map(TreeLine::entry)
    .collect();

  entries.sort_by(|one, other| one.path.cmp(&other.path));

  Some(entries)
}

async fn tree_line(root: &Path, commit: &str, path: &str) -> Option<TreeLine> {
  let run = git(
    root,
    &["ls-tree", "-l", "-z", "--full-tree", "--end-of-options", commit, "--", path],
    LINE_CAP,
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

/// The one entry a path names in a revision's tree, where it stands there and may be shown. The empty path is the
/// repository's own folder.
pub async fn find_entry(root: &Path, commit: &str, path: &str) -> Option<TreeEntry> {
  if !is_workspace_path(path) {
    return None;
  }

  if path.is_empty() {
    return Some(TreeEntry {
      path: String::new(),
      kind: EntryKind::Folder,
      byte_count: None,
      pinned_revision: None,
    });
  }

  tree_line(root, commit, path).await.map(|line| line.entry())
}

/// One file of a revision, read out of its blob. A file with a NUL near its head is binary and comes back
/// without a body; a file past the ceiling comes back with its head and says so.
pub async fn read_tree_file(root: &Path, commit: &str, path: &str, limits: &Limits) -> Option<FileSource> {
  if !is_workspace_path(path) || path.is_empty() {
    return None;
  }

  let line = tree_line(root, commit, path).await?;

  if line.kind != EntryKind::File {
    return None;
  }

  let run = git(root, &["cat-file", "blob", &line.object], limits.file_bytes).await;

  if !run.ok() {
    return None;
  }

  let byte_count = line.size.unwrap_or(run.out.len() as u64);
  let probe = &run.out[..run.out.len().min(limits.binary_probe)];
  let file = |text: Option<String>, line_count: u64, cut: bool| FileSource {
    path: path.to_owned(),
    revision: commit.to_owned(),
    byte_count,
    text,
    line_count,
    cut,
  };

  if probe.contains(&0) {
    return Some(file(None, 0, false));
  }

  let cut = run.cut || byte_count > run.out.len() as u64;
  let body = text_of(&run, cut);
  let line_count = body.split('\n').count() as u64;

  Some(file(Some(body), line_count, cut))
}

/// Why a commit list could not be read.
pub enum ListFailure {
  /// The path is no path that may be shown.
  Path,
  /// The cursor is no commit of the list.
  Cursor,
  /// git could not walk the history.
  Unreadable,
}

/// A page of the commits reachable from `to` and not from `from`, that touched `path` where one is named, newest
/// first, past the commit `after` names; and whether more stand.
pub async fn list_commits(
  root: &Path,
  to: &str,
  from: Option<&str>,
  path: &str,
  after: Option<&str>,
  limit: usize,
) -> Result<(Vec<Commit>, bool), ListFailure> {
  if !is_workspace_path(path) || (!path.is_empty() && (is_secret_path(path) || is_secret_folder(path))) {
    return Err(ListFailure::Path);
  }

  let left_out = from.map(|from| format!("^{from}"));
  let mut args = vec!["rev-list", "--end-of-options", to];

  args.extend(left_out.as_deref());
  args.push("--");

  if !path.is_empty() {
    args.push(path);
  }

  let run = git(root, &args, OUTPUT_CAP).await;

  if !run.ok() {
    return Err(ListFailure::Unreadable);
  }

  let text = run.text();
  let hashes: Vec<&str> = text.lines().map(str::trim).filter(|line| !line.is_empty()).collect();
  let start = match after {
    None => 0,
    Some(after) => hashes.iter().position(|hash| *hash == after).ok_or(ListFailure::Cursor)? + 1,
  };
  let page: Vec<String> = hashes.iter().skip(start).take(limit).map(|hash| (*hash).to_owned()).collect();
  let more = hashes.len() > start + page.len();
  let commits = describe(root, &page).await.ok_or(ListFailure::Unreadable)?;

  Ok((commits, more))
}

/// One commit with its diff: the patch against its first parent, or against nothing for a root commit, over the
/// files it touched that may be shown, cut at the diff ceiling.
pub async fn read_commit_diff(root: &Path, commit: &str, limits: &Limits) -> Option<Commit> {
  let mut described = describe(root, &[commit.to_owned()]).await?.into_iter().next()?;

  if described.files.is_empty() {
    described.diff = Some(String::new());
    return Some(described);
  }

  let parents = git(root, &["rev-list", "--parents", "-n", "1", "--end-of-options", commit], SMALL).await;

  if !parents.ok() {
    return None;
  }

  let listed = parents.text();
  let common = ["--no-color", "--no-ext-diff", "--no-textconv", "--no-renames"];
  let mut args: Vec<&str> = match listed.split_whitespace().nth(1) {
    Some(parent) => [&["diff"][..], &common, &["--end-of-options", parent, commit]].concat(),
    None => [
      &["diff-tree", "-p", "-r", "--root", "--no-commit-id"][..],
      &common,
      &["--end-of-options", commit],
    ]
    .concat(),
  };

  args.push("--");
  args.extend(described.files.iter().map(String::as_str));

  let run = git(root, &args, limits.diff_bytes).await;

  if !run.ok() {
    return None;
  }

  described.diff = Some(text_of(&run, run.cut));
  described.cut = run.cut;

  Some(described)
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

/// What a search found, and whether a ceiling cut it.
#[derive(Clone, Debug)]
pub struct Found<T> {
  pub items: Vec<T>,
  pub cut: bool,
}

/// The lines of one revision that hold the words, as written and ignoring case. Binary files are passed over, a
/// secret's lines never come back, and the search stops at its ceilings. git's grep takes no
/// end-of-options marker: the words go in as the value of its pattern option.
pub async fn search_tree(root: &Path, commit: &str, query: &str, limits: &Limits) -> Found<Match> {
  let most = format!("--max-count={}", limits.search_per_file);
  let args = ["grep", "-I", "-n", "-i", "-F", "-z", "--no-color", &most, "-e", query, commit, "--"];
  let run = git(root, &args, SEARCH_CAP).await;
  let mut items: Vec<Match> = matches_of(&run.text(), commit, limits)
    .into_iter()
    .filter(|found| !is_secret_path(&found.path))
    .collect();
  let cut = run.cut || run.code > 1 || items.len() > limits.search_total;

  items.truncate(limits.search_total);

  Found { items, cut }
}

/// Where a name is declared at one revision: every family's shapes run at once, then each line kept only where
/// its file's own family owns a shape that fits it, at most so many to a file, ordered and capped. git is asked
/// for more of a file than the answer keeps, so the lines a file's own family drops never hide the declarations
/// under them.
pub async fn find_definitions(
  root: &Path,
  commit: &str,
  reader: &str,
  name: &str,
  limits: &Limits,
) -> Found<Definition> {
  let most = format!("--max-count={}", limits.definition_scan_per_file);
  let pattern = definitions::definition_pattern(name);
  let args = ["grep", "-I", "-n", "-E", "-z", "--no-color", &most, "-e", &pattern, commit, "--"];
  let run: Run = git(root, &args, SEARCH_CAP).await;
  let mut taken: HashMap<String, usize> = HashMap::new();
  let mut items = Vec::new();

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
      items.push(Definition {
        path: found_line.path,
        line: found_line.line,
        text: found_line.text,
        kind,
      });
    }
  }

  definitions::rank(&mut items, reader);

  let cut = run.cut || run.code > 1 || items.len() > limits.definition_total;

  items.truncate(limits.definition_total);

  Found { items, cut }
}
