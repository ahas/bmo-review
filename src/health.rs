//! Whether the service is up (`docs/api.md`, 「Health」): the route, and the command that asks it.

use axum::http::{Method, StatusCode};
use axum::response::Response;
use schemars::generate::SchemaGenerator;

use crate::client::ReviewClient;
use crate::http::{Call, Refused, Route, answer};
use crate::openapi::{Stated, json_of};
use crate::settings::Settings;
use crate::wire::Health;

/// Readiness and the version. A service that answers has its store: it opens it before it answers anything.
pub struct Ready;

impl Route for Ready {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/health";

  async fn answer(&self, _call: Call) -> Result<Response, Refused> {
    let health = Health {
      ready: true,
      reason: None,
      version: env!("CARGO_PKG_VERSION").to_owned(),
    };

    Ok(answer(StatusCode::OK, &health))
  }

  fn stated(&self, generator: &mut SchemaGenerator) -> Stated {
    Stated {
      id: "readHealth",
      tag: "health",
      summary: "Readiness and version.",
      parameters: Vec::new(),
      body: None,
      answer: (200, json_of::<Health>(generator)),
      codes: Vec::new(),
    }
  }
}

crate::route!(Ready);

/// Whether the service the settings name answers ready, asked once through the service's own client.
pub async fn ask(settings: &Settings) -> Result<(), String> {
  let client = ReviewClient::new(&format!("http://{}", settings.server.listen), &settings.server.token)
    .map_err(|why| why.to_string())?;
  let health = client.health().await.map_err(|why| why.to_string())?;

  if health.ready {
    Ok(())
  } else {
    Err(format!("the service is not ready: {}", health.reason.unwrap_or_default()))
  }
}
