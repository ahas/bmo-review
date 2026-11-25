//! The routes reading a project (`docs/api.md`, 「Projects」): the projects, a revision's tree, one file, the
//! commits and one commit's diff, a content search and a definition search.

use axum::http::{Method, StatusCode};
use axum::response::Response;
use futures_util::future::join_all;
use schemars::generate::SchemaGenerator;

use super::definitions::definition_name;
use super::projects::{self, Checkout, require, require_revision};
use super::workspace::{
  self, Found, ListFailure, checkout_state, find_entry, is_workspace_path, list_commits, list_tree,
  read_commit_diff, read_tree_file, search_tree,
};
use crate::http::{Call, MOST_PAGE, PAGE, Refused, Route, answer, page_size};
use crate::openapi::{READING_CODES, Stated, commit_key, file_key, json_of, project_key, query};
use crate::wire::{
  Commit, CommitQuery, Definition, DefinitionQuery, EntryKind, FileQuery, FileSource, Match, MatchQuery,
  Page, Project, ProjectQuery, TreeEntry, TreeQuery,
};

/// The project a route's path names.
async fn project_of(call: &mut Call) -> Result<Checkout, Refused> {
  let name = call.path_param("projectName").await?;
  let seen = call.seen().await?;

  require(call.standing(), &seen, &name)
}

/// A page of a search's items, its cursor the count already read: whether more stand past the page, or the
/// search was cut short.
fn search_page<T>(found: Found<T>, after: Option<u32>, limit: Option<u32>) -> Result<Page<T>, Refused> {
  let limit = page_size(limit, PAGE, MOST_PAGE)?;
  let start = after.unwrap_or(0) as usize;
  let total = found.items.len();
  let items: Vec<T> = found.items.into_iter().skip(start).take(limit).collect();
  let more = found.cut || total > start + items.len();

  Ok(Page { items, more })
}

/// `GET /projects`: the projects the reader sees, each with whether there is anything in it to read and where it
/// stands. A project whose checkout cannot be read stays in the list with its state rather than vanishing.
pub struct Projects;

impl Route for Projects {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/projects";

  async fn answer(&self, call: Call) -> Result<Response, Refused> {
    let query: ProjectQuery = call.query()?;
    let limit = page_size(query.limit, PAGE, MOST_PAGE)?;
    let seen = call.seen().await?;
    let (checkouts, more) = projects::page(call.standing(), &seen, query.after.as_deref(), limit);
    let items = join_all(checkouts.into_iter().map(|checkout| async move {
      let standing = checkout_state(&checkout.root).await;

      Project {
        name: checkout.name,
        checkout: checkout.root.display().to_string(),
        state: standing.state,
        head: standing.head,
        branch: standing.branch,
        uncommitted_count: standing.uncommitted,
      }
    }))
    .await;

    Ok(answer(StatusCode::OK, &Page { items, more }))
  }

  fn stated(&self, generator: &mut SchemaGenerator) -> Stated {
    Stated {
      id: "listProjects",
      tag: "projects",
      summary: "The projects that can be read, in name order.",
      parameters: query::<ProjectQuery>(generator),
      body: None,
      answer: (200, json_of::<Page<Project>>(generator)),
      codes: vec![],
    }
  }
}

crate::route!(Projects);

/// `GET /projects/{projectName}/tree`: every entry under a folder of a revision that may be shown, in path order.
pub struct Tree;

impl Route for Tree {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/projects/{projectName}/tree";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let query: TreeQuery = call.query()?;
    let limits = &call.standing().settings.limits;
    let limit = page_size(query.limit, MOST_PAGE, limits.tree_entries)?;
    let checkout = project_of(&mut call).await?;
    let revision = require_revision(&checkout, query.revision.as_deref()).await?;
    let folder = query.path.unwrap_or_default();
    let found = find_entry(&checkout.root, &revision, &folder).await;

    if found.is_none_or(|entry| entry.kind != EntryKind::Folder) {
      return Err(Refused::not_found(format!(
        "{folder} is no folder of revision {revision} that may be shown"
      )));
    }

