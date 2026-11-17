//! Routes, the calls they answer, their failures, and the router the service stands them on
//! (`docs/api.md`, 「Conventions」, 「Codes」).

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
use bmo_agency_client::AgencyClient;
use bmo_console_face::ConsoleFace;
use bmo_job_client::JobClient;
use schemars::generate::SchemaGenerator;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::console;
use crate::events::Events;
use crate::openapi::Stated;
use crate::settings::Settings;
use crate::store::Store;
use crate::wire::WireError;

pub use inventory;

/// One route the service answers, registered beside its answer with `bmo_review::route!`.
pub trait Route: Send + Sync + 'static {
  const METHOD: Method;
  /// The path, in the router's own syntax: `/reviews/{reviewId}`.
  const PATH: &'static str;

  fn answer(&self, call: Call) -> impl Future<Output = Result<Response, Refused>> + Send;

  /// How the OpenAPI document states the route, its schemas written where `generator` keeps them.
  fn stated(&self, generator: &mut SchemaGenerator) -> Stated;
}

type Answering<'a> = Pin<Box<dyn Future<Output = Result<Response, Refused>> + Send + 'a>>;

/// A route as the router holds it, whatever its type.
pub trait Answers: Send + Sync {
  fn method(&self) -> Method;
  fn path(&self) -> &'static str;
  fn answer(&self, call: Call) -> Answering<'_>;
  fn stated(&self, generator: &mut SchemaGenerator) -> Stated;
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

