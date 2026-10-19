//! What a request names, looked up rather than trusted (`docs/code-review.md`, 「What a request may give」): a
//! worker's name becomes a checkout of the catalogue and a commit a full hash of that checkout's own.

use axum::http::StatusCode;

use super::checkouts::{Checkout, checkouts_of};
use super::workspace::{State, checkout_state, resolve_commit};
use crate::http::{Refused, Standing};

/// The checkouts of the catalogue as bmo-job lists its workers now.
pub async fn checkouts(standing: &Standing) -> Result<Vec<Checkout>, Refused> {
  Ok(checkouts_of(&standing.job.workers().await?))
}

/// The checkout of the worker a request names.
pub async fn require_checkout(standing: &Standing, name: Option<&str>) -> Result<Checkout, Refused> {
  let name = name.map(str::trim).unwrap_or_default();

  checkouts(standing)
    .await?
    .into_iter()
    .find(|checkout| checkout.name == name)
    .ok_or_else(|| {
      Refused::new(
        StatusCode::NOT_FOUND,
        "no_such_worker",
        "the project is no worker of the catalogue naming a directory",
      )
    })
}

/// The commit a request reads at: the one it names, or HEAD where it names none.
pub async fn require_commit(checkout: &Checkout, asked: Option<&str>) -> Result<String, Refused> {
  match asked.filter(|asked| !asked.is_empty()) {
    None => {
      let standing = checkout_state(&checkout.root).await;

      standing.head.map(|head| head.commit).ok_or_else(|| unreadable(standing.state))
    }
    Some(asked) => resolve_commit(&checkout.root, asked).await.ok_or_else(|| {
      Refused::new(StatusCode::NOT_FOUND, "no_such_commit", "no commit of that repository by that hash")
    }),
  }
}

/// Why a checkout with no HEAD to read cannot be read.
fn unreadable(state: State) -> Refused {
  let (code, error) = match state {
    State::Missing => ("no_path", "the worker's directory does not exist"),
    State::NotGit => ("not_git", "the worker's directory is not a git repository"),
    State::NoCommits => ("no_commits", "the repository has no commits"),
    State::Ready => ("unreadable_checkout", "the checkout cannot be read"),
  };

  Refused::new(StatusCode::CONFLICT, code, error)
}
