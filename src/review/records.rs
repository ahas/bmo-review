//! The reviews, their comments and the items filed from them as this service's store keeps them
//! (`docs/code-review.md`, 「Reviews」, 「Comments」, 「Filing an item」).

use sqlx::{FromRow, Sqlite, SqlitePool, Transaction};

use crate::http::Refused;
use crate::wire::{Comment, CommentQuery, Filing, Review, ReviewQuery, ReviewState};

/// A review as one row reads it, with its count of comments and the items filed from it.
const REVIEW: &str = "SELECT r.id, r.project_name, r.revision, r.title, r.state, r.author_name, r.created_at, \
  r.changed_at, (SELECT COUNT(*) FROM comments c WHERE c.review_id = r.id) AS comment_count, \
  (SELECT json_group_array(item_id) FROM (SELECT item_id FROM filings f WHERE f.review_id = r.id ORDER BY f.id)) \
  AS item_ids FROM reviews r";

const COMMENT: &str = "SELECT id, review_id, path, line, end_line, excerpt, text, author_name, created_at \
  FROM comments";

fn failed(why: sqlx::Error) -> Refused {
  Refused::failed(format!("the store failed: {why}"))
}

#[derive(FromRow)]
struct ReviewRow {
  id: i64,
  project_name: String,
  revision: String,
  title: String,
  state: String,
  author_name: String,
  created_at: i64,
  changed_at: i64,
  comment_count: i64,
  item_ids: String,
}

impl ReviewRow {
  fn review(self) -> Review {
    Review {
      id: self.id,
      project_name: self.project_name,
      revision: self.revision,
      title: self.title,
      state: ReviewState::parse(&self.state).unwrap_or(ReviewState::Open),
      author_name: self.author_name,
      comment_count: u64::try_from(self.comment_count).unwrap_or(0),
      item_ids: serde_json::from_str(&self.item_ids).unwrap_or_default(),
      created_at: self.created_at,
      changed_at: self.changed_at,
    }
  }
}

#[derive(FromRow)]
struct CommentRow {
  id: i64,
  review_id: i64,
  path: Option<String>,
  line: Option<i64>,
  end_line: Option<i64>,
  excerpt: Option<String>,
  text: String,
  author_name: String,
  created_at: i64,
}

impl CommentRow {
  fn comment(self) -> Comment {
    Comment {
      id: self.id,
      review_id: self.review_id,
      path: self.path,
      line: self.line.and_then(|line| u32::try_from(line).ok()),
      end_line: self.end_line.and_then(|line| u32::try_from(line).ok()),
      excerpt: self.excerpt,
      text: self.text,
      author_name: self.author_name,
      created_at: self.created_at,
    }
  }
}

/// One review, where it stands.
pub async fn review(pool: &SqlitePool, id: i64) -> Result<Option<Review>, Refused> {
  let row: Option<ReviewRow> = sqlx::query_as(&format!("{REVIEW} WHERE r.id = ?"))
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(failed)?;

  Ok(row.map(ReviewRow::review))
}