  fn stated(&self, generator: &mut SchemaGenerator) -> Stated {
    Route::stated(self, generator)
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

/// A failure: a status, and a body of two keys, a kebab-case `code` a caller matches on and an English sentence
/// under `error` (`docs/api.md`, 「Codes」).
#[derive(Debug, PartialEq, Eq)]
pub struct Refused {
  pub status: StatusCode,
  pub code: &'static str,
  /// The English sentence, for a log.
  pub error: String,
}

impl Refused {
  /// A failure under the status and the code the route gives it.
  pub fn new(status: StatusCode, code: &'static str, error: impl Into<String>) -> Self {
    Self {
      status,
      code,
      error: error.into(),
    }
  }

  /// 400 `bad-request`: the body, the query or the path is not the shape the route takes.
  pub fn bad_request(error: impl Into<String>) -> Self {
    Self::new(StatusCode::BAD_REQUEST, "bad-request", error)
  }

  /// 404 `not-found`: nothing stands at the key the request names.
  pub fn not_found(error: impl Into<String>) -> Self {
    Self::new(StatusCode::NOT_FOUND, "not-found", error)
  }

  /// 409 `conflict`: the member's state does not allow the change.
  pub fn conflict(error: impl Into<String>) -> Self {
    Self::new(StatusCode::CONFLICT, "conflict", error)
  }

  /// 422 under a code of the route's own: a rule of the service refuses the request.
  pub fn unprocessable(code: &'static str, error: impl Into<String>) -> Self {
    Self::new(StatusCode::UNPROCESSABLE_ENTITY, code, error)
  }

  /// 502 `upstream-failed`: a service this one called failed.
  pub fn upstream(error: impl Into<String>) -> Self {
    Self::new(StatusCode::BAD_GATEWAY, "upstream-failed", error)
  }

  /// 500 `failed`: a failure the route did not expect, with the failure's own words.
  pub fn failed(error: impl std::fmt::Display) -> Self {
    Self::new(StatusCode::INTERNAL_SERVER_ERROR, "failed", error.to_string())
  }
}

impl IntoResponse for Refused {
  fn into_response(self) -> Response {
    let body = WireError {
      code: self.code.to_owned(),
      error: self.error,
    };

    (self.status, Json(body)).into_response()
  }
}

/// What the service stands on, which every call carries.
#[derive(Clone)]
pub struct Standing {
  pub settings: Arc<Settings>,
  /// The store, open and migrated.
  pub store: Store,
  /// The event log every reader of `GET /events` hears.
  pub events: Events,
  /// Agency's client, at the address the settings name.
  pub agency: AgencyClient,
  /// The job service's client, at the address the settings name.
  pub job: JobClient,
  /// The console face, where a console credential is set and the bundle's folder holds a bundle.
  pub face: Option<ConsoleFace>,
}

impl Standing {
  /// What the service stands on, or why a client of another unit cannot be made. A console face that cannot be
  /// built leaves the service with none, and says why on standard error.
  pub fn new(settings: Settings, store: Store) -> Result<Self, String> {
    let agency = AgencyClient::new(&settings.agency.address, settings.agency.token.clone())
      .map_err(|why| format!("agency's address does not read: {why}"))?;
    let job = JobClient::new(&settings.job.address, &settings.job.token)
      .map_err(|why| format!("the job service's address does not read: {why}"))?;
    let face = console_face(&settings);

    Ok(Self {
      settings: Arc::new(settings),
      events: Events::new(store.clone()),
      store,
      agency,
      job,
      face,
    })
  }
}

// The console face of the settings, none where no console credential is set or the folder holds no bundle.
fn console_face(settings: &Settings) -> Option<ConsoleFace> {
  if settings.console.token.is_empty() {
    eprintln!("bmo-review: the console face is not served: no console credential is set");

    return None;
  }

  console::face(&settings.console.folder, &settings.console.token)
    .inspect_err(|why| eprintln!("bmo-review: the console face is not served: {why}"))
    .ok()
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

  /// The parameters the route's path names, by name, percent-decoded.
  pub async fn path_params(&mut self) -> Result<HashMap<String, String>, Refused> {
    let (mut parts, body) = std::mem::take(&mut self.request).into_parts();
    let read = RawPathParams::from_request_parts(&mut parts, &()).await;

    self.request = Request::from_parts(parts, body);

    let params = read.map_err(|why| Refused::bad_request(why.to_string()))?;

    Ok(params.iter().map(|(name, value)| (name.to_owned(), value.to_owned())).collect())
  }

  /// One parameter of the route's path, which the route's own path names.
  pub async fn path_param(&mut self, name: &str) -> Result<String, Refused> {
    self
      .path_params()
      .await?
      .remove(name)
      .ok_or_else(|| Refused::failed(format!("the route's path names no {name}")))
  }

  /// One parameter of the route's path read as an id, a whole number.
  pub async fn path_id(&mut self, name: &str) -> Result<i64, Refused> {
    let value = self.path_param(name).await?;

    value
      .parse()
      .map_err(|_| Refused::bad_request(format!("{name} {value} is not a whole number")))
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

  /// Whether the caller asks for JSON rather than an event stream.
  pub fn accepts_json(&self) -> bool {
    self.header(header::ACCEPT).is_some_and(|accept| accept.contains("application/json"))
  }

  /// The query read as the route's query type; a query of another shape is refused.
  pub fn query<T: DeserializeOwned>(&self) -> Result<T, Refused> {
    Query::<T>::try_from_uri(self.request.uri())
      .map(|Query(query)| query)
      .map_err(|why| Refused::bad_request(format!("the query is not the shape this route takes: {why}")))
  }

  /// The body read as the route's body type; a body that is not JSON of that shape is refused. A route reads it
  /// once.
  pub async fn body<T: DeserializeOwned>(&mut self) -> Result<T, Refused> {
    let body = std::mem::take(self.request.body_mut());
    let bytes = axum::body::to_bytes(body, BODY_LIMIT)
      .await
      .map_err(|why| Refused::bad_request(why.to_string()))?;

    serde_json::from_slice(&bytes)
      .map_err(|why| Refused::bad_request(format!("the body is not the shape this route takes: {why}")))
  }
}

/// An answer: a JSON body under a status.
pub fn answer(status: StatusCode, body: &impl Serialize) -> Response {
  (status, Json(body)).into_response()
}

/// The page size a list asks for: 1 to `most`, `default` where absent.
pub fn page_size(limit: Option<u32>, default: usize, most: usize) -> Result<usize, Refused> {
  match limit {
    None => Ok(default.min(most)),
    Some(limit) if limit >= 1 && limit as usize <= most => Ok(limit as usize),
    Some(limit) => Err(Refused::bad_request(format!("limit {limit} is not 1 to {most}"))),
  }
}

/// The page size of a list where none is asked.
pub const PAGE: usize = 100;
/// The most a list's page holds.
pub const MOST_PAGE: usize = 1_000;

/// A route at its own path is let in only with the service token as its bearer; an empty token lets nobody in.
async fn require_service_token(
  State(standing): State<Arc<Standing>>,
  request: Request,
  next: Next,
) -> Response {
  bearer_is(&standing.settings.server.token, request, next).await
}

/// A route under the console's prefix is let in only with the console credential as its bearer.
async fn require_console_credential(
  State(standing): State<Arc<Standing>>,
  request: Request,
  next: Next,
) -> Response {
  bearer_is(&standing.settings.console.token, request, next).await
}

async fn bearer_is(expected: &str, request: Request, next: Next) -> Response {
  let presented = request
    .headers()
    .get(header::AUTHORIZATION)
    .and_then(|value| value.to_str().ok())
    .and_then(|value| value.strip_prefix("Bearer "));

  match presented {
    Some(token) if !expected.is_empty() && token == expected => next.run(request).await,
    _ => Refused::new(StatusCode::UNAUTHORIZED, "unauthorized", "a missing or wrong token").into_response(),
  }
}

async fn no_route(request: Request) -> Response {
  Refused::new(
    StatusCode::NOT_FOUND,
    "no-route",
    format!("no route answers {}", route_name(request.method(), request.uri().path())),
  )
  .into_response()
}

/// The router answering every registered route behind the service token, and, where a console face stands, its
/// routes and every registered route again under the console's prefix, behind the console credential and the
/// face's guard of the screen's role; or what is wrong with the routes linked in.
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
  let routes = by_path
    .into_iter()
    .fold(Router::new(), |router, (path, methods)| router.route(path, methods.fallback(no_route)));
  let mut router = routes
    .clone()
    .layer(middleware::from_fn_with_state(standing.clone(), require_service_token));

  if let Some(face) = &standing.face {
    let prefix = face.manifest()["apiPrefix"].as_str().unwrap_or_default();
    let screens = face
      .guard_screens(routes)
      .fallback(no_route)
      .layer(middleware::from_fn_with_state(standing.clone(), require_console_credential));

    router = router.nest(prefix, screens).merge(face.router());
  }

  Ok(router.fallback(no_route).with_state(standing))
}
