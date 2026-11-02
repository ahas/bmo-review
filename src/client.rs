//! The service's own client (`docs/api.md`, 「The client」): one method for each route, sending and reading the
//! bodies of [`crate::wire`], and a failure read back as its status, code and sentence. It carries the contract
//! and no caller's policy.

use reqwest::header::{ACCEPT, AUTHORIZATION};
use reqwest::{Method, RequestBuilder, Url};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::wire::{
  Comment, CommentQuery, Commit, CommitQuery, Definition, DefinitionQuery, Event, EventQuery, FileQuery,
  FileSource, Filing, Health, Match, MatchQuery, NewComment, NewFiling, NewReview, Page, Project,
  ProjectQuery, Review, ReviewChange, ReviewQuery, ScreenManifest, TreeEntry, TreeQuery, WireError, Words,
};

/// The query of a route that takes none.
const NO_QUERY: [(&str, &str); 0] = [];

/// Why a call came to no answer of the route's shape.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ClientFailure {
  /// The service answered with a status that is no success: its error body where it was one, and the body as
  /// it came.
  #[error("the review service answered {status}: {}", answer.as_ref().map_or(body.as_str(), |answer| answer.error.as_str()))]
  Answered {
    status: u16,
    answer: Option<WireError>,
    body: String,
  },
  /// Nothing answered at the address.
  #[error("the review service could not be reached: {0}")]
  Unreached(String),
  /// A success came back in no shape the route answers.
  #[error("the review service's answer does not read: {0}")]
  Unreadable(String),
}

type Called<T> = Result<T, ClientFailure>;

/// The client of one review service: its address and its bearer token.
#[derive(Clone, Debug)]
pub struct ReviewClient {
  base: Url,
  token: String,
  http: reqwest::Client,
}

impl ReviewClient {
  /// A client of the service whose routes stand at `base`, sending `token` as its bearer token.
  pub fn new(base: &str, token: &str) -> Called<Self> {
    let base = Url::parse(base).map_err(|why| ClientFailure::Unreached(format!("{base}: {why}")))?;
    let http = reqwest::Client::builder()
      .build()
      .map_err(|why| ClientFailure::Unreached(why.to_string()))?;

    Ok(Self {
      base,
      token: token.to_owned(),
      http,
    })
  }

  // The address of a route's path, each segment given encoded as a path segment.
  fn address(&self, segments: &[&str]) -> Url {
    let mut address = self.base.clone();

    if let Ok(mut path) = address.path_segments_mut() {
      path.pop_if_empty().extend(segments);
    }

    address
  }

  fn request(&self, method: Method, segments: &[&str]) -> RequestBuilder {
    self
      .http
      .request(method, self.address(segments))
      .header(AUTHORIZATION, format!("Bearer {}", self.token))
      .header(ACCEPT, "application/json")
  }

  // One call sent; its answer read as the route's shape.
  async fn read<T: DeserializeOwned>(request: RequestBuilder) -> Called<T> {
    let answered = request.send().await.map_err(|why| ClientFailure::Unreached(why.to_string()))?;
    let status = answered.status().as_u16();
    let body = answered.text().await.map_err(|why| ClientFailure::Unreadable(why.to_string()))?;

    if !(200..300).contains(&status) {
      return Err(ClientFailure::Answered {
        status,
        answer: serde_json::from_str(&body).ok(),
        body,
      });
    }

    serde_json::from_str(&body).map_err(|why| ClientFailure::Unreadable(why.to_string()))
  }

  async fn get<T: DeserializeOwned>(&self, segments: &[&str], query: &impl Serialize) -> Called<T> {
    Self::read(self.request(Method::GET, segments).query(query)).await
  }

  async fn send<T: DeserializeOwned>(
    &self,
    method: Method,
    segments: &[&str],
    body: &impl Serialize,
  ) -> Called<T> {
    Self::read(self.request(method, segments).json(body)).await
  }

  /// `GET /health`.
  pub async fn health(&self) -> Called<Health> {
    self.get(&["health"], &NO_QUERY).await
  }

  /// `GET /console-manifest`.
  pub async fn screen_manifest(&self) -> Called<ScreenManifest> {
    self.get(&["console-manifest"], &NO_QUERY).await
  }

  /// `GET /words`.
  pub async fn words(&self) -> Called<Words> {
    self.get(&["words"], &NO_QUERY).await
  }

