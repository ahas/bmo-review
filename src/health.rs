//! Whether the service is up (`docs/api.md`, 「Health」): the route, and the command that asks it.

use std::time::Duration;

use axum::http::{Method, StatusCode};
use axum::response::Response;
use serde::Deserialize;
use serde_json::json;

use crate::http::{Call, Refused, Route, answer};
use crate::settings::Settings;

/// The process, the version and whether the store is open and migrated. A service that answers has its
/// store: it opens it before it answers anything.
pub struct Health;

impl Route for Health {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/health";

  async fn answer(&self, _call: Call) -> Result<Response, Refused> {
    Ok(answer(
      StatusCode::OK,
      json!({ "pid": std::process::id(), "version": env!("CARGO_PKG_VERSION"), "ready": true }),
    ))
  }
}

crate::route!(Health);

// How long the command waits on the service.
const WAIT: Duration = Duration::from_secs(5);

#[derive(Deserialize)]
struct Answer {
  ready: bool,
}

/// Whether the service the settings name answers ready, asked once and waited on at most five seconds.
pub async fn ask(settings: &Settings) -> Result<(), String> {
  let address = format!("http://{}/health", settings.server.listen);
  let answer = reqwest::Client::new()
    .get(&address)
    .bearer_auth(&settings.server.token)
    .timeout(WAIT)
    .send()
    .await
    .map_err(|why| format!("the service at {} did not answer: {why}", settings.server.listen))?;
  let status = answer.status();

  if !status.is_success() {
    return Err(format!("the service answered {}", status.as_u16()));
  }

  let read: Answer = answer.json().await.map_err(|why| format!("the answer could not be read: {why}"))?;

  if read.ready { Ok(()) } else { Err("the service is not ready".to_owned()) }
}
