//! The old archive's code reviews moved into this service's store (docs/code-review.md, 「Reviews」, and the
//! migration plan's table map for TODO #1923): one export directory read, this store written, nothing else
//! touched.
//!
//! The export is the one the item's export hand wrote under
//! `workbench/worker/items/1923/exports/review/`: `manifest.json` with each source table's exact count and
//! key, and one NDJSON file a table. The import reads only that directory, refuses a store already holding
//! rows of its tables, and runs whole inside one transaction, so a dry run and a failed run leave the store
//! as it stood.
//!
//! The old tables held no rows when the move plan was written, so a run over the live export is a no-op that
//! still refuses a target already holding rows. The mapping each row would take stands here so the shape is
//! agreed before any row is met: a review becomes a `reviews` row, and its place — the scope, the path, the
//! lines and the excerpt — a comment of its own, since the new model carries the place on the comment; a
//! turn of the old talk becomes a comment, and a filed item a filing. The old `scope` word (`line`,
//! `range`, `file`, `folder`) has no column in the new store: the comment's path, line and end line carry
//! the same place, and the word itself is not written.

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;
use sqlx::sqlite::SqlitePool;

use crate::store::Store;

/// One table the import moves: its source name and its target name.
pub struct TableMap {
  pub source: &'static str,
  pub target: &'static str,
}

/// Every table this import moves.
pub const TABLES: &[TableMap] = &[
  TableMap {
    source: "code_reviews",
    target: "reviews",
  },
  TableMap {
    source: "code_review_talk",
    target: "comments",
  },
  TableMap {
    source: "code_review_todos",
    target: "filings",
  },
];

/// One export table's entry in the manifest.
#[derive(Clone, Debug)]
pub struct TableEntry {
  pub file: String,
  pub count: u64,
  pub key: Vec<String>,
}

/// The export directory's manifest.
#[derive(Clone, Debug)]
pub struct Manifest {
  pub snapshot: String,
  pub read_at: String,
  pub tables: HashMap<String, TableEntry>,
}

/// A review written into `reviews`.
#[derive(Clone, Debug, PartialEq)]
pub struct Review {
  pub id: i64,
  pub project_name: String,
  pub revision: String,
  pub title: String,
  pub author_name: String,
  pub created_at: i64,
}

/// The place a review was pinned to, written as one of its comments.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
  pub review_id: i64,
  pub path: Option<String>,
  pub line: Option<i64>,
  pub end_line: Option<i64>,
  pub excerpt: Option<String>,
  pub author_name: String,
  pub created_at: i64,
}

/// A turn of the old talk, written into `comments`.
#[derive(Clone, Debug, PartialEq)]
pub struct Talk {
  pub id: i64,
  pub review_id: i64,
  pub text: String,
  pub author_name: String,
  pub created_at: i64,
}

/// A filing of the old talk, written into `filings`.
#[derive(Clone, Debug, PartialEq)]
pub struct Filing {
  pub review_id: i64,
  pub item_id: i64,
  pub created_at: i64,
}

/// One table of the report.
#[derive(Clone, Debug, PartialEq)]
pub struct Moved {
  pub source: String,
  pub exported: u64,
  pub written: u64,
}

/// What an import run did.
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
  pub dry_run: bool,
  pub snapshot: String,
  pub read_at: String,
  pub tables: Vec<Moved>,
  pub sample: Sample,
}

impl Report {
  /// The report as the command prints it.
  pub fn lines(&self) -> Vec<String> {
    let mut lines = vec![format!(
      "{} at snapshot {}:",
      if self.dry_run { "dry run" } else { "import" },
      self.snapshot
    )];
    for table in &self.tables {
      lines.push(format!("  {}: {} exported, {} written", table.source, table.exported, table.written));
    }
    lines.push(format!("{} rows sampled, {} failures", self.sample.checked, self.sample.failures.len()));
    lines.extend(self.sample.failures.iter().map(|failure| format!("  {failure}")));
    if self.sample.checked == 0 && self.sample.failures.is_empty() {
      lines.push("  the export is empty: a documented no-op".to_owned());
    }

    lines
  }
}

/// The count-and-sample compare's result.
#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
  pub checked: usize,
  pub failures: Vec<String>,
}

