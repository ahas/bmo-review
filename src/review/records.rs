//! The reviews and their threads as this service's store keeps them (`docs/code-review.md`, 「The review table」,
//! 「The exchange」).

use serde::Serialize;
use sqlx::{FromRow, SqlitePool};

use crate::http::Refused;

/// What a review is pinned to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
  Line,
  Range,
  File,
  Folder,
}

impl Scope {
  pub fn of(name: &str) -> Option<Self> {
    match name {
      "line" => Some(Self::Line),
      "range" => Some(Self::Range),
      "file" => Some(Self::File),
      "folder" => Some(Self::Folder),
      _ => None,
    }
  }

  pub fn name(self) -> &'static str {
    match self {
      Self::Line => "line",
      Self::Range => "range",
      Self::File => "file",
      Self::Folder => "folder",
    }
  }

  /// Whether the place is lines of a file, carrying lines and an excerpt.
  pub fn lined(self) -> bool {
    matches!(self, Self::Line | Self::Range)
  }
}

/// Who said a line of a thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Speaker {
  Owner,
  Agent,
}

impl Speaker {
  fn name(self) -> &'static str {
    match self {
      Self::Owner => "owner",
      Self::Agent => "agent",
    }
  }
}

/// A review as the store keeps it.
#[derive(Clone, Debug, FromRow)]
pub struct Review {
  pub id: i64,
  pub worker: String,
  pub root: String,
  pub commit_sha: String,
  pub scope: String,
  pub path: String,
  pub start_line: Option<i64>,
  pub end_line: Option<i64>,
  pub excerpt: Option<String>,
  pub created_at: i64,
}

/// A review to be written.
pub struct NewReview<'a> {
  pub worker: &'a str,
  pub root: &'a str,
  pub commit: &'a str,
  pub scope: Scope,
  pub path: &'a str,
  pub start_line: Option<i64>,
  pub end_line: Option<i64>,
  pub excerpt: Option<&'a str>,
  pub created_at: i64,
}

/// A review as a list shows it.
#[derive(Clone, Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
  pub id: i64,
  pub worker: String,
  pub commit: String,
  pub scope: String,
  pub path: String,
  pub start_line: Option<i64>,
  pub end_line: Option<i64>,
  pub created_at: i64,
  /// The thread's first words.
  pub first: String,
  /// The count of its words.
  pub turns: i64,
  /// When the last words were said.
  pub last_at: i64,
  #[serde(skip)]
  pub excerpt: Option<String>,
  #[serde(skip)]
  pub filed: Option<String>,
  /// The items filed from it.
  #[sqlx(skip)]
  pub todo_ids: Vec<i64>,
}

impl Summary {
  fn read(mut self) -> Self {
    self.todo_ids = self
      .filed
      .as_deref()
      .unwrap_or_default()
      .split(',')
      .filter_map(|id| id.trim().parse().ok())
      .collect();
    self
  }
}

/// One line of a thread.
#[derive(Clone, Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Turn {
  pub id: i64,
  pub speaker: String,
  pub text: String,
  pub said_at: i64,
}

impl Turn {
  pub fn by_owner(&self) -> bool {
    self.speaker == Speaker::Owner.name()
  }
}

const SUMMARY: &str = r#"
  SELECT r.id, r.worker, r.commit_sha AS "commit", r.scope, r.path, r.start_line, r.end_line, r.created_at,
    r.excerpt,
    COALESCE((SELECT text FROM code_review_talk WHERE review_id = r.id ORDER BY id LIMIT 1), '') AS first,
    (SELECT count(*) FROM code_review_talk WHERE review_id = r.id) AS turns,
    COALESCE((SELECT max(said_at) FROM code_review_talk WHERE review_id = r.id), r.created_at) AS last_at,
    (SELECT group_concat(filed_item, ',') FROM code_review_todos WHERE review_id = r.id) AS filed
  FROM code_reviews r
"#;

fn failed(why: sqlx::Error) -> Refused {
  Refused::new(axum::http::StatusCode::INTERNAL_SERVER_ERROR, "database", why.to_string())
}

/// Every review of one worker, newest first.
pub async fn summaries(pool: &SqlitePool, worker: &str) -> Result<Vec<Summary>, Refused> {
  let rows: Vec<Summary> = sqlx::query_as(&format!("{SUMMARY} WHERE r.worker = ? ORDER BY r.id DESC"))
    .bind(worker)
    .fetch_all(pool)
    .await
    .map_err(failed)?;

  Ok(rows.into_iter().map(Summary::read).collect())
}

/// One review as a list shows it, with its excerpt.
pub async fn summary(pool: &SqlitePool, id: i64) -> Result<Option<Summary>, Refused> {
  let row: Option<Summary> = sqlx::query_as(&format!("{SUMMARY} WHERE r.id = ?"))
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(failed)?;

  Ok(row.map(Summary::read))
}

/// One review as it was written.
pub async fn review(pool: &SqlitePool, id: i64) -> Result<Option<Review>, Refused> {
  sqlx::query_as("SELECT * FROM code_reviews WHERE id = ?")
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(failed)
}

/// Writes a review, answering its number.
pub async fn make(pool: &SqlitePool, review: &NewReview<'_>) -> Result<i64, Refused> {
  sqlx::query_scalar(
    "INSERT INTO code_reviews (worker, root, commit_sha, scope, path, start_line, end_line, excerpt, created_at)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
  )
  .bind(review.worker)
  .bind(review.root)
  .bind(review.commit)
  .bind(review.scope.name())
  .bind(review.path)
  .bind(review.start_line)
  .bind(review.end_line)
  .bind(review.excerpt)
  .bind(review.created_at)
  .fetch_one(pool)
  .await
  .map_err(failed)
}

/// A review's thread, oldest first.
pub async fn thread(pool: &SqlitePool, id: i64) -> Result<Vec<Turn>, Refused> {
  sqlx::query_as("SELECT id, speaker, text, said_at FROM code_review_talk WHERE review_id = ? ORDER BY id")
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(failed)
}

/// Writes one line of a review's thread.
pub async fn say(pool: &SqlitePool, id: i64, speaker: Speaker, text: &str, at: i64) -> Result<(), Refused> {
  sqlx::query("INSERT INTO code_review_talk (review_id, speaker, text, said_at) VALUES (?, ?, ?, ?)")
    .bind(id)
    .bind(speaker.name())
    .bind(text)
    .bind(at)
    .execute(pool)
    .await
    .map(drop)
    .map_err(failed)
}

/// Remembers an item filed from a review.
pub async fn filed(pool: &SqlitePool, id: i64, item: i64) -> Result<(), Refused> {
  sqlx::query("INSERT INTO code_review_todos (review_id, filed_item) VALUES (?, ?)")
    .bind(id)
    .bind(item)
    .execute(pool)
    .await
    .map(drop)
    .map_err(failed)
}

/// Now, in Unix seconds.
pub fn now() -> i64 {
  std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map_or(0, |since| since.as_secs() as i64)
}