    let entries = list_tree(&checkout.root, &revision)
      .await
      .ok_or_else(|| Refused::failed(format!("the tree of revision {revision} could not be read")))?;
    let under = if folder.is_empty() { String::new() } else { format!("{folder}/") };
    let mut items: Vec<_> = entries
      .into_iter()
      .filter(|entry| entry.path.starts_with(&under))
      .filter(|entry| query.after.as_ref().is_none_or(|after| entry.path > *after))
      .take(limit + 1)
      .collect();
    let more = items.len() > limit;

    items.truncate(limit);

    Ok(answer(StatusCode::OK, &Page { items, more }))
  }

  fn stated(&self, generator: &mut SchemaGenerator) -> Stated {
    Stated {
      id: "listTreeEntries",
      tag: "projects",
      summary: "A project's tree at a revision, under a path.",
      parameters: [vec![project_key()], query::<TreeQuery>(generator)].concat(),
      body: None,
      answer: (200, json_of::<Page<TreeEntry>>(generator)),
      codes: READING_CODES.to_vec(),
    }
  }
}

crate::route!(Tree);

/// `GET /projects/{projectName}/files/{filePath}`: one file as it stood at a revision. A path the revision's tree
/// does not hold as a file that may be shown is no file at all here.
pub struct FileAt;

impl Route for FileAt {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/projects/{projectName}/files/{*filePath}";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let query: FileQuery = call.query()?;
    let path = call.path_param("filePath").await?;
    let checkout = project_of(&mut call).await?;
    let revision = require_revision(&checkout, query.revision.as_deref()).await?;
    let file = read_tree_file(&checkout.root, &revision, &path, &call.standing().settings.limits)
      .await
      .ok_or_else(|| {
        Refused::not_found(format!("{path} is no file of revision {revision} that may be shown"))
      })?;

    Ok(answer(StatusCode::OK, &file))
  }

  fn stated(&self, generator: &mut SchemaGenerator) -> Stated {
    Stated {
      id: "readFile",
      tag: "projects",
      summary: "One file at a revision.",
      parameters: [vec![project_key(), file_key()], query::<FileQuery>(generator)].concat(),
      body: None,
      answer: (200, json_of::<FileSource>(generator)),
      codes: READING_CODES.to_vec(),
    }
  }
}

crate::route!(FileAt);

/// `GET /projects/{projectName}/commits`: the commits back from a revision, newest first.
pub struct Commits;

impl Route for Commits {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/projects/{projectName}/commits";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let query: CommitQuery = call.query()?;
    let limit = page_size(query.limit, PAGE, MOST_PAGE)?;
    let checkout = project_of(&mut call).await?;
    let to = require_revision(&checkout, query.to.as_deref()).await?;
    let from = match query.from.as_deref().map(str::trim).filter(|from| !from.is_empty()) {
      None => None,
      Some(from) => Some(require_revision(&checkout, Some(from)).await?),
    };
    let path = query.path.unwrap_or_default();
    let (items, more) =
      list_commits(&checkout.root, &to, from.as_deref(), &path, query.after.as_deref(), limit)
        .await
        .map_err(|failure| match failure {
          ListFailure::Path => Refused::not_found(format!("{path} is not a path that may be shown")),
          ListFailure::Cursor => Refused::bad_request("after is no commit of this list"),
          ListFailure::Unreadable => Refused::failed("the history could not be read"),
        })?;

    Ok(answer(StatusCode::OK, &Page { items, more }))
  }

  fn stated(&self, generator: &mut SchemaGenerator) -> Stated {
    Stated {
      id: "listCommits",
      tag: "projects",
      summary: "Commits, newest first, from a revision or between two.",
      parameters: [vec![project_key()], query::<CommitQuery>(generator)].concat(),
      body: None,
      answer: (200, json_of::<Page<Commit>>(generator)),
      codes: READING_CODES.to_vec(),
    }
  }
}

crate::route!(Commits);

/// `GET /projects/{projectName}/commits/{commitId}`: one commit and its diff.
pub struct CommitDiff;