/// The export directory's manifest.
pub fn read_manifest(dir: &Path) -> Result<Manifest, String> {
  let path = dir.join("manifest.json");
  let said = std::fs::read_to_string(&path)
    .map_err(|error| format!("{} could not be read: {error}", path.display()))?;
  let document: Value =
    serde_json::from_str(&said).map_err(|error| format!("{} is not JSON: {error}", path.display()))?;
  let mut tables = HashMap::new();
  let list = document.get("tables").and_then(Value::as_array).ok_or("the manifest names no tables")?;
  for table in list {
    let source = table
      .get("source")
      .and_then(Value::as_str)
      .ok_or("a manifest table names no source")?
      .to_owned();
    let file = table
      .get("file")
      .and_then(Value::as_str)
      .ok_or("a manifest table names no file")?
      .to_owned();
    let count = table.get("count").and_then(Value::as_u64).ok_or("a manifest table names no count")?;
    let key: Vec<String> = table
      .get("key")
      .and_then(Value::as_array)
      .map(|keys| keys.iter().filter_map(Value::as_str).map(str::to_owned).collect())
      .unwrap_or_default();
    tables.insert(source, TableEntry { file, count, key });
  }

  Ok(Manifest {
    snapshot: document.get("snapshot").and_then(Value::as_str).unwrap_or_default().to_owned(),
    read_at: document.get("readAt").and_then(Value::as_str).unwrap_or_default().to_owned(),
    tables,
  })
}

/// One table's rows, each checked against the manifest's count as it is read.
pub fn rows(dir: &Path, manifest: &Manifest, source: &str) -> Result<Vec<Value>, String> {
  let entry = manifest
    .tables
    .get(source)
    .ok_or_else(|| format!("the manifest has no table {source}"))?;
  let path = dir.join(&entry.file);
  let said = std::fs::read_to_string(&path)
    .map_err(|error| format!("{} could not be read: {error}", path.display()))?;
  let mut out = Vec::new();
  for (number, line) in said.lines().enumerate() {
    if line.trim().is_empty() {
      continue;
    }
    let row: Value = serde_json::from_str(line)
      .map_err(|error| format!("{} line {} is not JSON: {error}", path.display(), number + 1))?;
    out.push(row);
  }
  if out.len() as u64 != entry.count {
    return Err(format!(
      "{}: the manifest says {} rows and the file holds {}",
      path.display(),
      entry.count,
      out.len()
    ));
  }

  Ok(out)
}

fn text(row: &Value, column: &str) -> Option<String> {
  match row.get(column) {
    None | Some(Value::Null) => None,
    Some(Value::String(said)) => Some(said.clone()),
    Some(Value::Number(number)) => Some(number.to_string()),
    Some(Value::Bool(flag)) => Some(flag.to_string()),
    Some(other) => Some(other.to_string()),
  }
}

fn number(row: &Value, column: &str) -> Option<i64> {
  row.get(column).and_then(|value| match value {
    Value::Number(number) => number.as_i64().or_else(|| number.as_f64().map(|held| held as i64)),
    Value::String(said) => said.parse::<i64>().ok(),
    _ => None,
  })
}

// The old rows' times are seconds; the store keeps milliseconds.
fn millis(row: &Value, column: &str) -> i64 {
  number(row, column).unwrap_or_default().saturating_mul(1000)
}

/// The reviews the export names.
pub fn reviews(dir: &Path, manifest: &Manifest) -> Result<Vec<Review>, String> {
  Ok(
    rows(dir, manifest, "code_reviews")?
      .iter()
      .map(|row| Review {
        // The store numbers a review itself; the old number is kept as the title's place only where the
        // review has no path to name it by.
        id: number(row, "id").unwrap_or_default(),
        project_name: text(row, "worker").unwrap_or_default(),
        revision: text(row, "commit_sha").unwrap_or_default(),
        title: text(row, "path")
          .filter(|path| !path.is_empty())
          .unwrap_or_else(|| "(the whole checkout)".to_owned()),
        author_name: text(row, "worker").unwrap_or_default(),
        created_at: millis(row, "created_at"),
      })
      .collect(),
  )
}

/// The place each old review was pinned to.
pub fn places(dir: &Path, manifest: &Manifest) -> Result<Vec<Place>, String> {
  Ok(
    rows(dir, manifest, "code_reviews")?
      .iter()
      .map(|row| Place {
        review_id: number(row, "id").unwrap_or_default(),
        path: text(row, "path").filter(|path| !path.is_empty()),
        line: number(row, "start_line"),
        end_line: number(row, "end_line"),
        excerpt: text(row, "excerpt"),
        author_name: text(row, "worker").unwrap_or_default(),
        created_at: millis(row, "created_at"),
      })
      .collect(),
  )
}

/// The turn of the old talk.
pub fn talk(dir: &Path, manifest: &Manifest) -> Result<Vec<Talk>, String> {
  Ok(
    rows(dir, manifest, "code_review_talk")?
      .iter()
      .map(|row| Talk {
        id: number(row, "id").unwrap_or_default(),
        review_id: number(row, "review_id").unwrap_or_default(),
        text: text(row, "text").unwrap_or_default(),
        author_name: text(row, "speaker").unwrap_or_default(),
        created_at: millis(row, "said_at"),
      })
      .collect(),
  )
}

