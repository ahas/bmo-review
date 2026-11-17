//! The projects (`docs/code-review.md`, 「Projects」): the repositories agency keeps on this machine, read through
//! its client on each request, and what a request names in one looked up rather than trusted: a project's name
//! becomes its checkout, and a revision a full hash of that checkout's own.

use std::path::PathBuf;

use bmo_agency_client::{ClientFailure, Repository};

use super::workspace::{head_of, resolve_commit};
use crate::http::{Refused, Standing};
use crate::wire::ProjectState;

/// One project's checkout.
#[derive(Clone, Debug)]
pub struct Checkout {
  /// The name agency keeps the repository under.
  pub name: String,
  /// The absolute path of the checkout.
  pub root: PathBuf,
}

/// Agency's failure as this service answers it.
fn upstream(failure: &ClientFailure) -> Refused {
  Refused::upstream(format!("agency failed: {failure}"))
}

/// A repository as a project, or none for one that stands on another machine, whose checkout is not read here.
fn checkout_of(repository: Repository) -> Option<Checkout> {
  repository.node_name.is_none().then(|| Checkout {
    name: repository.name,
    root: PathBuf::from(repository.path),
  })
}

/// A page of the projects in the order of their names, past the one `after` names, and whether more stand.
pub async fn page(
  standing: &Standing,
  after: Option<&str>,
  limit: usize,
) -> Result<(Vec<Checkout>, bool), Refused> {
  let listed = standing.agency.list_repositories().await.map_err(|failure| upstream(&failure))?;
  let mut checkouts: Vec<Checkout> = listed
    .items
    .into_iter()
    .filter_map(checkout_of)
    .filter(|checkout| after.is_none_or(|after| checkout.name.as_str() > after))
    .collect();

  checkouts.sort_by(|one, other| one.name.cmp(&other.name));
  let more = checkouts.len() > limit;
  checkouts.truncate(limit);

  Ok((checkouts, more))
}

/// The checkout of the project a request names, or none where agency keeps no such repository on this machine.
pub async fn find(standing: &Standing, name: &str) -> Result<Option<Checkout>, Refused> {
  match standing.agency.get_repository(name).await {
    Ok(repository) => Ok(checkout_of(repository)),
    Err(ClientFailure::Answered { status: 404, .. }) => Ok(None),
    Err(failure) => Err(upstream(&failure)),
  }
}

/// The checkout of the project a route's path names.
pub async fn require(standing: &Standing, name: &str) -> Result<Checkout, Refused> {
  find(standing, name)
    .await?
    .ok_or_else(|| Refused::not_found(format!("agency keeps no project {name}")))
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