impl Route for CommitDiff {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/projects/{projectName}/commits/{commitId}";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let asked = call.path_param("commitId").await?;
    let checkout = project_of(&mut call).await?;
    let revision = require_revision(&checkout, Some(&asked)).await?;
    let commit = read_commit_diff(&checkout.root, &revision, &call.standing().settings.limits)
      .await
      .ok_or_else(|| Refused::failed(format!("commit {revision} could not be read")))?;

    Ok(answer(StatusCode::OK, &commit))
  }

  fn stated(&self, generator: &mut SchemaGenerator) -> Stated {
    Stated {
      id: "readCommit",
      tag: "projects",
      summary: "One commit and its diff.",
      parameters: vec![project_key(), commit_key()],
      body: None,
      answer: (200, json_of::<Commit>(generator)),
      codes: READING_CODES.to_vec(),
    }
  }
}

crate::route!(CommitDiff);

/// `GET /projects/{projectName}/matches`: the lines of a revision that hold the words asked for.
pub struct Matches;

impl Route for Matches {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/projects/{projectName}/matches";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let query: MatchQuery = call.query()?;
    let limits = call.standing().settings.limits.clone();
    let words = query.q.clone().unwrap_or_default();

    if bmo_text::js::trim(&words).is_empty()
      || bmo_text::js::units(&words) > limits.search_query
      || words.contains(['\0', '\n'])
    {
      return Err(Refused::bad_request(format!(
        "q is empty, past {} characters, or holds a NUL or a line break",
        limits.search_query
      )));
    }

    let checkout = project_of(&mut call).await?;
    let revision = require_revision(&checkout, query.revision.as_deref()).await?;
    let found = search_tree(&checkout.root, &revision, &words, &limits).await;

    Ok(answer(StatusCode::OK, &search_page(found, query.after, query.limit)?))
  }

  fn stated(&self, generator: &mut SchemaGenerator) -> Stated {
    Stated {
      id: "listMatches",
      tag: "projects",
      summary: "Lines matching words at a revision.",
      parameters: [vec![project_key()], query::<MatchQuery>(generator)].concat(),
      body: None,
      answer: (200, json_of::<Page<Match>>(generator)),
      codes: READING_CODES.to_vec(),
    }
  }
}

crate::route!(Matches);

/// `GET /projects/{projectName}/definitions`: where a name is declared, at a revision. The reader's file orders
/// the answer and says which words are the language's own rather than a name. A name nothing declares is an empty
/// page.
pub struct Definitions;

impl Route for Definitions {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/projects/{projectName}/definitions";

  async fn answer(&self, mut call: Call) -> Result<Response, Refused> {
    let query: DefinitionQuery = call.query()?;
    let reader = query.path.clone().unwrap_or_default();

    if !is_workspace_path(&reader) {
      return Err(Refused::bad_request(format!("{reader} is not a path a request may name")));
    }

    let asked = query.name.clone().ok_or_else(|| Refused::bad_request("name is required"))?;
    let name = definition_name(&asked, &reader).ok_or_else(|| {
      Refused::unprocessable("not-a-name", format!("{asked} is not a name the definition search takes"))
    })?;
    let checkout = project_of(&mut call).await?;
    let revision = require_revision(&checkout, query.revision.as_deref()).await?;
    let found = workspace::find_definitions(
      &checkout.root,
      &revision,
      &reader,
      &name,
      &call.standing().settings.limits,
    )
    .await;
    let found = Found {
      items: found
        .items
        .into_iter()
        .map(|place| Definition {
          name: name.clone(),
          path: place.path,
          line: place.line,
          text: place.text,
          kind: place.kind,
        })
        .collect(),
      cut: found.cut,
    };

    Ok(answer(StatusCode::OK, &search_page(found, query.after, query.limit)?))
  }

  fn stated(&self, generator: &mut SchemaGenerator) -> Stated {
    Stated {
      id: "listDefinitions",
      tag: "projects",
      summary: "Where a name is defined at a revision.",
      parameters: [vec![project_key()], query::<DefinitionQuery>(generator)].concat(),
      body: None,
      answer: (200, json_of::<Page<Definition>>(generator)),
      codes: [READING_CODES, &["not-a-name"]].concat(),
    }
  }
}

crate::route!(Definitions);
