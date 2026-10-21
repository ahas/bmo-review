//! The old archive's code reviews moved into the store (TODO #1923): the mapping of the three old tables,
//! the refusal of a store already holding rows, the dry run, the empty export as a no-op, and the
//! count-and-sample compare.

use std::path::{Path, PathBuf};

use bmo_review::import::{self, TABLES};
use bmo_review::store::Store;

fn scratch(name: &str) -> PathBuf {
  let folder = std::env::temp_dir().join(format!("bmo-review-import-{}-{name}", std::process::id()));
  let _ = std::fs::remove_dir_all(&folder);
  std::fs::create_dir_all(&folder).expect("the scratch folder is made");

  folder
}

fn manifest(folder: &Path, reviews: u64, talk: u64, filings: u64) {
  let said = format!(
    r#"{{"snapshot":"1:2:","readAt":"2026-10-21T17:00:00+09:00","tables":[
      {{"source":"code_reviews","file":"code_reviews.ndjson","count":{reviews},"key":["id"]}},
      {{"source":"code_review_talk","file":"code_review_talk.ndjson","count":{talk},"key":["id"]}},
      {{"source":"code_review_todos","file":"code_review_todos.ndjson","count":{filings},"key":["review_id","todo_id"]}}]}}"#
  );
  std::fs::write(folder.join("manifest.json"), said).expect("the manifest is written");
}

fn write(folder: &Path, table: &str, lines: &[&str]) {
  std::fs::write(folder.join(format!("{table}.ndjson")), lines.join("\n") + "\n")
    .expect("the rows are written");
}

const REVIEW: &str = r#"{"id":7,"worker":"bmo","root":"/work/bmo","commit_sha":"abcdef1","scope":"range","path":"src/lib.rs","start_line":10,"end_line":12,"excerpt":"let a = 1;\nlet b = 2;","created_at":1791866256,"rowid":1}"#;
const TALK: &str =
  r#"{"id":1,"review_id":7,"speaker":"ahas","text":"why this range?","said_at":1791866300,"rowid":1}"#;
const FILING: &str = r#"{"review_id":7,"todo_id":1923,"rowid":1}"#;

#[tokio::test]
async fn the_three_tables_are_mapped_and_compared() {
  let folder = scratch("samples");
  manifest(&folder, 1, 1, 1);
  write(&folder, "code_reviews", &[REVIEW]);
  write(&folder, "code_review_talk", &[TALK]);
  write(&folder, "code_review_todos", &[FILING]);

  let store = folder.join("review.db");
  let report = import::run(&store, &folder, false).await.expect("the import runs");

  assert_eq!(report.tables.len(), 3);
  assert_eq!(report.tables[0].written, 1);
  assert_eq!(report.tables[1].written, 1);
  assert_eq!(report.tables[2].written, 1);
  assert_eq!(report.sample.checked, 3);
  assert_eq!(report.sample.failures, Vec::<String>::new());

  let opened = Store::open(&store).await.expect("the store opens");
  let pool = opened.pool();
  let review: (String, String, String, String) =
    sqlx::query_as("SELECT project_name, revision, title, state FROM reviews WHERE id = 7")
      .fetch_one(pool)
      .await
      .expect("the review is read");
  assert_eq!(
    review,
    ("bmo".to_owned(), "abcdef1".to_owned(), "src/lib.rs".to_owned(), "open".to_owned())
  );
  type CommentRow = (Option<String>, Option<i64>, Option<i64>, String);
  let comments: Vec<CommentRow> =
    sqlx::query_as("SELECT path, line, end_line, text FROM comments ORDER BY id")
      .fetch_all(pool)
      .await
      .expect("the comments are read");
  assert_eq!(comments.len(), 2);
  assert_eq!(comments[0].3, "why this range?");
  assert_eq!(comments[1].0.as_deref(), Some("src/lib.rs"));
  assert_eq!(comments[1].1, Some(10));
  assert_eq!(comments[1].2, Some(12));
  let filing: i64 = sqlx::query_scalar("SELECT item_id FROM filings WHERE review_id = 7")
    .fetch_one(pool)
    .await
    .expect("the filing is read");
  assert_eq!(filing, 1923);
  assert_eq!(
    TABLES.iter().map(|table| table.target).collect::<Vec<_>>(),
    ["reviews", "comments", "filings"]
  );
}

#[tokio::test]
async fn an_empty_export_is_a_documented_no_op() {
  let folder = scratch("empty");
  manifest(&folder, 0, 0, 0);
  write(&folder, "code_reviews", &[]);
  write(&folder, "code_review_talk", &[]);
  write(&folder, "code_review_todos", &[]);

  let store = folder.join("review.db");
  let report = import::run(&store, &folder, false).await.expect("the import runs");
  assert_eq!(report.sample.checked, 0);
  assert_eq!(report.sample.failures, Vec::<String>::new());
  assert!(report.lines().iter().any(|line| line.contains("documented no-op")));
}

#[tokio::test]
async fn a_dry_run_writes_nothing() {
  let folder = scratch("dry");
  manifest(&folder, 1, 1, 1);
  write(&folder, "code_reviews", &[REVIEW]);
  write(&folder, "code_review_talk", &[TALK]);
  write(&folder, "code_review_todos", &[FILING]);

  let store = folder.join("review.db");
  let report = import::run(&store, &folder, true).await.expect("the dry run runs");
  assert!(report.dry_run);

  let opened = Store::open(&store).await.expect("the store opens");
  let held: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reviews")
    .fetch_one(opened.pool())
    .await
    .expect("the count is read");
  assert_eq!(held, 0);
}

#[tokio::test]
async fn a_store_holding_rows_is_refused() {
  let folder = scratch("refuse");
  manifest(&folder, 1, 1, 1);
  write(&folder, "code_reviews", &[REVIEW]);
  write(&folder, "code_review_talk", &[TALK]);
  write(&folder, "code_review_todos", &[FILING]);

  let store = folder.join("review.db");
  import::run(&store, &folder, false).await.expect("the first import runs");
  let refused = import::run(&store, &folder, false).await.expect_err("the second is refused");

  assert!(refused.contains("already holds rows"), "{refused}");
  assert!(refused.contains("reviews holds 1"), "{refused}");
}
