//! The projects (`docs/code-review.md`, 「Projects」): the checkouts the settings name, and what a request names
//! in one looked up rather than trusted: a project's name becomes its checkout, and a revision a full hash of
//! that checkout's own.

use std::path::PathBuf;

use super::workspace::{head_of, resolve_commit};
use crate::http::{Refused, Standing};
use crate::owners::Seen;
use crate::wire::ProjectState;

/// One project's checkout.
#[derive(Clone, Debug)]
pub struct Checkout {
  /// The name the settings give the project.
  pub name: String,
  /// The absolute path of the checkout.
  pub root: PathBuf,
}

/// Every project the settings name that the reader sees, in the order of their names.
fn named(standing: &Standing, seen: &Seen) -> Vec<Checkout> {
  let mut checkouts: Vec<Checkout> = standing
    .settings
    .projects
    .iter()
    .filter(|project| seen.holds(&project.name))
    .map(|project| Checkout {
      name: project.name.clone(),
      root: PathBuf::from(&project.checkout),
    })
    .collect();

  checkouts.sort_by(|one, other| one.name.cmp(&other.name));
  checkouts
}

/// A page of the projects the reader sees in the order of their names, past the one `after` names, and whether
/// more stand.
pub fn page(standing: &Standing, seen: &Seen, after: Option<&str>, limit: usize) -> (Vec<Checkout>, bool) {
  let mut checkouts: Vec<Checkout> = named(standing, seen)
    .into_iter()
    .filter(|checkout| after.is_none_or(|after| checkout.name.as_str() > after))
    .collect();
  let more = checkouts.len() > limit;

  checkouts.truncate(limit);
  (checkouts, more)
}

/// The checkout of the project a request names, or none where the settings name no such project or the reader
/// does not see it.
pub fn find(standing: &Standing, seen: &Seen, name: &str) -> Option<Checkout> {
  named(standing, seen).into_iter().find(|checkout| checkout.name == name)
}

/// The checkout of the project a route's path names, a project the reader does not see being none.
pub fn require(standing: &Standing, seen: &Seen, name: &str) -> Result<Checkout, Refused> {
  find(standing, seen, name).ok_or_else(|| Refused::not_found(format!("no project is named {name}")))
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
