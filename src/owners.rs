//! Whose projects a reader sees (`docs/api.md`, 「The console face」, 「Whose projects a reader sees」): a call at a
//! route's own path and an administrator's call through the console see every project; a member's call through
//! the console sees the projects whose repository the agency records as owned by the member's own account.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::Request;
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::Response;
use bmo_agency_client::AgencyClient;
use bmo_console_face::{OWNER_KEY_HEADER, OWNER_ROLE_HEADER};
use tokio::sync::Mutex;
use tokio::time::Instant;

use crate::http::Refused;
use crate::settings::Agency;

/// How long what the agency answered is kept for the next member's call: a screen's burst of calls asks once,
/// and a change of an owner reaches a read this long after it at most.
const KEPT_FOR: Duration = Duration::from_secs(2);
/// How long a call to the agency is waited on before it counts as not answered.
const ANSWER_WITHIN: Duration = Duration::from_secs(5);
/// The role the console names an administrator by.
const ADMINISTRATOR: &str = "administrator";

/// Who a call reads for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reader {
  /// A caller with the service token, or an administrator through the console: every project.
  Everyone,
  /// A member through the console, by the account the console names, none where it names none or names one that
  /// is no account's id.
  Member(Option<i64>),
}

/// The projects a reader sees.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Seen {
  /// Every project the settings name.
  All,
  /// Only those of these names.
  Only(BTreeSet<String>),
}

impl Seen {
  /// Whether the reader sees the project of `name`.
  pub fn holds(&self, name: &str) -> bool {
    match self {
      Self::All => true,
      Self::Only(names) => names.contains(name),
    }
  }

  /// The names a store's query narrows to, as a JSON array, or none where every project is seen.
  pub fn names_json(&self) -> Option<String> {
    match self {
      Self::All => None,
      Self::Only(names) => Some(serde_json::Value::from_iter(names.iter().cloned()).to_string()),
    }
  }
}

/// Marks a call through the console with the reader the console names: an administrator, or a member by their
/// owner key. A call at a route's own path carries no mark and reads for everyone.
pub async fn mark_reader(mut request: Request, next: Next) -> Response {
  let reader = reader_of(request.headers());

  request.extensions_mut().insert(reader);
  next.run(request).await
}

// The reader the console's headers name.
fn reader_of(headers: &HeaderMap) -> Reader {
  let header = |name: &str| headers.get(name).and_then(|value| value.to_str().ok());

  if header(OWNER_ROLE_HEADER) == Some(ADMINISTRATOR) {
    Reader::Everyone
  } else {
    Reader::Member(header(OWNER_KEY_HEADER).and_then(|key| key.trim().parse().ok()))
  }
}

// Each repository's name and the account that owns it, as the agency answered them, and when.
struct Answered {
  at: Instant,
  owners: Arc<HashMap<String, Option<i64>>>,
}

/// The agency asked whose the repositories are, with what it last answered.
pub struct Owners {
  agency: Option<AgencyClient>,
  kept: Mutex<Option<Answered>>,
}

impl Owners {
  /// The agency the settings name, none where they name no address or no readable token, which is said on
  /// standard error; or why the address does not read.
  pub fn new(settings: &Agency) -> Result<Self, String> {
    Ok(Self {
      agency: agency(settings)?,
      kept: Mutex::new(None),
    })
  }

  /// The projects `reader` sees. A member is asked of the agency; where no agency is named they see none, and
  /// where the agency refuses or does not answer the call is refused `upstream-failed`.
  pub async fn seen(&self, reader: &Reader) -> Result<Seen, Refused> {
    let account = match reader {
      Reader::Everyone => return Ok(Seen::All),
      Reader::Member(None) => return Ok(Seen::Only(BTreeSet::new())),
      Reader::Member(Some(account)) => *account,
    };
    let Some(agency) = &self.agency else {
      return Ok(Seen::Only(BTreeSet::new()));
    };
    let owners = self.owners(agency).await?;

    Ok(Seen::Only(
      owners
        .iter()
        .filter(|(_, owner)| **owner == Some(account))
        .map(|(name, _)| name.clone())
        .collect(),
    ))
  }

  // Each repository's owning account, as the agency answered it at most `KEPT_FOR` ago. The lock is held while the
  // agency is asked, so calls arriving together ask once.
  async fn owners(&self, agency: &AgencyClient) -> Result<Arc<HashMap<String, Option<i64>>>, Refused> {
    let mut kept = self.kept.lock().await;

    if let Some(answered) = kept.as_ref().filter(|answered| answered.at.elapsed() < KEPT_FOR) {
      return Ok(answered.owners.clone());
    }

    let listed = agency.list_repositories().await.map_err(|failure| {
      Refused::upstream(format!("the agency could not be asked whose the repositories are: {failure}"))
    })?;
    let owners: Arc<HashMap<String, Option<i64>>> = Arc::new(
      listed
        .items
        .into_iter()
        .map(|repository| (repository.name, repository.owner_account))
        .collect(),
    );

    *kept = Some(Answered {
      at: Instant::now(),
      owners: owners.clone(),
    });

    Ok(owners)
  }
}

// The agency's client, where the settings name an address and a token that reads.
fn agency(settings: &Agency) -> Result<Option<AgencyClient>, String> {
  if settings.address.trim().is_empty() || settings.token_file.as_os_str().is_empty() {
    eprintln!("bmo-review: no agency is named, so a member through the console sees no project");

    return Ok(None);
  }

  let token = match std::fs::read_to_string(&settings.token_file) {
    Ok(token) if !token.trim().is_empty() => token.trim().to_owned(),
    Ok(_) => {
      eprintln!(
        "bmo-review: the agency's token file is empty, so a member through the console sees no project"
      );

      return Ok(None);
    }
    Err(why) => {
      eprintln!(
        "bmo-review: the agency's token file {} cannot be read ({why}), so a member through the console sees no \
         project",
        settings.token_file.display()
      );

      return Ok(None);
    }
  };
  let http = reqwest::Client::builder()
    .timeout(ANSWER_WITHIN)
    .build()
    .map_err(|why| format!("the agency's HTTP client cannot be made: {why}"))?;
  let client = AgencyClient::new(settings.address.trim(), token)
    .map_err(|why| format!("the agency's address does not read: {why}"))?;

  Ok(Some(client.with_http(http)))
}
