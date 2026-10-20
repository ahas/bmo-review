//! The projects (`docs/code-review.md`, 「Projects」): the checkouts the graph service registers, read through
//! its client on each request, and what a request names in one looked up rather than trusted: a project's name
//! becomes its checkout, and a revision a full hash of that checkout's own.

use std::path::PathBuf;

use bmo_graph::client::{ClientFailure, GraphClient};
use bmo_graph::wire::ListQuery;

use super::workspace::{head_of, resolve_commit};
use crate::http::{Refused, Standing};
use crate::wire::ProjectState;

/// One project's checkout.
#[derive(Clone, Debug)]
pub struct Checkout {
  /// The name the graph service registers it under.
  pub name: String,
  /// The absolute path of the checkout.
  pub root: PathBuf,
}

/// One call of the graph service's client, which waits on its answer, run where waiting holds up no other
/// request's work.
async fn graph<T, F>(standing: &Standing, call: F) -> Result<T, ClientFailure>
where
  T: Send + 'static,
  F: FnOnce(&GraphClient) -> Result<T, ClientFailure> + Send + 'static,
{
  let client = standing.graph.clone();

  tokio::task::spawn_blocking(move || call(&client))
    .await
    .unwrap_or_else(|why| Err(ClientFailure::Unreached(why.to_string())))
}

/// The graph service's failure as this service answers it.
fn upstream(failure: &ClientFailure) -> Refused {
  Refused::upstream(format!("the graph service failed: {failure}"))
}

/// A page of the graph service's projects, each with its checkout, and whether more stand.
pub async fn page(standing: &Standing, query: ListQuery) -> Result<(Vec<Checkout>, bool), Refused> {
  let page = graph(standing, move |client| client.projects(&query))
    .await
    .map_err(|failure| upstream(&failure))?;
  let checkouts = page
    .items
    .into_iter()
    .map(|project| Checkout {
      name: project.name,
      root: PathBuf::from(project.checkout),
    })
    .collect();

  Ok((checkouts, page.more))
}

/// The checkout of the project a request names, or none where the graph service knows no such project.
pub async fn find(standing: &Standing, name: &str) -> Result<Option<Checkout>, Refused> {
  let asked = name.to_owned();

  match graph(standing, move |client| client.project(&asked)).await {
    Ok(project) => Ok(Some(Checkout {
      name: project.name,
      root: PathBuf::from(project.checkout),
    })),
    Err(ClientFailure::Answered { status: 404, .. }) => Ok(None),
    Err(failure) => Err(upstream(&failure)),
  }
}

/// The checkout of the project a route's path names.
pub async fn require(standing: &Standing, name: &str) -> Result<Checkout, Refused> {
  find(standing, name)
    .await?
    .ok_or_else(|| Refused::not_found(format!("the graph service knows no project {name}")))
}

/// Why a checkout with no HEAD to read cannot be read.
pub fn unreadable(name: &str, state: ProjectState) -> Refused {
  let why = match state {
    ProjectState::Missing => "its directory does not exist",
    ProjectState::NotGit => "its directory is in no git repository",
    ProjectState::NoCommits => "its repository has no commits",
    ProjectState::Ready => "it cannot be read",
  };

  Refused::unprocessable(
    "unreadable-checkout",
    format!("the checkout of project {name} cannot be read: {why}"),
  )
}

/// The revision a request reads at, as a full hash: the one it names, HEAD where it names none; none where the
/// one it names resolves to no commit of the checkout.
pub async fn revision(checkout: &Checkout, asked: Option<&str>) -> Result<Option<String>, Refused> {
  let head = head_of(&checkout.root).await.map_err(|state| unreadable(&checkout.name, state))?;

  match asked.map(str::trim).filter(|asked| !asked.is_empty()) {
    None => Ok(Some(head)),
    Some(asked) => Ok(resolve_commit(&checkout.root, asked).await),
  }
}

/// The revision a route reads at, a revision it names that resolves to nothing being no such resource.
pub async fn require_revision(checkout: &Checkout, asked: Option<&str>) -> Result<String, Refused> {
  revision(checkout, asked).await?.ok_or_else(|| {
    Refused::not_found(format!(
      "revision {} is no commit of project {}",
      asked.unwrap_or_default(),
      checkout.name
    ))
  })
}
