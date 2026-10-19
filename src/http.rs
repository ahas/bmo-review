//! Routes, the calls they answer, their refusals, and the router the service stands them on
//! (`docs/api.md`, 「Conventions」).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::Router;
use axum::extract::{FromRequestParts, Query, RawPathParams, Request, State};
use axum::http::{Method, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{MethodFilter, MethodRouter};
use serde_json::{Map, Value, json};

use crate::events::Events;
use crate::settings::Settings;
use crate::store::Store;

pub use inventory;

/// One route the service answers, registered beside its answer with `bmo_review::route!`.
pub trait Route: Send + Sync + 'static {
  const METHOD: Method;
  /// The path, in the router's own syntax: `/api/code/reviews/{id}`.
  const PATH: &'static str;

  fn answer(&self, call: Call) -> impl Future<Output = Result<Response, Refused>> + Send;
}

type Answering<'a> = Pin<Box<dyn Future<Output = Result<Response, Refused>> + Send + 'a>>;

/// A route as the router holds it, whatever its type.
pub trait Answers: Send + Sync {
  fn method(&self) -> Method;
  fn path(&self) -> &'static str;
  fn answer(&self, call: Call) -> Answering<'_>;
}

impl<R: Route> Answers for R {
  fn method(&self) -> Method {
    R::METHOD
  }

  fn path(&self) -> &'static str {
    R::PATH
  }

  fn answer(&self, call: Call) -> Answering<'_> {
    Box::pin(Route::answer(self, call))
  }
}

/// One registered route.
pub struct Registered(pub &'static dyn Answers);

inventory::collect!(Registered);

/// Registers a route with the service: `bmo_review::route!(Health);`
#[macro_export]
macro_rules! route {
  ($route:expr) => {
    $crate::http::inventory::submit! { $crate::http::Registered(&$route) }
  };
}

/// Every route linked into this binary.
pub fn routes() -> impl Iterator<Item = &'static dyn Answers> {
  inventory::iter::<Registered>.into_iter().map(|entry| entry.0)
}

/// A route's name as a caller writes it: its method and its path.
pub fn route_name(method: &Method, path: &str) -> String {
  format!("{method} {path}")
}

/// What is wrong with the routes linked in: two answers for one method and path would leave which one
/// answers to the order they were linked, so the service refuses to stand on any.
pub fn route_faults() -> Vec<String> {
  let mut seen = BTreeSet::new();

  routes()
    .map(|route| route_name(&route.method(), route.path()))
    .filter(|name| !seen.insert(name.clone()))
    .map(|name| format!("{name}: registered twice"))
    .collect()
}

/// The code of a failure a route did not expect.
pub const FAILED: &str = "failed";
/// The code of a body that is not JSON of the route's shape.
pub const BAD_REQUEST: &str = "bad_request";
/// The code of a missing or wrong token.
pub const UNAUTHORIZED: &str = "unauthorized";
/// The code of a path no route answers.
pub const NO_ROUTE: &str = "no_route";

/// A refusal: a status, and a body carrying an English sentence under `error` and a snake_case code under
/// `code` (`docs/api.md`, 「Refusals」).
#[derive(Debug, PartialEq, Eq)]
pub struct Refused {
  pub status: StatusCode,
  /// The snake_case code a caller matches on and a screen words.
  pub code: &'static str,
  /// The English sentence, for a log.
  pub error: String,
  /// What else the body carries beside them, under the names the route gives it.
  pub more: Map<String, Value>,
}

impl Refused {
  /// A refusal under the status and the code the route gives it.
  pub fn new(status: StatusCode, code: &'static str, error: impl Into<String>) -> Self {
    Self {
      status,
      code,
      error: error.into(),
      more: Map::new(),
    }
  }

  /// What a route answers for a failure it did not expect: 500, with the failure's own words.
  pub fn failed(error: impl std::fmt::Display) -> Self {
    Self::new(StatusCode::INTERNAL_SERVER_ERROR, FAILED, error.to_string())
  }

  /// The refusal with one more field in its body; `error` and `code` stay its own.
  pub fn with(mut self, name: impl Into<String>, value: impl serde::Serialize) -> Self {
    self.more.insert(name.into(), json!(value));
    self
  }
}

impl IntoResponse for Refused {
  fn into_response(self) -> Response {
    let mut body = self.more;

    body.insert("error".into(), json!(self.error));
    body.insert("code".into(), json!(self.code));

    (self.status, Json(Value::Object(body))).into_response()
  }
}

/// What the service stands on, which every call carries.
#[derive(Clone, Debug)]
pub struct Standing {
  pub settings: Arc<Settings>,
  /// The reviews' store, open and migrated.
  pub store: Store,
  /// The event stream every reader of `GET /api/code/events` hears.
  pub events: Events,
}

impl Standing {
  pub fn new(settings: Settings, store: Store) -> Self {
    Self {
      settings: Arc::new(settings),
      store,
      events: Events::new(),
    }
  }
}

