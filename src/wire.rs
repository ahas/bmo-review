//! Every body the service takes and answers (`docs/api.md`): the routes answer in these types, the client reads
//! them, the MCP server's tools take and give them, and the OpenAPI document is written from them, so the four
//! never part.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use crate::review::definitions::Kind as DefinitionKind;

/// Every failure: a stable kebab-case code a caller decides on, and an English sentence it decides nothing on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "Error")]
pub struct WireError {
  pub code: String,
  pub error: String,
}

/// Whether the service can answer, and its version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Health {
  pub ready: bool,
  /// Why it is not ready; null while it is.
  pub reason: Option<String>,
  pub version: String,
}

/// One page of a list: its items, and whether anything stands past the last of them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "{T}Page")]
pub struct Page<T> {
  pub items: Vec<T>,
  pub more: bool,
}

/// One event of the service.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Event {
  /// The event number, which only grows.
  pub id: i64,
  /// `review.created`, `review.changed`, `comment.created` or `filing.created`.
  pub kind: String,
  /// When it happened, in epoch milliseconds.
  pub at: i64,
  /// The member as a GET answers it.
  pub data: Value,
}

/// A reading of the event stream, or a page of it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EventQuery {
  /// A comma-separated list of the kinds wanted.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub kinds: Option<String>,
  /// The number of the last event read; the events start past it.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub after: Option<i64>,
  /// 1 to 1,000; 100 where absent. For a page only.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub limit: Option<u32>,
  /// The events of one review: its own, and those of its comments and filings.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub review_id: Option<i64>,
}

/// A list of projects: its cursor and size.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectQuery {
  /// The name of the last project read; the page starts past it.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub after: Option<String>,
  /// 1 to 1,000; 100 where absent.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub limit: Option<u32>,
}

/// Whether a project's checkout has anything to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectState {
  /// A git repository with at least one commit.
  Ready,
  /// The directory does not exist.
  Missing,
  /// The directory is in no git repository.
  NotGit,
  /// A repository with no commits.
  NoCommits,
}

/// One project: a repository agency keeps on this machine, and where it stands.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Project {
  /// The name agency keeps the repository under.
  pub name: String,
  /// The absolute path of the checkout.
  pub checkout: String,
  pub state: ProjectState,
  /// HEAD's full hash; null unless the project is ready.
  pub head: Option<String>,
  /// The branch HEAD names; null where it names none or the project is not ready.
  pub branch: Option<String>,
  /// The tracked files' uncommitted changes.
  pub uncommitted_count: u64,
}

/// A page of a revision's tree.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TreeQuery {
  /// The revision, seven to sixty-four hex characters; HEAD where absent.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub revision: Option<String>,
  /// The folder whose entries are listed, at any depth; the whole repository where absent.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub path: Option<String>,
  /// The path of the last entry read; the page starts past it.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub after: Option<String>,
  /// 1 to the tree ceiling; 1,000 where absent.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub limit: Option<u32>,
}

/// What a tree entry is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum EntryKind {
  File,
  Folder,
  Link,
  Submodule,
}

/// One entry of a revision's tree.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TreeEntry {
  pub path: String,
  pub kind: EntryKind,
  /// A file's bytes; null otherwise.
  pub byte_count: Option<u64>,
  /// The revision a submodule pins; null otherwise.
  pub pinned_revision: Option<String>,
}

/// One file at a revision.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct FileQuery {
  /// The revision, seven to sixty-four hex characters; HEAD where absent.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub revision: Option<String>,
}

/// One file as it stood at one revision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FileSource {
  pub path: String,
  /// The full hash.
  pub revision: String,
  pub byte_count: u64,
  /// The body; null for a binary file.
  pub text: Option<String>,
  pub line_count: u64,
  /// Whether the body ceiling cut the body.
  pub cut: bool,
}

/// A page of commits.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CommitQuery {
  /// The revision the list walks back from; HEAD where absent.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub to: Option<String>,
  /// An older revision the list stops short of, with its history.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub from: Option<String>,
  /// Only the commits that touched this path.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub path: Option<String>,
  /// The full hash of the last commit read; the page starts past it.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub after: Option<String>,
  /// 1 to 1,000; 100 where absent.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub limit: Option<u32>,
}

/// One commit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Commit {
  /// The full hash.
  pub id: String,
  pub short_id: String,
  pub subject: String,
  pub author_name: String,
  /// The commit time, in epoch milliseconds.
  pub at: i64,
  /// The paths it touched against its first parent, the excluded ones left out.
  pub files: Vec<String>,
  /// The patch against its first parent over its files, where the commit is read alone; null in a list.
  pub diff: Option<String>,
  /// Whether the diff ceiling cut the diff.
  pub cut: bool,
}

