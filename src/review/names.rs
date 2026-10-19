//! The names the screen's words are filled with (`docs/api.md`, 「Names」): the owner's and the agent's, as the
//! settings give them.

use axum::http::{Method, StatusCode};
use axum::response::Response;
use serde_json::json;

use crate::http::{Call, Refused, Route, answer};

/// The owner's and the agent's names.
pub struct Names;

impl Route for Names {
  const METHOD: Method = Method::GET;
  const PATH: &'static str = "/api/code/names";

  async fn answer(&self, call: Call) -> Result<Response, Refused> {
    let filing = &call.standing().settings.filing;

    Ok(answer(StatusCode::OK, json!({ "owner": filing.owner, "agent": filing.agent })))
  }
}

crate::route!(Names);