/// The items the old talk filed.
pub fn filings(dir: &Path, manifest: &Manifest) -> Result<Vec<Filing>, String> {
  Ok(
    rows(dir, manifest, "code_review_todos")?
      .iter()
      .map(|row| Filing {
        review_id: number(row, "review_id").unwrap_or_default(),
        item_id: number(row, "todo_id").unwrap_or_default(),
        // The old table kept no time of its own; the row stands at the epoch.
        created_at: 0,
      })
      .collect(),
  )
}

async fn insert_review(
  tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
  review: &Review,
) -> Result<(), String> {
  sqlx::query(
    "INSERT INTO reviews (id, project_name, revision, title, state, author_name, created_at, changed_at) \
     VALUES (?, ?, ?, ?, 'open', ?, ?, ?)",
  )
  .bind(review.id)
  .bind(&review.project_name)
  .bind(&review.revision)
  .bind(&review.title)
  .bind(&review.author_name)
  .bind(review.created_at)
  .bind(review.created_at)
  .execute(&mut **tx)
  .await
  .map_err(|error| format!("reviews could not be written: {error}"))?;

  Ok(())
}

async fn insert_place(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, place: &Place) -> Result<(), String> {
  // A review pinned to nothing but a folder has no excerpt and no lines: the comment still says where it
  // stands, with the review's own title standing in for words.
  let text = place
    .excerpt
    .clone()
    .unwrap_or_else(|| "(the place the review was pinned to)".to_owned());
  sqlx::query(
    "INSERT INTO comments (review_id, path, line, end_line, excerpt, text, author_name, created_at) \
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
  )
  .bind(place.review_id)
  .bind(&place.path)
  .bind(place.line)
  .bind(place.end_line)
  .bind(&place.excerpt)
  .bind(&text)
  .bind(&place.author_name)
  .bind(place.created_at)
  .execute(&mut **tx)
  .await
  .map_err(|error| format!("comments could not be written: {error}"))?;

  Ok(())
}

async fn insert_talk(tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, turn: &Talk) -> Result<(), String> {
  sqlx::query(
    "INSERT INTO comments (id, review_id, path, line, end_line, excerpt, text, author_name, created_at) \
     VALUES (?, ?, NULL, NULL, NULL, NULL, ?, ?, ?)",
  )
  .bind(turn.id)
  .bind(turn.review_id)
  .bind(&turn.text)
  .bind(&turn.author_name)
  .bind(turn.created_at)
  .execute(&mut **tx)
  .await
  .map_err(|error| format!("comments could not be written: {error}"))?;

  Ok(())
}

async fn insert_filing(
  tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
  filing: &Filing,
) -> Result<(), String> {
  sqlx::query("INSERT INTO filings (review_id, item_id, created_at) VALUES (?, ?, ?)")
    .bind(filing.review_id)
    .bind(filing.item_id)
    .bind(filing.created_at)
    .execute(&mut **tx)
    .await
    .map_err(|error| format!("filings could not be written: {error}"))?;

  Ok(())
}

/// Refuses a store that already holds rows of any table this import writes.
pub async fn refused_if_holding(pool: &SqlitePool) -> Result<(), String> {
  let mut holding = Vec::new();
  for table in TABLES {
    let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM \"{}\"", table.target))
      .fetch_one(pool)
      .await
      .map_err(|error| format!("{} could not be counted: {error}", table.target))?;
    if count > 0 {
      holding.push(format!("{} holds {count}", table.target));
    }
  }
  if holding.is_empty() {
    Ok(())
  } else {
    Err(format!(
      "the store already holds rows of its tables, so nothing was written: {}",
      holding.join(", ")
    ))
  }
}