  /// `GET /bundle/{version}/{fileName}`: the file's bytes as they came.
  pub async fn bundle_file(&self, version: &str, file_name: &str) -> Called<Vec<u8>> {
    let segments: Vec<&str> = ["bundle", version].into_iter().chain(file_name.split('/')).collect();
    let answered = self
      .http
      .get(self.address(&segments))
      .header(AUTHORIZATION, format!("Bearer {}", self.token))
      .send()
      .await
      .map_err(|why| ClientFailure::Unreached(why.to_string()))?;
    let status = answered.status().as_u16();
    let bytes = answered.bytes().await.map_err(|why| ClientFailure::Unreadable(why.to_string()))?;

    if !(200..300).contains(&status) {
      let body = String::from_utf8_lossy(&bytes).into_owned();

      return Err(ClientFailure::Answered {
        status,
        answer: serde_json::from_str(&body).ok(),
        body,
      });
    }

    Ok(bytes.to_vec())
  }

  /// `GET /events`, as a page.
  pub async fn events(&self, query: &EventQuery) -> Called<Page<Event>> {
    self.get(&["events"], query).await
  }

  /// `GET /projects`.
  pub async fn projects(&self, query: &ProjectQuery) -> Called<Page<Project>> {
    self.get(&["projects"], query).await
  }

  /// `GET /projects/{projectName}/tree`.
  pub async fn tree(&self, project_name: &str, query: &TreeQuery) -> Called<Page<TreeEntry>> {
    self.get(&["projects", project_name, "tree"], query).await
  }

  /// `GET /projects/{projectName}/files/{filePath}`.
  pub async fn file(&self, project_name: &str, file_path: &str, query: &FileQuery) -> Called<FileSource> {
    let segments: Vec<&str> =
      ["projects", project_name, "files"].into_iter().chain(file_path.split('/')).collect();

    self.get(&segments, query).await
  }

  /// `GET /projects/{projectName}/commits`.
  pub async fn commits(&self, project_name: &str, query: &CommitQuery) -> Called<Page<Commit>> {
    self.get(&["projects", project_name, "commits"], query).await
  }

  /// `GET /projects/{projectName}/commits/{commitId}`.
  pub async fn commit(&self, project_name: &str, commit_id: &str) -> Called<Commit> {
    self.get(&["projects", project_name, "commits", commit_id], &NO_QUERY).await
  }

  /// `GET /projects/{projectName}/matches`.
  pub async fn matches(&self, project_name: &str, query: &MatchQuery) -> Called<Page<Match>> {
    self.get(&["projects", project_name, "matches"], query).await
  }

  /// `GET /projects/{projectName}/definitions`.
  pub async fn definitions(&self, project_name: &str, query: &DefinitionQuery) -> Called<Page<Definition>> {
    self.get(&["projects", project_name, "definitions"], query).await
  }

  /// `GET /reviews`.
  pub async fn reviews(&self, query: &ReviewQuery) -> Called<Page<Review>> {
    self.get(&["reviews"], query).await
  }

  /// `POST /reviews`.
  pub async fn create_review(&self, review: &NewReview) -> Called<Review> {
    self.send(Method::POST, &["reviews"], review).await
  }

  /// `GET /reviews/{reviewId}`.
  pub async fn review(&self, review_id: i64) -> Called<Review> {
    self.get(&["reviews", &review_id.to_string()], &NO_QUERY).await
  }

  /// `PATCH /reviews/{reviewId}`.
  pub async fn change_review(&self, review_id: i64, change: &ReviewChange) -> Called<Review> {
    self.send(Method::PATCH, &["reviews", &review_id.to_string()], change).await
  }

  /// `GET /reviews/{reviewId}/comments`.
  pub async fn comments(&self, review_id: i64, query: &CommentQuery) -> Called<Page<Comment>> {
    self.get(&["reviews", &review_id.to_string(), "comments"], query).await
  }

  /// `POST /reviews/{reviewId}/comments`.
  pub async fn create_comment(&self, review_id: i64, comment: &NewComment) -> Called<Comment> {
    self.send(Method::POST, &["reviews", &review_id.to_string(), "comments"], comment).await
  }

  /// `POST /reviews/{reviewId}/filings`.
  pub async fn create_filing(&self, review_id: i64, filing: &NewFiling) -> Called<Filing> {
    self.send(Method::POST, &["reviews", &review_id.to_string(), "filings"], filing).await
  }
}