/// One request a route answers.
pub struct Call {
  pub request: Request,
  standing: Arc<Standing>,
}

// The most a JSON body read whole may carry: 1 MiB, far beyond any route's fields.
const BODY_LIMIT: usize = 1024 * 1024;

impl Call {
  pub fn new(request: Request, standing: Arc<Standing>) -> Self {
    Self { request, standing }
  }

  /// What the service stands on.
  pub fn standing(&self) -> &Standing {
    &self.standing
  }

  /// The parameters the route's path names, by name.
  pub async fn path_params(&mut self) -> Result<HashMap<String, String>, Refused> {
    let (mut parts, body) = std::mem::take(&mut self.request).into_parts();
    let read = RawPathParams::from_request_parts(&mut parts, &()).await;

    self.request = Request::from_parts(parts, body);

    let params = read.map_err(Refused::failed)?;

    Ok(params.iter().map(|(name, value)| (name.to_owned(), value.to_owned())).collect())
  }

  /// One header's value, where it is given and reads as text.
  pub fn header(&self, name: impl header::AsHeaderName) -> Option<String> {
    self
      .request
      .headers()
      .get(name)
      .and_then(|value| value.to_str().ok())
      .map(str::to_owned)
  }

  /// One query parameter, the first where it is given more than once.
  pub fn query(&self, name: &str) -> Option<String> {
    let Query(pairs) = Query::<Vec<(String, String)>>::try_from_uri(self.request.uri()).ok()?;

    pairs.into_iter().find(|(key, _)| key == name).map(|(_, value)| value)
  }

  /// The body read as JSON, whatever its shape; a body that is not JSON is refused. A route reads it once.
  pub async fn body(&mut self) -> Result<Value, Refused> {
    let body = std::mem::take(self.request.body_mut());
    let bytes = axum::body::to_bytes(body, BODY_LIMIT)
      .await
      .map_err(|why| Refused::new(StatusCode::BAD_REQUEST, BAD_REQUEST, why.to_string()))?;

    serde_json::from_slice(&bytes)
      .map_err(|_| Refused::new(StatusCode::BAD_REQUEST, BAD_REQUEST, "the body is not JSON"))
  }
}

/// A number that is whole, however it was written, as JavaScript's `Number.isInteger` reads one.
pub fn whole(value: &Value) -> Option<i64> {
  value
    .as_i64()
    .or_else(|| value.as_f64().filter(|number| number.fract() == 0.0).map(|number| number as i64))
}

/// An answer: a JSON body under a status.
pub fn answer(status: StatusCode, body: Value) -> Response {
  (status, Json(body)).into_response()
}

/// Every request is let in only with the settings' token as its bearer; an empty token lets nobody in.
async fn require_token(State(standing): State<Arc<Standing>>, request: Request, next: Next) -> Response {
  let expected = standing.settings.server.token.as_str();
  let presented = request
    .headers()
    .get(header::AUTHORIZATION)
    .and_then(|value| value.to_str().ok())
    .and_then(|value| value.strip_prefix("Bearer "));

  match presented {
    Some(token) if !expected.is_empty() && token == expected => next.run(request).await,
    _ => Refused::new(StatusCode::UNAUTHORIZED, UNAUTHORIZED, "a missing or wrong token").into_response(),
  }
}

async fn no_route(request: Request) -> Response {
  Refused::new(
    StatusCode::NOT_FOUND,
    NO_ROUTE,
    format!("no route answers {}", route_name(request.method(), request.uri().path())),
  )
  .into_response()
}

/// The router answering every registered route behind the token, or what is wrong with the routes linked in.
pub fn router(standing: Standing) -> Result<Router, Vec<String>> {
  let faults = route_faults();

  if !faults.is_empty() {
    return Err(faults);
  }

  let mut by_path: BTreeMap<&'static str, MethodRouter<Arc<Standing>>> = BTreeMap::new();
  let mut faults = Vec::new();

  for route in routes() {
    let Ok(filter) = MethodFilter::try_from(route.method()) else {
      faults.push(format!("{}: the method cannot be routed", route_name(&route.method(), route.path())));
      continue;
    };
    let handler = move |State(standing): State<Arc<Standing>>, request: Request| async move {
      route
        .answer(Call::new(request, standing))
        .await
        .unwrap_or_else(IntoResponse::into_response)
    };
    let methods = by_path.remove(route.path()).unwrap_or_default();

    by_path.insert(route.path(), methods.on(filter, handler));
  }

  if !faults.is_empty() {
    return Err(faults);
  }

  let standing = Arc::new(standing);
  let router = by_path
    .into_iter()
    .fold(Router::new(), |router, (path, methods)| router.route(path, methods))
    .fallback(no_route)
    .layer(middleware::from_fn_with_state(standing.clone(), require_token));

  Ok(router.with_state(standing))
}