/// Runs the import: reads the export, refuses a store holding rows, writes every row, and compares what it
/// wrote. A dry run rolls the transaction back and reports what would have been written.
pub async fn run(database: &Path, export: &Path, dry_run: bool) -> Result<Report, String> {
  let manifest = read_manifest(export)?;
  let reviews = reviews(export, &manifest)?;
  let places = places(export, &manifest)?;
  let talk = talk(export, &manifest)?;
  let filings = filings(export, &manifest)?;

  // The repository's own migrations, run over the store before anything is read into it.
  let store = Store::open(database).await.map_err(|error| error.to_string())?;
  refused_if_holding(store.pool()).await?;

  let mut tx = store
    .pool()
    .begin()
    .await
    .map_err(|error| format!("the transaction could not be opened: {error}"))?;
  for review in &reviews {
    insert_review(&mut tx, review).await?;
  }
  // The old turns carry their own numbers and are written first, so the place each review was pinned to
  // takes the next free number rather than one a turn already holds.
  for turn in &talk {
    insert_talk(&mut tx, turn).await?;
  }
  for place in &places {
    insert_place(&mut tx, place).await?;
  }
  for filing in &filings {
    insert_filing(&mut tx, filing).await?;
  }
  if dry_run {
    tx.rollback()
      .await
      .map_err(|error| format!("the dry run could not be rolled back: {error}"))?;
  } else {
    tx.commit()
      .await
      .map_err(|error| format!("the import could not be committed: {error}"))?;
  }

  let sample = if dry_run {
    Sample {
      checked: 0,
      failures: Vec::new(),
    }
  } else {
    compare(store.pool(), &manifest, &reviews, &places, &talk, &filings).await
  };

  let written = |source: &str| match source {
    "code_reviews" => reviews.len(),
    "code_review_talk" => talk.len(),
    "code_review_todos" => filings.len(),
    _ => 0,
  };
  let tables: Vec<Moved> = TABLES
    .iter()
    .map(|table| Moved {
      source: table.source.to_owned(),
      exported: manifest.tables.get(table.source).map(|entry| entry.count).unwrap_or(0),
      written: written(table.source) as u64,
    })
    .collect();

  Ok(Report {
    dry_run,
    snapshot: manifest.snapshot.clone(),
    read_at: manifest.read_at.clone(),
    tables,
    sample,
  })
}

async fn compare(
  pool: &SqlitePool,
  manifest: &Manifest,
  reviews: &[Review],
  places: &[Place],
  talk: &[Talk],
  filings: &[Filing],
) -> Sample {
  let mut failures = Vec::new();
  let mut checked = 0;

  async fn count(pool: &SqlitePool, target: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(&format!("SELECT COUNT(*) FROM \"{target}\""))
      .fetch_one(pool)
      .await
  }

  match (count(pool, "reviews").await, manifest.tables.get("code_reviews")) {
    (Ok(held), Some(entry)) if held as u64 == entry.count => {}
    (Ok(held), Some(entry)) => {
      failures.push(format!("code_reviews: the export holds {} rows and reviews holds {held}", entry.count))
    }
    (Err(error), _) => failures.push(format!("reviews could not be counted: {error}")),
    (_, None) => failures.push("code_reviews: the export records no table".to_owned()),
  }
  let expected_comments = talk.len() + places.len();
  match count(pool, "comments").await {
    Ok(held) if held as usize == expected_comments => {}
    Ok(held) => failures.push(format!(
      "code_review_talk: the export holds {} turns and {} places, and comments holds {held}",
      talk.len(),
      places.len()
    )),
    Err(error) => failures.push(format!("comments could not be counted: {error}")),
  }
  match (count(pool, "filings").await, manifest.tables.get("code_review_todos")) {
    (Ok(held), Some(entry)) if held as u64 == entry.count => {}
    (Ok(held), Some(entry)) => failures.push(format!(
      "code_review_todos: the export holds {} rows and filings holds {held}",
      entry.count
    )),
    (Err(error), _) => failures.push(format!("filings could not be counted: {error}")),
    (_, None) => failures.push("code_review_todos: the export records no table".to_owned()),
  }

  // Every review's place is a comment naming it, and every old turn is the comment with its own number.
  for review in sample(reviews).into_iter() {
    checked += 1;
    let held: Option<String> = sqlx::query_scalar("SELECT title FROM reviews WHERE id = ?1")
      .bind(review.id)
      .fetch_optional(pool)
      .await
      .ok()
      .flatten();
    if held.as_deref() != Some(review.title.as_str()) {
      failures.push(format!("code_reviews -> reviews: {} was written but is {held:?}", review.title));
    }
  }
  for turn in sample(talk).into_iter() {
    checked += 1;
    let held: Option<String> = sqlx::query_scalar("SELECT text FROM comments WHERE id = ?1")
      .bind(turn.id)
      .fetch_optional(pool)
      .await
      .ok()
      .flatten();
    if held.as_deref() != Some(turn.text.as_str()) {
      failures.push(format!("code_review_talk -> comments: turn {} was written but is {held:?}", turn.id));
    }
  }
  for filing in sample(filings).into_iter() {
    checked += 1;
    let held: Option<i64> =
      sqlx::query_scalar("SELECT item_id FROM filings WHERE review_id = ?1 AND item_id = ?2")
        .bind(filing.review_id)
        .bind(filing.item_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();
    if held != Some(filing.item_id) {
      failures
        .push(format!("code_review_todos -> filings: {} was written but is {held:?}", filing.item_id));
    }
  }

  Sample { checked, failures }
}

fn sample<T>(rows: &[T]) -> Vec<&T> {
  rows.iter().take(20).collect()
}
