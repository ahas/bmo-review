//! bmo-job, as its `docs/api.md` 「Workers」 states it: the worker catalogue this service reads its checkouts from.

use axum::http::StatusCode;
use reqwest::{Client, Url};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::http::Refused;

/// The code of a call on bmo-job that failed.
pub const JOB_FAILED: &str = "job_failed";
// The code bmo-job refuses a name no worker has with.
const NO_WORKER: &str = "no_worker";

/// A worker as bmo-job keeps it, the fields this service reads.
#[derive(Clone, Debug, Deserialize)]
pub struct Worker {
  pub name: String,
  pub label: Option<String>,
  /// The worker's working directory.
  pub cwd: Option<String>,
  /// The base worker a support worker stands beside.
  pub support_of: Option<String>,
}

#[derive(Deserialize)]
struct Refusal {
  code: String,
}

/// bmo-job, at its base address with the token it takes.
#[derive(Clone, Debug)]
pub struct Job {
  client: Client,
  address: String,
  token: String,
}

impl Job {
  pub fn new(address: &str, token: &str) -> Self {
    Self {
      client: Client::new(),
      address: address.trim_end_matches('/').to_owned(),
      token: token.to_owned(),
    }
  }

  fn url(&self, segments: &[&str]) -> Result<Url, Refused> {
    let mut url = Url::parse(&self.address).map_err(|why| failed(format!("bmo-job's address: {why}")))?;

    url
      .path_segments_mut()
      .map_err(|()| failed("bmo-job's address takes no path"))?
      .pop_if_empty()
      .extend(segments);

    Ok(url)
  }

  async fn get<T: DeserializeOwned>(&self, segments: &[&str]) -> Result<Option<T>, Refused> {
    let mut asked = self.client.get(self.url(segments)?);

    if !self.token.is_empty() {
      asked = asked.bearer_auth(&self.token);
    }

    let answer = asked.send().await.map_err(|why| failed(format!("bmo-job did not answer: {why}")))?;
    let status = answer.status();

    if status.is_success() {
      return answer.json().await.map(Some).map_err(|why| failed(format!("bmo-job's answer: {why}")));
    }

    let code = answer.json::<Refusal>().await.map(|refusal| refusal.code).unwrap_or_default();

    if code == NO_WORKER {
      return Ok(None);
    }

    Err(failed(format!("bmo-job answered {} {code}", status.as_u16())))
  }

  /// `GET /workers`: every worker.
  pub async fn workers(&self) -> Result<Vec<Worker>, Refused> {
    Ok(self.get(&["workers"]).await?.unwrap_or_default())
  }

  /// `GET /workers/{name}`: one worker, none where bmo-job has no worker of that name.
  pub async fn worker(&self, name: &str) -> Result<Option<Worker>, Refused> {
    self.get(&["workers", name]).await
  }
}

fn failed(why: impl Into<String>) -> Refused {
  Refused::new(StatusCode::BAD_GATEWAY, JOB_FAILED, why)
}
