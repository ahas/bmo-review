//! The routes reading a checkout (`docs/api.md`, 「Reading a checkout」): the projects, a commit's tree, one file,
//! the commits that touched a path, a content search and a definition search.

use axum::http::{Method, StatusCode};
use axum::response::Response;
use futures_util::future::join_all;
use serde::Serialize;
use serde_json::json;

use super::definitions::definition_name;
use super::request::{checkouts, require_checkout, require_commit};
use super::workspace::{
  self, CheckoutState, Commit, find_definitions, is_workspace_path, list_commits, list_tree, read_commit,
  read_tree_file, search_tree,
};
use crate::http::{Call, Refused, Route, answer};

/// One project: a worker's checkout and where it stands.
#[derive(Serialize)]
struct Project {
  name: String,
  label: String,
  #[serde(flatten)]
  standing: CheckoutState,
}

/// Every worker's checkout, with whether there is anything in it to read and where it stands. A worker whose
/// checkout cannot be read stays in the list with its state rather than vanishing.
pub struct Projects;

impl Route for Projects {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/api/code/projects";

  async fn answer(&self, call: Call) -> Result<Response, Refused> {
    let projects = join_all(checkouts(call.standing()).await?.into_iter().map(|checkout| async move {
      Project {
        standing: workspace::checkout_state(&checkout.root).await,
        name: checkout.name,
        label: checkout.label,
      }
    }))
    .await;

    Ok(answer(StatusCode::OK, json!(projects)))
  }
}

crate::route!(Projects);

/// One commit's tree as it may be shown.
pub struct Tree;

impl Route for Tree {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/api/code/tree";

  async fn answer(&self, call: Call) -> Result<Response, Refused> {
    let checkout = require_checkout(call.standing(), call.query("project").as_deref()).await?;
    let commit = require_commit(&checkout, call.query("commit").as_deref()).await?;
    let described: Commit = read_commit(&checkout.root, &commit)
      .await
      .ok_or_else(|| Refused::new(StatusCode::NOT_FOUND, "no_such_commit", "the commit cannot be read"))?;
    let tree = list_tree(&checkout.root, &commit, &call.standing().settings.limits).await;

    Ok(answer(
      StatusCode::OK,
      json!({ "project": checkout.name, "commit": described, "entries": tree.entries, "cut": tree.cut }),
    ))
  }
}

crate::route!(Tree);

/// One file as it stood at one commit. A path the commit's tree does not hold as a file that may be shown is no
/// file at all here.
pub struct FileAt;

impl Route for FileAt {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/api/code/file";

  async fn answer(&self, call: Call) -> Result<Response, Refused> {
    let checkout = require_checkout(call.standing(), call.query("project").as_deref()).await?;
    let commit = require_commit(&checkout, call.query("commit").as_deref()).await?;
    let path = call.query("path").unwrap_or_default();
    let mut file = read_tree_file(&checkout.root, &commit, &path, &call.standing().settings.limits)
      .await
      .ok_or_else(|| {
      Refused::new(StatusCode::NOT_FOUND, "no_such_file", "no file of that commit that may be shown")
    })?;

    file.project = checkout.name;

    Ok(answer(StatusCode::OK, json!(file)))
  }
}

crate::route!(FileAt);

/// The commits the picker offers: those reachable from HEAD that touched a path, newest first.
pub struct Commits;

impl Route for Commits {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/api/code/commits";

  async fn answer(&self, call: Call) -> Result<Response, Refused> {
    let checkout = require_checkout(call.standing(), call.query("project").as_deref()).await?;
    let head = require_commit(&checkout, None).await?;
    let path = call.query("path").unwrap_or_default();
    let commits =
      list_commits(&checkout.root, &path, &call.standing().settings.limits)
        .await
        .ok_or_else(|| {
          Refused::new(StatusCode::NOT_FOUND, "no_such_path", "the path is not one that may be shown")
        })?;

    Ok(answer(
      StatusCode::OK,
      json!({ "project": checkout.name, "path": path, "head": head, "commits": commits }),
    ))
  }
}

crate::route!(Commits);

/// The lines of one commit that hold the words asked for.
pub struct Search;

impl Route for Search {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/api/code/search";

  async fn answer(&self, call: Call) -> Result<Response, Refused> {
    let limits = &call.standing().settings.limits;
    let checkout = require_checkout(call.standing(), call.query("project").as_deref()).await?;
    let commit = require_commit(&checkout, call.query("commit").as_deref()).await?;
    let query = call.query("q").unwrap_or_default();

    if bmo_markup::js::trim(&query).is_empty()
      || bmo_markup::js::units(&query) > limits.search_query
      || query.contains(['\0', '\n'])
    {
      return Err(
        Refused::new(
          StatusCode::BAD_REQUEST,
          "search_too_long",
          "the search words are empty, too long, or hold a NUL or a line break",
        )
        .with("limit", limits.search_query),
      );
    }

    let found = search_tree(&checkout.root, &commit, &query, limits).await;

    Ok(answer(
      StatusCode::OK,
      json!({ "project": checkout.name, "commit": commit, "matches": found.matches, "cut": found.cut }),
    ))
  }
}

crate::route!(Search);

/// Where a name is declared, at one commit. The reader's own file comes with the press: it orders the answer and
/// says which words are the language's own rather than a name. A name nothing declares is an empty list.
pub struct Definitions;

impl Route for Definitions {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/api/code/definition";

  async fn answer(&self, call: Call) -> Result<Response, Refused> {
    let checkout = require_checkout(call.standing(), call.query("project").as_deref()).await?;
    let commit = require_commit(&checkout, call.query("commit").as_deref()).await?;
    let path = call.query("path").unwrap_or_default();

    if !is_workspace_path(&path) || path.is_empty() {
      return Err(Refused::new(
        StatusCode::BAD_REQUEST,
        "definition_no_file",
        "a definition search names no reader's file",
      ));
    }

    let name = definition_name(&call.query("name").unwrap_or_default(), &path).ok_or_else(|| {
      Refused::new(
        StatusCode::BAD_REQUEST,
        "definition_word",
        "the word is not one the definition search takes",
      )
    })?;
    let definitions =
      find_definitions(&checkout.root, &commit, &path, &name, &call.standing().settings.limits).await;

    Ok(answer(
      StatusCode::OK,
      json!({ "project": checkout.name, "commit": commit, "path": path, "name": name, "definitions": definitions }),
    ))
  }
}

crate::route!(Definitions);