/// A page of a content search.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MatchQuery {
  /// The words, taken literally and ignoring case.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub q: Option<String>,
  /// The revision, seven to sixty-four hex characters; HEAD where absent.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub revision: Option<String>,
  /// How many matches were already read; the page starts past them.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub after: Option<u32>,
  /// 1 to 1,000; 100 where absent.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub limit: Option<u32>,
}

/// One line a content search found.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Match {
  pub path: String,
  pub line: u64,
  /// The line, cut to the match line ceiling.
  pub text: String,
}

/// A page of a definition search.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DefinitionQuery {
  /// The name looked for.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub name: Option<String>,
  /// The reader's file, which orders the answer and names the language whose keywords are no name.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub path: Option<String>,
  /// The revision, seven to sixty-four hex characters; HEAD where absent.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub revision: Option<String>,
  /// How many definitions were already read; the page starts past them.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub after: Option<u32>,
  /// 1 to 1,000; 100 where absent.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub limit: Option<u32>,
}

/// One place a name is declared.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Definition {
  pub name: String,
  pub path: String,
  pub line: u64,
  pub text: String,
  pub kind: DefinitionKind,
}

/// Where a review stands in its life (`docs/code-review.md`, 「Its states」).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ReviewState {
  /// Just opened, or its last comment is its author's.
  Open,
  /// Its last comment is someone else's.
  Answered,
  /// Closed.
  Closed,
}

impl ReviewState {
  /// The state as the wire and the store write it.
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Open => "open",
      Self::Answered => "answered",
      Self::Closed => "closed",
    }
  }

  /// The state a word names, or none.
  pub fn parse(word: &str) -> Option<Self> {
    match word {
      "open" => Some(Self::Open),
      "answered" => Some(Self::Answered),
      "closed" => Some(Self::Closed),
      _ => None,
    }
  }
}

/// One review.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Review {
  pub id: i64,
  pub project_name: String,
  /// The full hash of the revision it is about.
  pub revision: String,
  pub title: String,
  pub state: ReviewState,
  pub author_name: String,
  pub comment_count: u64,
  /// The job service's items filed from it, oldest first.
  pub item_ids: Vec<i64>,
  /// Epoch milliseconds.
  pub created_at: i64,
  /// Epoch milliseconds.
  pub changed_at: i64,
}

/// A page of reviews.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReviewQuery {
  /// The reviews of one project.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub project_name: Option<String>,
  /// The reviews of one revision, by its full hash.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub revision: Option<String>,
  /// The reviews in one state.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub state: Option<ReviewState>,
  /// The id of the last review read; the page continues with older ones.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub after: Option<i64>,
  /// 1 to 1,000; 100 where absent.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub limit: Option<u32>,
}

/// A review to open.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewReview {
  pub project_name: String,
  /// Seven to sixty-four hex characters; HEAD where empty.
  pub revision: String,
  pub title: String,
  pub author_name: String,
}

/// A change of a review's state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewChange {
  /// `closed` to close it, `open` to reopen it.
  pub state: ReviewState,
}

/// A page of a review's comments.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CommentQuery {
  /// The id of the last comment read; the page continues with newer ones.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub after: Option<i64>,
  /// 1 to 1,000; 100 where absent.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub limit: Option<u32>,
}

/// One comment on a review.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
  pub id: i64,
  pub review_id: i64,
  /// The file or folder it is about; null for the whole review.
  pub path: Option<String>,
  /// The first line it points at; null where it points at none.
  pub line: Option<u32>,
  /// The last line of a range; null for one line or none.
  pub end_line: Option<u32>,
  /// The lines it points at, copied from the review's revision; null where it points at none.
  pub excerpt: Option<String>,
  pub text: String,
  pub author_name: String,
  /// Epoch milliseconds.
  pub created_at: i64,
}

/// A comment to write.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewComment {
  /// A file or folder of the review's revision; the whole review where absent or null.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub path: Option<String>,
  /// The first line it points at, from 1, with a file's path.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub line: Option<u32>,
  /// The last line of a range, past `line`.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub end_line: Option<u32>,
  pub text: String,
  pub author_name: String,
}

/// An item to file from a review into the job service.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewFiling {
  pub title: String,
  pub brief: String,
  /// The job service's queue the item goes into.
  pub queue_name: String,
}

/// An item filed from a review.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Filing {
  pub id: i64,
  pub review_id: i64,
  /// The number the job service answered for the item.
  pub item_id: i64,
  /// Epoch milliseconds.
  pub created_at: i64,
}

/// One review and a page of its comments, as the `review_review` tool answers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ReviewThread {
  pub review: Review,
  pub comments: Page<Comment>,
}