/// One review inside a write, where it stands.
pub async fn review_in(
  transaction: &mut Transaction<'static, Sqlite>,
  id: i64,
) -> Result<Option<Review>, Refused> {
  let row: Option<ReviewRow> = sqlx::query_as(&format!("{REVIEW} WHERE r.id = ?"))
    .bind(id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(failed)?;

  Ok(row.map(ReviewRow::review))
}

/// The reviews a query narrows to, newest first, past the cursor: at most `limit`, and one more to tell whether
/// more stand.
pub async fn reviews(pool: &SqlitePool, query: &ReviewQuery, limit: usize) -> Result<Vec<Review>, Refused> {
  let rows: Vec<ReviewRow> = sqlx::query_as(&format!(
    "{REVIEW} WHERE (?1 IS NULL OR r.project_name = ?1) AND (?2 IS NULL OR r.revision = ?2) \
     AND (?3 IS NULL OR r.state = ?3) AND (?4 IS NULL OR r.id < ?4) ORDER BY r.id DESC LIMIT ?5"
  ))
  .bind(query.project_name.as_deref())
  .bind(query.revision.as_deref())
  .bind(query.state.map(ReviewState::as_str))
  .bind(query.after)
  .bind(i64::try_from(limit + 1).unwrap_or(i64::MAX))
  .fetch_all(pool)
  .await
  .map_err(failed)?;

  Ok(rows.into_iter().map(ReviewRow::review).collect())
}

/// A review opened, its id.
pub async fn open_review(
  transaction: &mut Transaction<'static, Sqlite>,
  project_name: &str,
  revision: &str,
  title: &str,
  author_name: &str,
  at: i64,
) -> Result<i64, Refused> {
  sqlx::query_scalar(
    "INSERT INTO reviews (project_name, revision, title, state, author_name, created_at, changed_at) \
     VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id",
  )
  .bind(project_name)
  .bind(revision)
  .bind(title)
  .bind(ReviewState::Open.as_str())
  .bind(author_name)
  .bind(at)
  .bind(at)
  .fetch_one(&mut **transaction)
  .await
  .map_err(failed)
}

/// A review's state set, and the time it changed.
pub async fn set_state(
  transaction: &mut Transaction<'static, Sqlite>,
  id: i64,
  state: ReviewState,
  at: i64,
) -> Result<(), Refused> {
  sqlx::query("UPDATE reviews SET state = ?, changed_at = ? WHERE id = ?")
    .bind(state.as_str())
    .bind(at)
    .bind(id)
    .execute(&mut **transaction)
    .await
    .map_err(failed)?;

  Ok(())
}

/// The author of a review's last comment, where it has one.
pub async fn last_commenter(
  transaction: &mut Transaction<'static, Sqlite>,
  id: i64,
) -> Result<Option<String>, Refused> {
  sqlx::query_scalar("SELECT author_name FROM comments WHERE review_id = ? ORDER BY id DESC LIMIT 1")
    .bind(id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(failed)
}

/// A review's comments, oldest first, past the cursor: at most `limit`, and one more to tell whether more stand.
pub async fn comments(
  pool: &SqlitePool,
  review_id: i64,
  query: &CommentQuery,
  limit: usize,
) -> Result<Vec<Comment>, Refused> {
  let rows: Vec<CommentRow> = sqlx::query_as(&format!(
    "{COMMENT} WHERE review_id = ?1 AND (?2 IS NULL OR id > ?2) ORDER BY id LIMIT ?3"
  ))
  .bind(review_id)
  .bind(query.after)
  .bind(i64::try_from(limit + 1).unwrap_or(i64::MAX))
  .fetch_all(pool)
  .await
  .map_err(failed)?;

  Ok(rows.into_iter().map(CommentRow::comment).collect())
}

/// A comment as it is written.
pub struct Written<'a> {
  pub review_id: i64,
  pub path: Option<&'a str>,
  pub line: Option<u32>,
  pub end_line: Option<u32>,
  pub excerpt: Option<&'a str>,
  pub text: &'a str,
  pub author_name: &'a str,
  pub at: i64,
}

/// A comment written, as it now stands.
pub async fn write_comment(
  transaction: &mut Transaction<'static, Sqlite>,
  written: &Written<'_>,
) -> Result<Comment, Refused> {
  let row: CommentRow = sqlx::query_as(
    "INSERT INTO comments (review_id, path, line, end_line, excerpt, text, author_name, created_at) \
     VALUES (?, ?, ?, ?, ?, ?, ?, ?) \
     RETURNING id, review_id, path, line, end_line, excerpt, text, author_name, created_at",
  )
  .bind(written.review_id)
  .bind(written.path)
  .bind(written.line)
  .bind(written.end_line)
  .bind(written.excerpt)
  .bind(written.text)
  .bind(written.author_name)
  .bind(written.at)
  .fetch_one(&mut **transaction)
  .await
  .map_err(failed)?;

  Ok(row.comment())
}

/// An item filed from a review remembered, as it now stands.
pub async fn write_filing(
  transaction: &mut Transaction<'static, Sqlite>,
  review_id: i64,
  item_id: i64,
  at: i64,
) -> Result<Filing, Refused> {
  let id: i64 = sqlx::query_scalar(
    "INSERT INTO filings (review_id, item_id, created_at) VALUES (?, ?, ?) RETURNING id",
  )
  .bind(review_id)
  .bind(item_id)
  .bind(at)
  .fetch_one(&mut **transaction)
  .await
  .map_err(failed)?;

  sqlx::query("UPDATE reviews SET changed_at = ? WHERE id = ?")
    .bind(at)
    .bind(review_id)
    .execute(&mut **transaction)
    .await
    .map_err(failed)?;

  Ok(Filing {
    id,
    review_id,
    item_id,
    created_at: at,
  })
}

/// A write begun on the store.
pub async fn begin(pool: &SqlitePool) -> Result<Transaction<'static, Sqlite>, Refused> {
  pool.begin().await.map_err(failed)
}

/// A write committed.
pub async fn commit(transaction: Transaction<'static, Sqlite>) -> Result<(), Refused> {
  transaction.commit().await.map_err(failed)
}
